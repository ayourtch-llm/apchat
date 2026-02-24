/// Reverso model implementation using candle tensors.
///
/// Implements the full Reverso architecture:
/// - Embedding layer
/// - Interleaved CNN + MLP and Attention + MLP blocks
/// - Cross-attention decoder head
///
/// Based on arXiv:2602.17634.
/// Original Python implementation by @oaustegard:
/// https://github.com/oaustegard/claude-skills/tree/main/forecasting-reverso
use candle_core::{Device, Result as CandleResult, Tensor};
use std::collections::HashMap;

use super::config::ReversoConfig;
use super::ops::*;

// ---------------------------------------------------------------------------
// Model blocks
// ---------------------------------------------------------------------------

/// Long depthwise FFT convolution with gating.
pub struct CNNBlock {
    kernel: Tensor,     // (d, L)
    gate_dw_w: Tensor,  // (d, ks) depthwise conv
    gate_dw_b: Tensor,  // (d,)
    gate_pw_w: Tensor,  // (d, d) pointwise conv (squeezed from (d, d, 1))
    gate_pw_b: Tensor,  // (d,)
    norm_w: Tensor,     // (d,)
    norm_b: Tensor,     // (d,)
}

impl CNNBlock {
    pub fn forward(&self, x: &Tensor, device: &Device) -> CandleResult<Tensor> {
        let residual = x.clone();

        // Gating: depthwise short conv → SiLU → pointwise conv → sigmoid
        let g = depthwise_short_conv(x, &self.gate_dw_w, Some(&self.gate_dw_b), device)?;
        let g = silu(&g)?;
        // Pointwise conv (kernel_size=1) = per-position linear
        let g = g.matmul(&self.gate_pw_w.t()?)?.broadcast_add(&self.gate_pw_b)?;
        let g = sigmoid(&g)?;
        let gated = x.mul(&g)?;

        // Long convolution via FFT
        let out = fft_long_conv(&gated, &self.kernel, device)?;
        let out = relu(&out)?;
        let out = layer_norm(&out, &self.norm_w, Some(&self.norm_b), 1e-5)?;
        out.add(&residual)
    }
}

/// Two-layer MLP with optional skip projection.
pub struct MLPBlock {
    linear_w: Tensor,   // (in, intermediate) — already transposed
    linear_b: Tensor,   // (intermediate,)
    final_w: Tensor,    // (intermediate, out) — already transposed
    final_b: Tensor,    // (out,)
    norm_w: Tensor,     // (out,)
    norm_b: Tensor,     // (out,)
    skip_w: Option<Tensor>, // (in, out) — already transposed
    skip_b: Option<Tensor>, // (out,)
}

impl MLPBlock {
    pub fn forward(&self, x: &Tensor) -> CandleResult<Tensor> {
        let residual = match (&self.skip_w, &self.skip_b) {
            (Some(w), Some(b)) => x.matmul(w)?.broadcast_add(b)?,
            _ => x.clone(),
        };
        let y = x.matmul(&self.linear_w)?.broadcast_add(&self.linear_b)?;
        let y = relu(&y)?;
        let y = y.matmul(&self.final_w)?.broadcast_add(&self.final_b)?;
        let y = layer_norm(&y, &self.norm_w, Some(&self.norm_b), 1e-5)?;
        y.add(&residual)
    }
}

/// DeltaNet linear attention with short convolutions.
pub struct AttentionBlock {
    q_proj_w: Tensor,   // (d, d) — already transposed
    k_proj_w: Tensor,
    v_proj_w: Tensor,
    o_proj_w: Tensor,
    beta_w: Tensor,     // (d, n_heads) — already transposed
    q_conv_w: Tensor,   // (d, conv_size)
    k_conv_w: Tensor,
    v_conv_w: Tensor,
    o_norm_w: Tensor,   // (d_head,) per-head RMSNorm
    norm_w: Tensor,     // (d,)
    norm_b: Tensor,     // (d,)
    n_heads: usize,
    state_weaving: bool,
}

