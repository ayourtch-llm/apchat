//! Local inference for GLM-OCR using candle.
//!
//! Implements local OCR inference using the candle ML framework.
//! Loads the GLM-OCR model (CogViT visual encoder + MLP connector + GLM-0.5B
//! decoder) from safetensors weights and runs inference on CPU or GPU.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use candle_core::{DType, Device, IndexOp, Tensor, D};
use candle_nn::{self as nn, Module, VarBuilder};
use tokenizers::Tokenizer;

// ---------------------------------------------------------------------------
// Model configuration (deserialized from config.json)
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize, Debug, Clone)]
pub struct ModelConfig {
    #[serde(default)]
    pub vision_config: VisionConfig,
    #[serde(default)]
    pub text_config: TextConfig,
    #[serde(default)]
    pub connector_config: ConnectorConfig,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct VisionConfig {
    #[serde(default = "default_vision_hidden")]
    pub hidden_size: usize,
    #[serde(default = "default_image_size")]
    pub image_size: usize,
    #[serde(default = "default_vision_intermediate")]
    pub intermediate_size: usize,
    #[serde(default = "default_vision_heads")]
    pub num_attention_heads: usize,
    #[serde(default = "default_vision_layers")]
    pub num_hidden_layers: usize,
    #[serde(default = "default_patch_size")]
    pub patch_size: usize,
    #[serde(default = "default_num_channels")]
    pub num_channels: usize,
    #[serde(default = "default_ln_eps")]
    pub layer_norm_eps: f64,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct TextConfig {
    #[serde(default = "default_text_hidden")]
    pub hidden_size: usize,
    #[serde(default = "default_text_intermediate")]
    pub intermediate_size: usize,
    #[serde(default = "default_max_pos")]
    pub max_position_embeddings: usize,
    #[serde(default = "default_text_heads")]
    pub num_attention_heads: usize,
    #[serde(default = "default_text_layers")]
    pub num_hidden_layers: usize,
    #[serde(default)]
    pub num_key_value_heads: Option<usize>,
    #[serde(default = "default_vocab_size")]
    pub vocab_size: usize,
    #[serde(default = "default_rms_norm_eps")]
    pub rms_norm_eps: f64,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct ConnectorConfig {
    #[serde(default = "default_vision_hidden")]
    pub vision_hidden_size: usize,
    #[serde(default = "default_text_hidden")]
    pub text_hidden_size: usize,
    #[serde(default = "default_downsample")]
    pub downsample_ratio: usize,
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            hidden_size: default_vision_hidden(),
            image_size: default_image_size(),
            intermediate_size: default_vision_intermediate(),
            num_attention_heads: default_vision_heads(),
            num_hidden_layers: default_vision_layers(),
            patch_size: default_patch_size(),
            num_channels: default_num_channels(),
            layer_norm_eps: default_ln_eps(),
        }
    }
}

impl Default for TextConfig {
    fn default() -> Self {
        Self {
            hidden_size: default_text_hidden(),
            intermediate_size: default_text_intermediate(),
            max_position_embeddings: default_max_pos(),
            num_attention_heads: default_text_heads(),
            num_hidden_layers: default_text_layers(),
            num_key_value_heads: None,
            vocab_size: default_vocab_size(),
            rms_norm_eps: default_rms_norm_eps(),
        }
    }
}

impl Default for ConnectorConfig {
    fn default() -> Self {
        Self {
            vision_hidden_size: default_vision_hidden(),
            text_hidden_size: default_text_hidden(),
            downsample_ratio: default_downsample(),
        }
    }
}

fn default_vision_hidden() -> usize { 1024 }
fn default_image_size() -> usize { 1120 }
fn default_vision_intermediate() -> usize { 4096 }
fn default_vision_heads() -> usize { 16 }
fn default_vision_layers() -> usize { 24 }
fn default_patch_size() -> usize { 14 }
fn default_num_channels() -> usize { 3 }
fn default_ln_eps() -> f64 { 1e-6 }
fn default_text_hidden() -> usize { 1024 }
fn default_text_intermediate() -> usize { 2816 }
fn default_max_pos() -> usize { 16384 }
fn default_text_heads() -> usize { 16 }
fn default_text_layers() -> usize { 24 }
fn default_vocab_size() -> usize { 151552 }
fn default_rms_norm_eps() -> f64 { 1e-5 }
fn default_downsample() -> usize { 4 }

// ---------------------------------------------------------------------------
// RMS Normalization
// ---------------------------------------------------------------------------

struct RmsNorm {
    weight: Tensor,
    eps: f64,
}

impl RmsNorm {
    fn load(dim: usize, eps: f64, vb: VarBuilder) -> candle_core::Result<Self> {
        let weight = vb.get(dim, "weight")?;
        Ok(Self { weight, eps })
    }

    fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        let dtype = x.dtype();
        let x = x.to_dtype(DType::F32)?;
        let variance = (&x * &x)?.mean_keepdim(D::Minus1)?;
        let x_normed = x.broadcast_div(&(variance + self.eps)?.sqrt()?)?;
        let w = self.weight.to_dtype(DType::F32)?;
        x_normed.broadcast_mul(&w)?.to_dtype(dtype)
    }
}

// ---------------------------------------------------------------------------
// Rotary Position Embedding (RoPE)
// ---------------------------------------------------------------------------

struct RotaryEmbedding {
    cos: Tensor,
    sin: Tensor,
}

impl RotaryEmbedding {
    fn new(head_dim: usize, max_len: usize, device: &Device) -> candle_core::Result<Self> {
        let theta: Vec<f32> = (0..head_dim / 2)
            .map(|i| 1.0 / (10000f32.powf(2.0 * i as f32 / head_dim as f32)))
            .collect();
        let theta = Tensor::new(theta.as_slice(), device)?;
        let positions: Vec<f32> = (0..max_len).map(|p| p as f32).collect();
        let positions = Tensor::new(positions.as_slice(), device)?.unsqueeze(1)?;
        let angles = positions.matmul(&theta.unsqueeze(0)?)?;
        let cos = angles.cos()?;
        let sin = angles.sin()?;
        Ok(Self { cos, sin })
    }

    fn apply(&self, q: &Tensor, k: &Tensor, offset: usize) -> candle_core::Result<(Tensor, Tensor)> {
        let seq_len = q.dim(D::Minus2)?;
        let cos = self.cos.i(offset..offset + seq_len)?.unsqueeze(0)?.unsqueeze(0)?;
        let sin = self.sin.i(offset..offset + seq_len)?.unsqueeze(0)?.unsqueeze(0)?;
        let q_rot = apply_rotary(q, &cos, &sin)?;
        let k_rot = apply_rotary(k, &cos, &sin)?;
        Ok((q_rot, k_rot))
    }
}

fn apply_rotary(x: &Tensor, cos: &Tensor, sin: &Tensor) -> candle_core::Result<Tensor> {
    let dim = x.dim(D::Minus1)?;
    let half = dim / 2;
    let x1 = x.narrow(D::Minus1, 0, half)?;
    let x2 = x.narrow(D::Minus1, half, half)?;
    let rotated = Tensor::cat(&[
        &(x1.broadcast_mul(cos)? - x2.broadcast_mul(sin)?)?,
        &(x2.broadcast_mul(cos)? + x1.broadcast_mul(sin)?)?,
    ], D::Minus1)?;
    Ok(rotated)
}

// ---------------------------------------------------------------------------
// Vision Encoder (CogViT / ViT)
// ---------------------------------------------------------------------------

struct VisionAttention {
    q_proj: nn::Linear,
    k_proj: nn::Linear,
    v_proj: nn::Linear,
    out_proj: nn::Linear,
    num_heads: usize,
    head_dim: usize,
}

impl VisionAttention {
    fn load(cfg: &VisionConfig, vb: VarBuilder) -> candle_core::Result<Self> {
        let dim = cfg.hidden_size;
        let num_heads = cfg.num_attention_heads;
        let head_dim = dim / num_heads;
        Ok(Self {
            q_proj: nn::linear(dim, dim, vb.pp("q_proj"))?,
            k_proj: nn::linear(dim, dim, vb.pp("k_proj"))?,
            v_proj: nn::linear(dim, dim, vb.pp("v_proj"))?,
            out_proj: nn::linear(dim, dim, vb.pp("out_proj"))?,
            num_heads,
            head_dim,
        })
    }

    fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        let (b, n, _) = x.dims3()?;
        let q = self.q_proj.forward(x)?;
        let k = self.k_proj.forward(x)?;
        let v = self.v_proj.forward(x)?;

        let q = q.reshape((b, n, self.num_heads, self.head_dim))?.transpose(1, 2)?;
        let k = k.reshape((b, n, self.num_heads, self.head_dim))?.transpose(1, 2)?;
        let v = v.reshape((b, n, self.num_heads, self.head_dim))?.transpose(1, 2)?;

        let scale = (self.head_dim as f64).sqrt();
        let attn = (q.matmul(&k.transpose(D::Minus2, D::Minus1)?)? / scale)?;
        let attn = nn::ops::softmax_last_dim(&attn)?;
        let out = attn.matmul(&v)?;
        let out = out.transpose(1, 2)?.reshape((b, n, self.num_heads * self.head_dim))?;
        self.out_proj.forward(&out)
    }
}

struct VisionMlp {
    fc1: nn::Linear,
    fc2: nn::Linear,
}

impl VisionMlp {
    fn load(cfg: &VisionConfig, vb: VarBuilder) -> candle_core::Result<Self> {
        let dim = cfg.hidden_size;
        let inter = cfg.intermediate_size;
        Ok(Self {
            fc1: nn::linear(dim, inter, vb.pp("fc1"))?,
            fc2: nn::linear(inter, dim, vb.pp("fc2"))?,
        })
    }

    fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        self.fc2.forward(&self.fc1.forward(x)?.gelu()?)
    }
}

