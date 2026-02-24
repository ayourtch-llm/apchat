/// Math operations for the Reverso model.
///
/// Implements neural network primitives using candle tensors, plus
/// FFT-based circular convolution via rustfft and the DeltaNet recurrence.
use candle_core::{Device, Result as CandleResult, Tensor};
use rustfft::{num_complex::Complex32, FftPlanner};

// ---------------------------------------------------------------------------
// Activation / normalization helpers
// ---------------------------------------------------------------------------

pub fn sigmoid(x: &Tensor) -> CandleResult<Tensor> {
    // sigmoid(x) = 1 / (1 + exp(-x))
    (x.neg()?.exp()? + 1.0)?.recip()
}

pub fn silu(x: &Tensor) -> CandleResult<Tensor> {
    x.mul(&sigmoid(x)?)
}

pub fn relu(x: &Tensor) -> CandleResult<Tensor> {
    x.maximum(&x.zeros_like()?)
}

pub fn softmax_last_dim(x: &Tensor) -> CandleResult<Tensor> {
    let dim = x.rank() - 1;
    let max_val = x.max(dim)?.unsqueeze(dim)?;
    let e = x.broadcast_sub(&max_val)?.exp()?;
    let sum_e = e.sum(dim)?.unsqueeze(dim)?;
    e.broadcast_div(&sum_e)
}

pub fn layer_norm(
    x: &Tensor,
    weight: &Tensor,
    bias: Option<&Tensor>,
    eps: f64,
) -> CandleResult<Tensor> {
    let dim = x.rank() - 1;
    let mean = x.mean(dim)?.unsqueeze(dim)?;
    let centered = x.broadcast_sub(&mean)?;
    let var = centered.sqr()?.mean(dim)?.unsqueeze(dim)?;
    let normed = centered.broadcast_div(&(var + eps)?.sqrt()?)?;
    let out = normed.broadcast_mul(weight)?;
    match bias {
        Some(b) => out.broadcast_add(b),
        None => Ok(out),
    }
}

pub fn rms_norm(x: &Tensor, weight: &Tensor, eps: f64) -> CandleResult<Tensor> {
    let dim = x.rank() - 1;
    let rms = x.sqr()?.mean(dim)?.unsqueeze(dim)?;
    let normed = x.broadcast_div(&(rms + eps)?.sqrt()?)?;
    normed.broadcast_mul(weight)
}

pub fn l2_normalize(x: &Tensor, eps: f64) -> CandleResult<Tensor> {
    let dim = x.rank() - 1;
    let norm = x.sqr()?.sum(dim)?.unsqueeze(dim)?;
    x.broadcast_div(&(norm + eps)?.sqrt()?)
}

// ---------------------------------------------------------------------------
// Depthwise short (causal) 1-D convolution
// ---------------------------------------------------------------------------

/// Causal depthwise 1-D convolution (correlation, matching PyTorch Conv1d).
///
/// x: (L, d), weight: (d, kernel_size), bias: optional (d,)
/// Returns: (L, d)
pub fn depthwise_short_conv(
    x: &Tensor,
    weight: &Tensor,
    bias: Option<&Tensor>,
    device: &Device,
) -> CandleResult<Tensor> {
    let (l, d) = x.dims2()?;
    let ks = weight.dim(1)?;

    let x_data: Vec<Vec<f32>> = x.to_vec2()?;
    let w_data: Vec<Vec<f32>> = weight.to_vec2()?;

    let mut out = vec![0.0f32; l * d];

    for c in 0..d {
        for i in 0..l {
            let mut acc = 0.0f32;
            for k in 0..ks {
                let src_idx = i as isize - k as isize;
                let val = if src_idx >= 0 {
                    x_data[src_idx as usize][c]
                } else {
                    0.0
                };
                acc += val * w_data[c][k];
            }
            out[i * d + c] = acc;
        }
    }

    let result = Tensor::from_vec(out, (l, d), device)?;
    match bias {
        Some(b) => result.broadcast_add(b),
        None => Ok(result),
    }
}

// ---------------------------------------------------------------------------
// FFT-based circular convolution
// ---------------------------------------------------------------------------

/// Depthwise long circular convolution via FFT.
///
/// Matches FlashFFTConv behaviour used during Reverso training:
/// circular convolution with n_fft = L (no zero-padding).
///
/// x: (L, d), kernel: (d, L) → result: (L, d)
pub fn fft_long_conv(x: &Tensor, kernel: &Tensor, device: &Device) -> CandleResult<Tensor> {
    let (l, d) = x.dims2()?;

    let x_data: Vec<Vec<f32>> = x.to_vec2()?;
    let k_data: Vec<Vec<f32>> = kernel.to_vec2()?;

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(l);
    let ifft = planner.plan_fft_inverse(l);

    let mut result = vec![0.0f32; l * d];

    for c in 0..d {
        let mut x_c: Vec<Complex32> = (0..l)
            .map(|i| Complex32::new(x_data[i][c], 0.0))
            .collect();
        let mut k_c: Vec<Complex32> = k_data[c]
            .iter()
            .map(|&v| Complex32::new(v, 0.0))
            .collect();

        fft.process(&mut x_c);
        fft.process(&mut k_c);

        let mut conv_c: Vec<Complex32> = x_c
            .iter()
            .zip(k_c.iter())
            .map(|(a, b)| a * b)
            .collect();

        ifft.process(&mut conv_c);

        let scale = 1.0 / l as f32;
        for i in 0..l {
            result[i * d + c] = conv_c[i].re * scale;
        }
    }

    Tensor::from_vec(result, (l, d), device)
}