impl AttentionBlock {
    pub fn forward(&self, x: &Tensor, device: &Device) -> CandleResult<Tensor> {
        let residual = x.clone();

        let x = if self.state_weaving {
            // Feed end-of-sequence info to start: x[0] += x[-1]
            let l = x.dim(0)?;
            let last_row = x.narrow(0, l - 1, 1)?;
            let first_row = x.narrow(0, 0, 1)?;
            let new_first = first_row.add(&last_row)?;
            if l > 1 {
                let rest = x.narrow(0, 1, l - 1)?;
                Tensor::cat(&[&new_first, &rest], 0)?
            } else {
                new_first
            }
        } else {
            x.clone()
        };

        let (l, d) = x.dims2()?;
        let d_h = d / self.n_heads;

        // Linear projections (no bias)
        let q = x.matmul(&self.q_proj_w)?;
        let k = x.matmul(&self.k_proj_w)?;
        let v = x.matmul(&self.v_proj_w)?;

        // Short convolutions (causal, depthwise, no bias)
        let q = depthwise_short_conv(&q, &self.q_conv_w, None, device)?;
        let k = depthwise_short_conv(&k, &self.k_conv_w, None, device)?;
        let v = depthwise_short_conv(&v, &self.v_conv_w, None, device)?;

        // Reshape to multi-head: (L, d) → (L, n_heads, d_h)
        let q = q.reshape((l, self.n_heads, d_h))?;
        let k = k.reshape((l, self.n_heads, d_h))?;
        let v = v.reshape((l, self.n_heads, d_h))?;

        // Activations + per-head L2 normalization
        let q = l2_normalize(&silu(&q)?, 1e-12)?;
        let k = l2_normalize(&silu(&k)?, 1e-12)?;

        // Beta gate (no bias)
        let beta = sigmoid(&x.matmul(&self.beta_w)?)?; // (L, n_heads)

        // DeltaNet recurrence
        let out = deltanet_recurrence(&q, &k, &v, &beta, device)?; // (L, n_heads, d_h)

        // Per-head RMS normalization
        let mut head_outputs = Vec::new();
        for h in 0..self.n_heads {
            let head = out.narrow(1, h, 1)?.squeeze(1)?; // (L, d_h)
            let normed = rms_norm(&head, &self.o_norm_w, 1e-6)?;
            head_outputs.push(normed);
        }
        let out = Tensor::stack(&head_outputs, 1)?; // (L, n_heads, d_h)

        // Reshape back and output projection
        let out = out.reshape((l, d))?;
        let out = out.matmul(&self.o_proj_w)?;

        let out = layer_norm(&out, &self.norm_w, Some(&self.norm_b), 1e-5)?;
        out.add(&residual)
    }
}

/// Attention-based decoder producing output_token_len predictions.
pub struct DecoderHead {
    head_w: Tensor,      // (p, seq_len)
    head_b: Tensor,      // (p,)
    q_proj_w: Tensor,    // (d, d) — transposed
    q_proj_b: Tensor,
    k_proj_w: Tensor,    // (d, d) — transposed
    k_proj_b: Tensor,
    v_proj_w: Tensor,    // (d, d) — transposed
    v_proj_b: Tensor,
    out_proj_w: Tensor,  // (d, 1) — transposed
    out_proj_b: Tensor,  // (1,)
}

impl DecoderHead {
    pub fn forward(&self, x: &Tensor, _device: &Device) -> CandleResult<Tensor> {
        let (l, d) = x.dims2()?;

        // Position mixing: head_w[:, :L] @ x + head_b[:, None]
        let head_w_slice = self.head_w.narrow(1, 0, l)?; // (p, L)
        let z = head_w_slice.matmul(&x)?; // (p, d)
        let z = z.broadcast_add(&self.head_b.unsqueeze(1)?)?; // (p, d) + (p, 1)

        // Cross-attention
        let q = z.matmul(&self.q_proj_w)?.broadcast_add(&self.q_proj_b)?; // (p, d)
        let k = x.matmul(&self.k_proj_w)?.broadcast_add(&self.k_proj_b)?; // (L, d)
        let v = x.matmul(&self.v_proj_w)?.broadcast_add(&self.v_proj_b)?; // (L, d)

        let scale = 1.0 / (d as f64).sqrt();
        let attn_weights = q.matmul(&k.t()?)?.affine(scale, 0.0)?; // (p, L)
        let attn_weights = softmax_last_dim(&attn_weights)?;
        let attn_out = attn_weights.matmul(&v)?; // (p, d)

        // Output projection → (p, 1) → squeeze to (p,)
        let out = attn_out
            .matmul(&self.out_proj_w)?
            .broadcast_add(&self.out_proj_b)?;
        out.squeeze(1)
    }
}