struct VisionBlock {
    attn: VisionAttention,
    mlp: VisionMlp,
    norm1: nn::LayerNorm,
    norm2: nn::LayerNorm,
}

impl VisionBlock {
    fn load(cfg: &VisionConfig, vb: VarBuilder) -> candle_core::Result<Self> {
        let dim = cfg.hidden_size;
        let ln_cfg = nn::LayerNormConfig {
            eps: cfg.layer_norm_eps,
            ..Default::default()
        };
        Ok(Self {
            attn: VisionAttention::load(cfg, vb.pp("attn"))?,
            mlp: VisionMlp::load(cfg, vb.pp("mlp"))?,
            norm1: nn::layer_norm(dim, ln_cfg, vb.pp("norm1"))?,
            norm2: nn::layer_norm(dim, ln_cfg, vb.pp("norm2"))?,
        })
    }

    fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        let residual = x;
        let x = self.attn.forward(&self.norm1.forward(x)?)?;
        let x = (residual + x)?;
        let residual = &x;
        let x = self.mlp.forward(&self.norm2.forward(&x)?)?;
        residual + x
    }
}

struct VisionEncoder {
    patch_embed: nn::Conv2d,
    position_embedding: Tensor,
    blocks: Vec<VisionBlock>,
    norm: nn::LayerNorm,
}

impl VisionEncoder {
    fn load(cfg: &VisionConfig, vb: VarBuilder) -> candle_core::Result<Self> {
        let conv_cfg = nn::Conv2dConfig {
            stride: cfg.patch_size,
            ..Default::default()
        };
        let patch_embed = nn::conv2d(
            cfg.num_channels,
            cfg.hidden_size,
            cfg.patch_size,
            conv_cfg,
            vb.pp("patch_embed.proj"),
        )?;

        let num_patches = (cfg.image_size / cfg.patch_size).pow(2);
        let position_embedding = vb.get(
            (1, num_patches + 1, cfg.hidden_size),
            "position_embedding",
        )?;

        let mut blocks = Vec::with_capacity(cfg.num_hidden_layers);
        for i in 0..cfg.num_hidden_layers {
            blocks.push(VisionBlock::load(cfg, vb.pp(format!("encoder.layers.{i}")))?);
        }

        let ln_cfg = nn::LayerNormConfig {
            eps: cfg.layer_norm_eps,
            ..Default::default()
        };
        let norm = nn::layer_norm(cfg.hidden_size, ln_cfg, vb.pp("norm"))?;

        Ok(Self { patch_embed, position_embedding, blocks, norm })
    }

    fn forward(&self, pixel_values: &Tensor) -> candle_core::Result<Tensor> {
        let x = self.patch_embed.forward(pixel_values)?;
        let (b, c, h, w) = x.dims4()?;
        let x = x.reshape((b, c, h * w))?.transpose(1, 2)?;

        let num_tokens = x.dim(1)?;
        let pos_embed = self.position_embedding.i((.., ..num_tokens, ..))?;
        let x = (x + pos_embed)?;

        let mut x = x;
        for block in &self.blocks {
            x = block.forward(&x)?;
        }
        self.norm.forward(&x)
    }
}

// ---------------------------------------------------------------------------
// Cross-Modal Connector (MLP with downsampling)
// ---------------------------------------------------------------------------

struct CrossModalConnector {
    linear1: nn::Linear,
    linear2: nn::Linear,
    norm: nn::LayerNorm,
    downsample_ratio: usize,
}

impl CrossModalConnector {
    fn load(cfg: &ConnectorConfig, vb: VarBuilder) -> candle_core::Result<Self> {
        let ln_cfg = nn::LayerNormConfig::default();
        Ok(Self {
            linear1: nn::linear(
                cfg.vision_hidden_size * cfg.downsample_ratio,
                cfg.text_hidden_size,
                vb.pp("linear1"),
            )?,
            linear2: nn::linear(cfg.text_hidden_size, cfg.text_hidden_size, vb.pp("linear2"))?,
            norm: nn::layer_norm(cfg.text_hidden_size, ln_cfg, vb.pp("norm"))?,
            downsample_ratio: cfg.downsample_ratio,
        })
    }

    fn forward(&self, vision_features: &Tensor) -> candle_core::Result<Tensor> {
        let (b, n, d) = vision_features.dims3()?;
        let ratio = self.downsample_ratio;
        let new_n = n / ratio;
        let x = vision_features.reshape((b, new_n, d * ratio))?;
        let x = self.linear1.forward(&x)?.gelu()?;
        let x = self.linear2.forward(&x)?;
        self.norm.forward(&x)
    }
}

// ---------------------------------------------------------------------------
// Causal mask helper
// ---------------------------------------------------------------------------

fn build_causal_mask(query_len: usize, seq_len: usize, device: &Device) -> candle_core::Result<Tensor> {
    let offset = seq_len - query_len;
    let mask: Vec<u8> = (0..query_len)
        .flat_map(|i| {
            (0..seq_len).map(move |j| {
                if j <= i + offset { 1u8 } else { 0u8 }
            })
        })
        .collect();
    Tensor::from_vec(mask, (query_len, seq_len), device)
}