// ---------------------------------------------------------------------------
// DeltaNet linear-attention recurrence
// ---------------------------------------------------------------------------

/// DeltaNet linear-attention recurrence (all heads).
///
/// Runs in f64 for numerical stability, matching the original implementation.
///
/// q, k, v: (L, n_heads, d_h), beta: (L, n_heads)
/// Returns: (L, n_heads, d_h)
pub fn deltanet_recurrence(
    q: &Tensor,
    k: &Tensor,
    v: &Tensor,
    beta: &Tensor,
    device: &Device,
) -> CandleResult<Tensor> {
    let dims = q.dims3()?;
    let (l, n_heads, d_h) = (dims.0, dims.1, dims.2);

    // Extract as f64 for numerical stability
    let q_data: Vec<Vec<Vec<f32>>> = q.to_vec3()?;
    let k_data: Vec<Vec<Vec<f32>>> = k.to_vec3()?;
    let v_data: Vec<Vec<Vec<f32>>> = v.to_vec3()?;
    let beta_data: Vec<Vec<f32>> = beta.to_vec2()?;

    let mut out = vec![0.0f32; l * n_heads * d_h];

    for h in 0..n_heads {
        let mut s = vec![0.0f64; d_h * d_h];

        for i in 0..l {
            let qi = &q_data[i][h];
            let ki = &k_data[i][h];
            let vi = &v_data[i][h];
            let bi = beta_data[i][h] as f64;

            // Sk = S @ ki
            let mut sk = vec![0.0f64; d_h];
            for a in 0..d_h {
                let mut acc = 0.0f64;
                for b in 0..d_h {
                    acc += s[a * d_h + b] * ki[b] as f64;
                }
                sk[a] = acc;
            }

            // S += bi * outer(vi - Sk, ki)
            for a in 0..d_h {
                let diff = bi * (vi[a] as f64 - sk[a]);
                for b in 0..d_h {
                    s[a * d_h + b] += diff * ki[b] as f64;
                }
            }

            // out[i, h] = S @ qi
            for a in 0..d_h {
                let mut acc = 0.0f64;
                for b in 0..d_h {
                    acc += s[a * d_h + b] * qi[b] as f64;
                }
                out[(i * n_heads + h) * d_h + a] = acc as f32;
            }
        }
    }

    Tensor::from_vec(out, (l, n_heads, d_h), device)
}

// ---------------------------------------------------------------------------
// Preprocessing / postprocessing
// ---------------------------------------------------------------------------

/// Prepare raw series for model input.
///
/// Handles NaN interpolation, padding, truncation, and min-max normalization.
/// Returns (normalized_series, x_min, x_max).
pub fn preprocess(series: &[f32], seq_len: usize) -> Result<(Vec<f32>, f32, f32), String> {
    if series.is_empty() {
        return Err("Series is empty".to_string());
    }

    let mut x: Vec<f32> = series.to_vec();

    // Interpolate NaNs
    let has_nan = x.iter().any(|v| v.is_nan());
    if has_nan {
        let valid: Vec<(usize, f32)> = x
            .iter()
            .enumerate()
            .filter(|(_, v)| !v.is_nan())
            .map(|(i, &v)| (i, v))
            .collect();
        if valid.is_empty() {
            return Err("Series is entirely NaN".to_string());
        }
        for i in 0..x.len() {
            if x[i].is_nan() {
                // Linear interpolation
                let before = valid.iter().rev().find(|(idx, _)| *idx < i);
                let after = valid.iter().find(|(idx, _)| *idx > i);
                x[i] = match (before, after) {
                    (Some(&(bi, bv)), Some(&(ai, av))) => {
                        let t = (i - bi) as f32 / (ai - bi) as f32;
                        bv + t * (av - bv)
                    }
                    (Some(&(_, bv)), None) => bv,
                    (None, Some(&(_, av))) => av,
                    (None, None) => unreachable!(),
                };
            }
        }
    }

    // Pad short series by back-filling with leftmost value
    if x.len() < seq_len {
        let pad_len = seq_len - x.len();
        let first_val = x[0];
        let mut padded = vec![first_val; pad_len];
        padded.extend_from_slice(&x);
        x = padded;
    }

    // Truncate to last seq_len values
    if x.len() > seq_len {
        x = x[x.len() - seq_len..].to_vec();
    }

    // Min-max normalization to [0, 1]
    let x_min = x.iter().cloned().fold(f32::INFINITY, f32::min);
    let x_max = x.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let denom = x_max - x_min;

    let x_norm = if denom < 1e-10 {
        vec![0.5f32; x.len()]
    } else {
        x.iter().map(|&v| (v - x_min) / denom).collect()
    };

    Ok((x_norm, x_min, x_max))
}