// ---------------------------------------------------------------------------
// Full model
// ---------------------------------------------------------------------------

enum LayerBlock {
    CNN(CNNBlock),
    MLP(MLPBlock),
    Attention(AttentionBlock),
}

pub struct ReversoModel {
    _config: ReversoConfig,
    embedding_w: Tensor, // (d_model, 1)
    layers: Vec<LayerBlock>,
    decoder: DecoderHead,
    device: Device,
}

impl ReversoModel {
    /// Run a single forward pass: normalized input (L,) → (output_token_len,) predictions.
    pub fn forward(&self, x_norm: &[f32]) -> CandleResult<Vec<f32>> {
        let l = x_norm.len();
        let x = Tensor::from_vec(x_norm.to_vec(), (l, 1), &self.device)?;

        // Embedding: (L, 1) @ (1, d_model) = (L, d_model)
        let h = x.matmul(&self.embedding_w.t()?)?;

        let mut h = h;
        for layer in &self.layers {
            h = match layer {
                LayerBlock::CNN(block) => block.forward(&h, &self.device)?,
                LayerBlock::MLP(block) => block.forward(&h)?,
                LayerBlock::Attention(block) => block.forward(&h, &self.device)?,
            };
        }

        let preds = self.decoder.forward(&h, &self.device)?;
        preds.to_vec1()
    }