// ---------------------------------------------------------------------------
// GLM Decoder
// ---------------------------------------------------------------------------

struct GlmAttention {
    q_proj: nn::Linear,
    k_proj: nn::Linear,
    v_proj: nn::Linear,
    o_proj: nn::Linear,
    num_heads: usize,
    num_kv_heads: usize,
    head_dim: usize,
}

impl GlmAttention {
    fn load(cfg: &TextConfig, vb: VarBuilder) -> candle_core::Result<Self> {
        let dim = cfg.hidden_size;
        let num_heads = cfg.num_attention_heads;
        let num_kv_heads = cfg.num_key_value_heads.unwrap_or(num_heads);
        let head_dim = dim / num_heads;
        Ok(Self {
            q_proj: nn::linear_no_bias(dim, num_heads * head_dim, vb.pp("q_proj"))?,
            k_proj: nn::linear_no_bias(dim, num_kv_heads * head_dim, vb.pp("k_proj"))?,
            v_proj: nn::linear_no_bias(dim, num_kv_heads * head_dim, vb.pp("v_proj"))?,
            o_proj: nn::linear_no_bias(num_heads * head_dim, dim, vb.pp("o_proj"))?,
            num_heads,
            num_kv_heads,
            head_dim,
        })
    }

    fn forward(
        &self,
        x: &Tensor,
        rope: &RotaryEmbedding,
        kv_cache: Option<(&Tensor, &Tensor)>,
        offset: usize,
    ) -> candle_core::Result<(Tensor, Tensor, Tensor)> {
        let (b, n, _) = x.dims3()?;
        let q = self.q_proj.forward(x)?;
        let k = self.k_proj.forward(x)?;
        let v = self.v_proj.forward(x)?;

        let q = q.reshape((b, n, self.num_heads, self.head_dim))?.transpose(1, 2)?;
        let k = k.reshape((b, n, self.num_kv_heads, self.head_dim))?.transpose(1, 2)?;
        let v = v.reshape((b, n, self.num_kv_heads, self.head_dim))?.transpose(1, 2)?;

        let (q, k) = rope.apply(&q, &k, offset)?;

        let (k, v) = if let Some((prev_k, prev_v)) = kv_cache {
            let k = Tensor::cat(&[prev_k, &k], 2)?;
            let v = Tensor::cat(&[prev_v, &v], 2)?;
            (k, v)
        } else {
            (k, v)
        };

        // GQA: repeat KV heads if num_kv_heads < num_heads
        let (k, v) = if self.num_kv_heads < self.num_heads {
            let repeats = self.num_heads / self.num_kv_heads;
            let k = k.repeat((1, repeats, 1, 1))?;
            let v = v.repeat((1, repeats, 1, 1))?;
            (k, v)
        } else {
            (k, v)
        };

        let scale = (self.head_dim as f64).sqrt();
        let attn = (q.matmul(&k.transpose(D::Minus2, D::Minus1)?)? / scale)?;

        // Causal mask
        let seq_len = attn.dim(D::Minus1)?;
        let query_len = attn.dim(D::Minus2)?;
        if query_len > 1 {
            let mask_bool = build_causal_mask(query_len, seq_len, attn.device())?;
            let mask_bool = mask_bool.unsqueeze(0)?.unsqueeze(0)?;
            let neg_inf = Tensor::new(f32::NEG_INFINITY, attn.device())?
                .broadcast_as(attn.shape())?;
            let attn = mask_bool.where_cond(&attn.to_dtype(DType::F32)?, &neg_inf)?;
            let attn = nn::ops::softmax_last_dim(&attn)?.to_dtype(v.dtype())?;
            let out = attn.matmul(&v)?;
            let out = out.transpose(1, 2)?.reshape((b, query_len, self.num_heads * self.head_dim))?;
            let out = self.o_proj.forward(&out)?;
            let (kc, vc) = self.extract_kv_cache(k, v)?;
            return Ok((out, kc, vc));
        }

        let attn = nn::ops::softmax_last_dim(&attn)?;
        let out = attn.matmul(&v)?;
        let out = out.transpose(1, 2)?.reshape((b, n, self.num_heads * self.head_dim))?;
        let out = self.o_proj.forward(&out)?;
        let (kc, vc) = self.extract_kv_cache(k, v)?;
        Ok((out, kc, vc))
    }

    fn extract_kv_cache(&self, k: Tensor, v: Tensor) -> candle_core::Result<(Tensor, Tensor)> {
        if self.num_kv_heads < self.num_heads {
            Ok((k.narrow(1, 0, self.num_kv_heads)?, v.narrow(1, 0, self.num_kv_heads)?))
        } else {
            Ok((k, v))
        }
    }
}

struct GlmFfn {
    gate_proj: nn::Linear,
    up_proj: nn::Linear,
    down_proj: nn::Linear,
}