/// Unnormalize predictions back to original scale.
pub fn postprocess(predictions: &[f32], x_min: f32, x_max: f32) -> Vec<f32> {
    predictions
        .iter()
        .map(|&v| v * (x_max - x_min) + x_min)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocess_basic() {
        let series = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let (norm, min, max) = preprocess(&series, 5).unwrap();
        assert_eq!(min, 1.0);
        assert_eq!(max, 5.0);
        assert!((norm[0] - 0.0).abs() < 1e-6);
        assert!((norm[4] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_preprocess_padding() {
        let series = vec![10.0, 20.0, 30.0];
        let (norm, _, _) = preprocess(&series, 5).unwrap();
        assert_eq!(norm.len(), 5);
        // First two should be padded with the first value (10.0)
        // After normalization: (10-10)/(30-10) = 0.0
        assert!((norm[0] - 0.0).abs() < 1e-6);
        assert!((norm[1] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_preprocess_truncation() {
        let series: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let (norm, _, _) = preprocess(&series, 10).unwrap();
        assert_eq!(norm.len(), 10);
    }

    #[test]
    fn test_preprocess_constant_series() {
        let series = vec![5.0; 10];
        let (norm, _, _) = preprocess(&series, 10).unwrap();
        for v in &norm {
            assert!((v - 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn test_preprocess_nan_interpolation() {
        let series = vec![1.0, f32::NAN, 3.0];
        let (norm, min, max) = preprocess(&series, 3).unwrap();
        assert_eq!(min, 1.0);
        assert_eq!(max, 3.0);
        assert!((norm[1] - 0.5).abs() < 1e-6); // interpolated to 2.0 → (2-1)/(3-1) = 0.5
    }

    #[test]
    fn test_preprocess_empty() {
        assert!(preprocess(&[], 10).is_err());
    }

    #[test]
    fn test_preprocess_all_nan() {
        let series = vec![f32::NAN, f32::NAN];
        assert!(preprocess(&series, 2).is_err());
    }

    #[test]
    fn test_postprocess() {
        let preds = vec![0.0, 0.5, 1.0];
        let result = postprocess(&preds, 10.0, 20.0);
        assert!((result[0] - 10.0).abs() < 1e-6);
        assert!((result[1] - 15.0).abs() < 1e-6);
        assert!((result[2] - 20.0).abs() < 1e-6);
    }

    #[test]
    fn test_sigmoid_values() {
        let device = Device::Cpu;
        let x = Tensor::new(&[0.0f32, 1.0, -1.0, 10.0, -10.0], &device).unwrap();
        let s = sigmoid(&x).unwrap();
        let vals: Vec<f32> = s.to_vec1().unwrap();
        assert!((vals[0] - 0.5).abs() < 1e-5);
        assert!((vals[1] - 0.7310586).abs() < 1e-4);
        assert!((vals[2] - 0.2689414).abs() < 1e-4);
        assert!(vals[3] > 0.999);
        assert!(vals[4] < 0.001);
    }

    #[test]
    fn test_silu_at_zero() {
        let device = Device::Cpu;
        let x = Tensor::new(&[0.0f32], &device).unwrap();
        let s = silu(&x).unwrap();
        let vals: Vec<f32> = s.to_vec1().unwrap();
        assert!((vals[0] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_l2_normalize() {
        let device = Device::Cpu;
        let x = Tensor::new(&[[3.0f32, 4.0]], &device).unwrap();
        let normed = l2_normalize(&x, 1e-12).unwrap();
        let vals: Vec<Vec<f32>> = normed.to_vec2().unwrap();
        assert!((vals[0][0] - 0.6).abs() < 1e-5);
        assert!((vals[0][1] - 0.8).abs() < 1e-5);
    }

    #[test]
    fn test_softmax() {
        let device = Device::Cpu;
        let x = Tensor::new(&[[1.0f32, 2.0, 3.0]], &device).unwrap();
        let s = softmax_last_dim(&x).unwrap();
        let vals: Vec<Vec<f32>> = s.to_vec2().unwrap();
        let sum: f32 = vals[0].iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
        assert!(vals[0][2] > vals[0][1]);
        assert!(vals[0][1] > vals[0][0]);
    }

    #[test]
    fn test_depthwise_short_conv_identity() {
        let device = Device::Cpu;
        // kernel_size=1, weight=1.0 should be identity
        let x = Tensor::new(&[[1.0f32, 2.0], [3.0, 4.0]], &device).unwrap();
        let w = Tensor::new(&[[1.0f32], [1.0]], &device).unwrap();
        let result = depthwise_short_conv(&x, &w, None, &device).unwrap();
        let vals: Vec<Vec<f32>> = result.to_vec2().unwrap();
        assert!((vals[0][0] - 1.0).abs() < 1e-5);
        assert!((vals[0][1] - 2.0).abs() < 1e-5);
        assert!((vals[1][0] - 3.0).abs() < 1e-5);
        assert!((vals[1][1] - 4.0).abs() < 1e-5);
    }
}