    /// Forward with flip equivariance for [0,1]-normalized input.
    pub fn forward_flip_equivariant(&self, x_norm: &[f32]) -> CandleResult<Vec<f32>> {
        let f_pos = self.forward(x_norm)?;
        let flipped: Vec<f32> = x_norm.iter().map(|&v| 1.0 - v).collect();
        let f_flip = self.forward(&flipped)?;
        Ok(f_pos
            .iter()
            .zip(f_flip.iter())
            .map(|(&p, &f)| (p + 1.0 - f) / 2.0)
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Weight loading helpers
// ---------------------------------------------------------------------------

fn get_weight(weights: &HashMap<String, Tensor>, key: &str) -> CandleResult<Tensor> {
    weights.get(key).cloned().ok_or_else(|| {
        candle_core::Error::Msg(format!("Weight '{}' not found in checkpoint", key))
    })
}

/// Transpose a weight tensor from (out, in) to (in, out) for x @ W computation.
fn transpose_weight(w: &Tensor) -> CandleResult<Tensor> {
    w.t()?.contiguous()
}

/// Squeeze depthwise conv weight from (d, 1, ks) to (d, ks).
fn squeeze_conv(w: &Tensor) -> CandleResult<Tensor> {
    if w.rank() == 3 && w.dim(1)? == 1 {
        w.squeeze(1)
    } else {
        Ok(w.clone())
    }
}

fn build_cnn_block(
    weights: &HashMap<String, Tensor>,
    idx: usize,
) -> CandleResult<CNNBlock> {
    let pfx = format!("layers.{}", idx);
    let gate_pw_w = get_weight(weights, &format!("{}.pregate.net.2.weight", pfx))?;
    let gate_pw_w = if gate_pw_w.rank() == 3 {
        gate_pw_w.squeeze(2)?
    } else {
        gate_pw_w
    };

    Ok(CNNBlock {
        kernel: get_weight(weights, &format!("{}.k", pfx))?,
        gate_dw_w: squeeze_conv(&get_weight(
            weights,
            &format!("{}.pregate.net.0.weight", pfx),
        )?)?,
        gate_dw_b: get_weight(weights, &format!("{}.pregate.net.0.bias", pfx))?,
        gate_pw_w,
        gate_pw_b: get_weight(weights, &format!("{}.pregate.net.2.bias", pfx))?,
        norm_w: get_weight(weights, &format!("{}.norm.weight", pfx))?,
        norm_b: get_weight(weights, &format!("{}.norm.bias", pfx))?,
    })
}

fn build_mlp_block(
    weights: &HashMap<String, Tensor>,
    idx: usize,
) -> CandleResult<MLPBlock> {
    let pfx = format!("layers.{}", idx);
    let skip_w_key = format!("{}.skip_linear.weight", pfx);
    let skip_b_key = format!("{}.skip_linear.bias", pfx);

    let skip_w = weights
        .get(&skip_w_key)
        .map(|w| transpose_weight(w))
        .transpose()?;
    let skip_b = weights.get(&skip_b_key).cloned();

    Ok(MLPBlock {
        linear_w: transpose_weight(&get_weight(
            weights,
            &format!("{}.linear.weight", pfx),
        )?)?,
        linear_b: get_weight(weights, &format!("{}.linear.bias", pfx))?,
        final_w: transpose_weight(&get_weight(
            weights,
            &format!("{}.linear_final.weight", pfx),
        )?)?,
        final_b: get_weight(weights, &format!("{}.linear_final.bias", pfx))?,
        norm_w: get_weight(weights, &format!("{}.norm.weight", pfx))?,
        norm_b: get_weight(weights, &format!("{}.norm.bias", pfx))?,
        skip_w,
        skip_b,
    })
}

fn build_attn_block(
    weights: &HashMap<String, Tensor>,
    idx: usize,
    config: &ReversoConfig,
    state_weaving: bool,
) -> CandleResult<AttentionBlock> {
    let pfx = format!("layers.{}", idx);
    let ap = format!("{}.attention", pfx);

    Ok(AttentionBlock {
        q_proj_w: transpose_weight(&get_weight(weights, &format!("{}.q_proj.weight", ap))?)?,
        k_proj_w: transpose_weight(&get_weight(weights, &format!("{}.k_proj.weight", ap))?)?,
        v_proj_w: transpose_weight(&get_weight(weights, &format!("{}.v_proj.weight", ap))?)?,
        o_proj_w: transpose_weight(&get_weight(weights, &format!("{}.o_proj.weight", ap))?)?,
        beta_w: transpose_weight(&get_weight(weights, &format!("{}.b_proj.weight", ap))?)?,
        q_conv_w: squeeze_conv(&get_weight(weights, &format!("{}.q_conv1d.weight", ap))?)?,
        k_conv_w: squeeze_conv(&get_weight(weights, &format!("{}.k_conv1d.weight", ap))?)?,
        v_conv_w: squeeze_conv(&get_weight(weights, &format!("{}.v_conv1d.weight", ap))?)?,
        o_norm_w: get_weight(weights, &format!("{}.o_norm.weight", ap))?,
        norm_w: get_weight(weights, &format!("{}.norm.weight", pfx))?,
        norm_b: get_weight(weights, &format!("{}.norm.bias", pfx))?,
        n_heads: config.n_heads,
        state_weaving,
    })
}

fn build_decoder(
    weights: &HashMap<String, Tensor>,
) -> CandleResult<DecoderHead> {
    Ok(DecoderHead {
        head_w: get_weight(weights, "head.weight")?,
        head_b: get_weight(weights, "head.bias")?,
        q_proj_w: transpose_weight(&get_weight(weights, "simple_q_proj.weight")?)?,
        q_proj_b: get_weight(weights, "simple_q_proj.bias")?,
        k_proj_w: transpose_weight(&get_weight(weights, "key_proj.weight")?)?,
        k_proj_b: get_weight(weights, "key_proj.bias")?,
        v_proj_w: transpose_weight(&get_weight(weights, "value_proj.weight")?)?,
        v_proj_b: get_weight(weights, "value_proj.bias")?,
        out_proj_w: transpose_weight(&get_weight(weights, "out_proj.weight")?)?,
        out_proj_b: get_weight(weights, "out_proj.bias")?,
    })
}

/// Load weights from a safetensors file into a HashMap.
pub fn load_safetensors(
    path: &str,
    device: &Device,
) -> CandleResult<HashMap<String, Tensor>> {
    candle_core::safetensors::load(path, device)
}

/// Construct a ReversoModel from a weight dictionary and config.
pub fn load_model(
    weights: &HashMap<String, Tensor>,
    config: &ReversoConfig,
    device: Device,
) -> CandleResult<ReversoModel> {
    let embedding_w = get_weight(weights, "embedding.weight")?;

    let mut layers = Vec::new();
    let mut layer_idx = 0;
    let mut n_attn = 0;
    let total_attn = config.module_list.iter().filter(|m| *m == "attn").count();

    for mod_type in &config.module_list {
        match mod_type.as_str() {
            "conv" => {
                layers.push(LayerBlock::CNN(build_cnn_block(weights, layer_idx)?));
                layer_idx += 1;
                layers.push(LayerBlock::MLP(build_mlp_block(weights, layer_idx)?));
                layer_idx += 1;
            }
            "attn" => {
                let is_intermediate = n_attn < (total_attn - 1);
                layers.push(LayerBlock::Attention(build_attn_block(
                    weights,
                    layer_idx,
                    config,
                    is_intermediate,
                )?));
                layer_idx += 1;
                n_attn += 1;
                layers.push(LayerBlock::MLP(build_mlp_block(weights, layer_idx)?));
                layer_idx += 1;
            }
            other => {
                return Err(candle_core::Error::Msg(format!(
                    "Unknown module type: {}",
                    other
                )));
            }
        }
    }

    let decoder = build_decoder(weights)?;

    Ok(ReversoModel {
        _config: config.clone(),
        embedding_w,
        layers,
        decoder,
        device,
    })
}

/// Zero-shot time series forecast using Reverso.
///
/// Takes raw historical observations, handles preprocessing, runs autoregressive
/// rollout, and returns denormalized predictions.
pub fn forecast(
    series: &[f32],
    prediction_length: usize,
    weights: &HashMap<String, Tensor>,
    config: &ReversoConfig,
    flip_equivariant: bool,
    device: Device,
) -> Result<Vec<f32>, String> {
    let model =
        load_model(weights, config, device).map_err(|e| format!("Model loading failed: {}", e))?;

    let (x_norm, x_min, x_max) =
        preprocess(series, config.seq_len)?;

    let forward_fn = if flip_equivariant {
        ReversoModel::forward_flip_equivariant
    } else {
        ReversoModel::forward
    };

    let mut context = x_norm;
    let mut predictions = Vec::new();
    let mut remaining = prediction_length;

    while remaining > 0 {
        let ctx_start = if context.len() > config.seq_len {
            context.len() - config.seq_len
        } else {
            0
        };
        let ctx = &context[ctx_start..];

        let chunk = forward_fn(&model, ctx).map_err(|e| format!("Forward pass failed: {}", e))?;

        let take = config.output_token_len.min(remaining);
        predictions.extend_from_slice(&chunk[..take]);
        context.extend_from_slice(&chunk);
        remaining -= take;
    }

    let result = postprocess(&predictions[..prediction_length], x_min, x_max);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transpose_weight() {
        let device = Device::Cpu;
        let w = Tensor::new(&[[1.0f32, 2.0], [3.0, 4.0], [5.0, 6.0]], &device).unwrap();
        // (3, 2) → (2, 3)
        let wt = transpose_weight(&w).unwrap();
        assert_eq!(wt.dims(), &[2, 3]);
    }

    #[test]
    fn test_squeeze_conv() {
        let device = Device::Cpu;
        let w = Tensor::new(&[[[1.0f32, 2.0]], [[3.0, 4.0]]], &device).unwrap();
        assert_eq!(w.dims(), &[2, 1, 2]);
        let squeezed = squeeze_conv(&w).unwrap();
        assert_eq!(squeezed.dims(), &[2, 2]);
    }
}