impl GlmFfn {
    fn load(cfg: &TextConfig, vb: VarBuilder) -> candle_core::Result<Self> {
        let dim = cfg.hidden_size;
        let inter = cfg.intermediate_size;
        Ok(Self {
            gate_proj: nn::linear_no_bias(dim, inter, vb.pp("gate_proj"))?,
            up_proj: nn::linear_no_bias(dim, inter, vb.pp("up_proj"))?,
            down_proj: nn::linear_no_bias(inter, dim, vb.pp("down_proj"))?,
        })
    }

    fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        let gate = self.gate_proj.forward(x)?.silu()?;
        let up = self.up_proj.forward(x)?;
        self.down_proj.forward(&(gate * up)?)
    }
}

struct GlmDecoderLayer {
    self_attn: GlmAttention,
    ffn: GlmFfn,
    input_norm: RmsNorm,
    post_attn_norm: RmsNorm,
}

impl GlmDecoderLayer {
    fn load(cfg: &TextConfig, vb: VarBuilder) -> candle_core::Result<Self> {
        Ok(Self {
            self_attn: GlmAttention::load(cfg, vb.pp("self_attn"))?,
            ffn: GlmFfn::load(cfg, vb.pp("mlp"))?,
            input_norm: RmsNorm::load(cfg.hidden_size, cfg.rms_norm_eps, vb.pp("input_layernorm"))?,
            post_attn_norm: RmsNorm::load(cfg.hidden_size, cfg.rms_norm_eps, vb.pp("post_attention_layernorm"))?,
        })
    }

    fn forward(
        &self,
        x: &Tensor,
        rope: &RotaryEmbedding,
        kv_cache: Option<(&Tensor, &Tensor)>,
        offset: usize,
    ) -> candle_core::Result<(Tensor, Tensor, Tensor)> {
        let residual = x;
        let x = self.input_norm.forward(x)?;
        let (x, k_cache, v_cache) = self.self_attn.forward(&x, rope, kv_cache, offset)?;
        let x = (residual + x)?;
        let residual = &x;
        let x = self.ffn.forward(&self.post_attn_norm.forward(&x)?)?;
        Ok(((residual + x)?, k_cache, v_cache))
    }
}

struct GlmDecoder {
    embed_tokens: nn::Embedding,
    layers: Vec<GlmDecoderLayer>,
    final_norm: RmsNorm,
    lm_head: nn::Linear,
    rope: RotaryEmbedding,
}

impl GlmDecoder {
    fn load(cfg: &TextConfig, vb: VarBuilder, device: &Device) -> candle_core::Result<Self> {
        let head_dim = cfg.hidden_size / cfg.num_attention_heads;
        let rope = RotaryEmbedding::new(head_dim, cfg.max_position_embeddings, device)?;

        let mut layers = Vec::with_capacity(cfg.num_hidden_layers);
        for i in 0..cfg.num_hidden_layers {
            layers.push(GlmDecoderLayer::load(cfg, vb.pp(format!("layers.{i}")))?);
        }

        Ok(Self {
            embed_tokens: nn::embedding(cfg.vocab_size, cfg.hidden_size, vb.pp("embed_tokens"))?,
            layers,
            final_norm: RmsNorm::load(cfg.hidden_size, cfg.rms_norm_eps, vb.pp("norm"))?,
            lm_head: nn::linear_no_bias(cfg.hidden_size, cfg.vocab_size, vb.pp("lm_head"))?,
            rope,
        })
    }

    fn forward(
        &self,
        input_ids: &Tensor,
        vision_embeds: Option<&Tensor>,
        kv_caches: &mut [Option<(Tensor, Tensor)>],
        offset: usize,
    ) -> candle_core::Result<Tensor> {
        let mut x = self.embed_tokens.forward(input_ids)?;

        // Vision embeddings are prepended to the token embeddings only on the
        // initial forward pass (offset == 0). In subsequent autoregressive steps
        // the KV cache already contains the vision context, so we only feed new
        // text tokens.
        if let Some(vis) = vision_embeds {
            if offset == 0 {
                x = Tensor::cat(&[vis, &x], 1)?;
            }
        }

        for (i, layer) in self.layers.iter().enumerate() {
            let cache = kv_caches[i].as_ref().map(|(k, v)| (k, v));
            let (new_x, k, v) = layer.forward(&x, &self.rope, cache, offset)?;
            x = new_x;
            kv_caches[i] = Some((k, v));
        }

        let x = self.final_norm.forward(&x)?;
        // Only compute logits for the last token
        let last = x.i((.., x.dim(1)? - 1..x.dim(1)?, ..))?;
        self.lm_head.forward(&last)?.squeeze(1)
    }
}

// ---------------------------------------------------------------------------
// Full Model
// ---------------------------------------------------------------------------

pub struct GlmOcrModel {
    vision_encoder: VisionEncoder,
    connector: CrossModalConnector,
    decoder: GlmDecoder,
    config: ModelConfig,
}

impl GlmOcrModel {
    pub fn load(cfg: ModelConfig, vb: VarBuilder, device: &Device) -> candle_core::Result<Self> {
        let vision_encoder = VisionEncoder::load(&cfg.vision_config, vb.pp("model.vision"))?;
        let connector = CrossModalConnector::load(&cfg.connector_config, vb.pp("model.connector"))?;
        let decoder = GlmDecoder::load(&cfg.text_config, vb.pp("model.language_model"), device)?;
        Ok(Self { vision_encoder, connector, decoder, config: cfg })
    }

    pub fn encode_image(&self, pixel_values: &Tensor) -> candle_core::Result<Tensor> {
        let vision_out = self.vision_encoder.forward(pixel_values)?;
        self.connector.forward(&vision_out)
    }

    pub fn forward(
        &self,
        input_ids: &Tensor,
        vision_embeds: Option<&Tensor>,
        kv_caches: &mut [Option<(Tensor, Tensor)>],
        offset: usize,
    ) -> candle_core::Result<Tensor> {
        self.decoder.forward(input_ids, vision_embeds, kv_caches, offset)
    }

    pub fn num_layers(&self) -> usize {
        self.config.text_config.num_hidden_layers
    }
}

// ---------------------------------------------------------------------------
// Image preprocessing
// ---------------------------------------------------------------------------

pub fn preprocess_image(
    path: &Path,
    cfg: &VisionConfig,
    device: &Device,
) -> Result<Tensor> {
    let img = image::open(path)
        .with_context(|| format!("Failed to open image: {}", path.display()))?;
    let img = img.to_rgb8();

    let target_size = cfg.image_size as u32;
    let img = image::imageops::resize(
        &img,
        target_size,
        target_size,
        image::imageops::FilterType::Lanczos3,
    );

    let (w, h) = (img.width() as usize, img.height() as usize);
    let pixels: Vec<f32> = img
        .pixels()
        .flat_map(|p| {
            let [r, g, b] = p.0;
            // ImageNet normalization
            let r = (r as f32 / 255.0 - 0.485) / 0.229;
            let g = (g as f32 / 255.0 - 0.456) / 0.224;
            let b = (b as f32 / 255.0 - 0.406) / 0.225;
            [r, g, b]
        })
        .collect();

    // Convert to [1, 3, H, W] tensor
    let tensor = Tensor::from_vec(pixels, (h, w, 3), device)?;
    let tensor = tensor.permute((2, 0, 1))?.unsqueeze(0)?;
    // Use F32 on CPU for precision, BF16 on GPU for performance
    let dtype = if device.is_cpu() { DType::F32 } else { DType::BF16 };
    tensor.to_dtype(dtype).map_err(Into::into)
}

// ---------------------------------------------------------------------------
// Text generation
// ---------------------------------------------------------------------------

pub fn greedy_generate(
    model: &GlmOcrModel,
    tokenizer: &Tokenizer,
    vision_embeds: &Tensor,
    prompt: &str,
    max_tokens: usize,
    device: &Device,
) -> Result<String> {
    let encoding = tokenizer.encode(prompt, true)
        .map_err(|e| anyhow::anyhow!("Tokenizer error: {}", e))?;
    let input_ids: Vec<u32> = encoding.get_ids().to_vec();
    let input_tensor = Tensor::new(input_ids.as_slice(), device)?.unsqueeze(0)?;

    let eos_token_id = tokenizer
        .token_to_id("</s>")
        .or_else(|| tokenizer.token_to_id("<|endoftext|>"))
        .unwrap_or(2);

    let mut kv_caches: Vec<Option<(Tensor, Tensor)>> = vec![None; model.num_layers()];
    let mut generated_ids: Vec<u32> = Vec::new();

    // First pass: process prompt + vision embeddings
    let logits = model.forward(&input_tensor, Some(vision_embeds), &mut kv_caches, 0)?;
    let next_token = logits.argmax(D::Minus1)?.to_vec1::<u32>()?[0];
    generated_ids.push(next_token);

    if next_token == eos_token_id {
        return decode_tokens(tokenizer, &generated_ids);
    }

    // Determine offset: vision tokens + prompt tokens
    let vis_len = vision_embeds.dim(1)?;
    let mut offset = vis_len + input_ids.len();

    // Subsequent passes: one token at a time
    for _ in 1..max_tokens {
        let token_tensor = Tensor::new(&[*generated_ids.last().unwrap()], device)?.unsqueeze(0)?;
        let logits = model.forward(&token_tensor, None, &mut kv_caches, offset)?;
        let next_token = logits.argmax(D::Minus1)?.to_vec1::<u32>()?[0];
        generated_ids.push(next_token);
        offset += 1;

        if next_token == eos_token_id {
            break;
        }
    }

    decode_tokens(tokenizer, &generated_ids)
}

fn decode_tokens(tokenizer: &Tokenizer, ids: &[u32]) -> Result<String> {
    tokenizer
        .decode(ids, true)
        .map_err(|e| anyhow::anyhow!("Tokenizer decode error: {}", e))
}

// ---------------------------------------------------------------------------
// Model loading helpers
// ---------------------------------------------------------------------------

pub fn resolve_device(device_str: &str) -> Result<Device> {
    match device_str {
        "cpu" => Ok(Device::Cpu),
        #[cfg(feature = "cuda")]
        "cuda" | "gpu" => Ok(Device::new_cuda(0)?),
        #[cfg(feature = "metal")]
        "metal" | "gpu" => Ok(Device::new_metal(0)?),
        other => anyhow::bail!(
            "Unknown device '{}'. Use 'cpu'{}",
            other,
            if cfg!(feature = "cuda") { ", 'cuda', or 'gpu'" }
            else if cfg!(feature = "metal") { ", 'metal', or 'gpu'" }
            else { "" }
        ),
    }
}

pub fn load_model_files(
    model_id: &str,
    model_path: Option<&Path>,
) -> Result<PathBuf> {
    if let Some(path) = model_path {
        if path.exists() {
            return Ok(path.to_path_buf());
        }
        anyhow::bail!("Model path does not exist: {}", path.display());
    }

    // Download from HuggingFace Hub
    eprintln!("Downloading model '{}'...", model_id);
    let api = hf_hub::api::sync::Api::new()
        .context("Failed to create HuggingFace Hub API client")?;
    let repo = api.model(model_id.to_string());

    // Download essential files
    for filename in &["config.json", "tokenizer.json"] {
        repo.get(filename)
            .with_context(|| format!("Failed to download {}", filename))?;
    }

    // Download safetensors weight files
    // Try to get the model index first
    if let Ok(index_path) = repo.get("model.safetensors.index.json") {
        let index_str = std::fs::read_to_string(&index_path)
            .context("Failed to read model index")?;
        let index: serde_json::Value = serde_json::from_str(&index_str)
            .context("Failed to parse model index")?;
        if let Some(weight_map) = index.get("weight_map").and_then(|m| m.as_object()) {
            let filenames: std::collections::HashSet<String> = weight_map
                .values()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            for filename in filenames {
                repo.get(&filename)
                    .with_context(|| format!("Failed to download {}", filename))?;
            }
        }
    } else {
        // Single safetensors file
        repo.get("model.safetensors")
            .context("Failed to download model.safetensors")?;
    }

    // Return the cache directory path
    let config_path = repo.get("config.json")
        .context("Failed to locate config.json")?;
    let model_dir = config_path.parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine model directory"))?;
    Ok(model_dir.to_path_buf())
}

pub fn load_config(model_dir: &Path) -> Result<ModelConfig> {
    let config_path = model_dir.join("config.json");
    let config_str = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;
    serde_json::from_str(&config_str)
        .with_context(|| format!("Failed to parse {}", config_path.display()))
}

pub fn load_tokenizer(model_dir: &Path) -> Result<Tokenizer> {
    let tokenizer_path = model_dir.join("tokenizer.json");
    Tokenizer::from_file(&tokenizer_path)
        .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))
}

fn find_safetensors(model_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(model_dir)
        .with_context(|| format!("Failed to read directory: {}", model_dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|p| p.extension().map(|e| e == "safetensors").unwrap_or(false))
        .collect();

    if files.is_empty() {
        anyhow::bail!("No .safetensors files found in {}", model_dir.display());
    }
    files.sort();
    Ok(files)
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub async fn run_local_parse(
    images: Vec<String>,
    model_id: String,
    model_path: Option<PathBuf>,
    device_str: String,
    max_tokens: usize,
    format: String,
    output: Option<PathBuf>,
) -> Result<()> {
    let device = resolve_device(&device_str)?;

    // Load model files
    let model_dir = load_model_files(&model_id, model_path.as_deref())?;
    eprintln!("Model directory: {}", model_dir.display());

    // Load configuration and tokenizer
    let config = load_config(&model_dir)?;
    let tokenizer = load_tokenizer(&model_dir)?;

    // Load model weights (use F32 on CPU, BF16 on GPU)
    eprintln!("Loading model weights...");
    let safetensors_files = find_safetensors(&model_dir)?;
    let safetensors_refs: Vec<&PathBuf> = safetensors_files.iter().collect();
    let dtype = if device.is_cpu() { DType::F32 } else { DType::BF16 };
    let vb = unsafe {
        VarBuilder::from_mmaped_safetensors(&safetensors_refs, dtype, &device)?
    };
    let model = GlmOcrModel::load(config.clone(), vb, &device)?;
    eprintln!("Model loaded successfully.");

    // Create output directory if needed
    if let Some(ref dir) = output {
        tokio::fs::create_dir_all(dir)
            .await
            .with_context(|| format!("Failed to create output directory: {}", dir.display()))?;
    }

    let prompt = "OCR this image. Return the full text content.";

    for (i, image_source) in images.iter().enumerate() {
        eprintln!("Processing: {}", image_source);
        let path = PathBuf::from(image_source);
        let pixel_values = preprocess_image(&path, &config.vision_config, &device)?;
        let vision_embeds = model.encode_image(&pixel_values)?;
        let raw = greedy_generate(&model, &tokenizer, &vision_embeds, prompt, max_tokens, &device)?;
        let text = super::format_result(&raw, &format);

        if let Some(ref dir) = output {
            let ext = match format.as_str() {
                "json" => "json",
                _ => "md",
            };
            let stem = PathBuf::from(image_source)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("result_{}", i));
            let out_path = dir.join(format!("{}.{}", stem, ext));
            tokio::fs::write(&out_path, &text)
                .await
                .with_context(|| format!("Failed to write output: {}", out_path.display()))?;
            eprintln!("Wrote: {}", out_path.display());
        } else {
            if images.len() > 1 {
                println!("--- {} ---", image_source);
            }
            println!("{}", text);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_config_defaults() {
        let json = r#"{ "vision_config": {}, "text_config": {}, "connector_config": {} }"#;
        let config: ModelConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.vision_config.hidden_size, 1024);
        assert_eq!(config.vision_config.image_size, 1120);
        assert_eq!(config.vision_config.patch_size, 14);
        assert_eq!(config.text_config.hidden_size, 1024);
        assert_eq!(config.text_config.vocab_size, 151552);
        assert_eq!(config.connector_config.downsample_ratio, 4);
    }

    #[test]
    fn test_model_config_custom() {
        let json = r#"{
            "vision_config": { "hidden_size": 768, "image_size": 224, "patch_size": 16, "num_attention_heads": 12, "num_hidden_layers": 12, "intermediate_size": 3072 },
            "text_config": { "hidden_size": 2048, "intermediate_size": 5632, "num_attention_heads": 32, "num_hidden_layers": 32, "vocab_size": 32000 },
            "connector_config": { "vision_hidden_size": 768, "text_hidden_size": 2048, "downsample_ratio": 2 }
        }"#;
        let config: ModelConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.vision_config.hidden_size, 768);
        assert_eq!(config.vision_config.image_size, 224);
        assert_eq!(config.text_config.hidden_size, 2048);
        assert_eq!(config.text_config.num_hidden_layers, 32);
        assert_eq!(config.connector_config.downsample_ratio, 2);
    }

    #[test]
    fn test_model_config_missing_sections() {
        let json = r#"{}"#;
        let config: ModelConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.vision_config.hidden_size, 1024);
        assert_eq!(config.text_config.vocab_size, 151552);
    }

    #[test]
    fn test_resolve_device_cpu() {
        let device = resolve_device("cpu").unwrap();
        assert!(matches!(device, Device::Cpu));
    }

    #[test]
    fn test_resolve_device_unknown() {
        let result = resolve_device("tpu");
        assert!(result.is_err());
    }

    #[test]
    fn test_find_safetensors_empty() {
        let dir = tempfile::tempdir().unwrap();
        let result = find_safetensors(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_find_safetensors_found() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("model.safetensors"), b"fake").unwrap();
        std::fs::write(dir.path().join("model-00001-of-00002.safetensors"), b"fake").unwrap();
        let files = find_safetensors(dir.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_load_config_valid() {
        let dir = tempfile::tempdir().unwrap();
        let config_json = r#"{
            "vision_config": { "hidden_size": 512 },
            "text_config": { "vocab_size": 32000 }
        }"#;
        std::fs::write(dir.path().join("config.json"), config_json).unwrap();
        let config = load_config(dir.path()).unwrap();
        assert_eq!(config.vision_config.hidden_size, 512);
        assert_eq!(config.text_config.vocab_size, 32000);
    }

    #[test]
    fn test_load_config_missing() {
        let dir = tempfile::tempdir().unwrap();
        let result = load_config(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_rms_norm() {
        let device = Device::Cpu;
        let weight = Tensor::ones(4, DType::F32, &device).unwrap();
        let norm = RmsNorm { weight, eps: 1e-5 };
        let x = Tensor::new(&[[1.0f32, 2.0, 3.0, 4.0]], &device).unwrap();
        let result = norm.forward(&x).unwrap();
        let values: Vec<f32> = result.flatten_all().unwrap().to_vec1().unwrap();
        // RMS norm of [1,2,3,4]: rms = sqrt((1+4+9+16)/4) = sqrt(7.5) ≈ 2.7386
        // normalized ≈ [0.3651, 0.7303, 1.0954, 1.4606]
        assert!(values[0] > 0.35 && values[0] < 0.38);
        assert!(values[3] > 1.45 && values[3] < 1.47);
    }

    #[test]
    fn test_rotary_embedding_creation() {
        let device = Device::Cpu;
        let rope = RotaryEmbedding::new(64, 2048, &device).unwrap();
        assert_eq!(rope.cos.dims(), &[2048, 32]);
        assert_eq!(rope.sin.dims(), &[2048, 32]);
    }
}
