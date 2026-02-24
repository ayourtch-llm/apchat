/// Model configuration for a Reverso variant.
///
/// Based on the published `args.json` shipped alongside each checkpoint.
/// See arXiv:2602.17634 for architecture details.
#[derive(Debug, Clone)]
pub struct ReversoConfig {
    pub d_model: usize,
    pub module_list: Vec<String>,
    pub seq_len: usize,
    pub output_token_len: usize,
    pub d_intermediate: usize,
    pub n_heads: usize,
    pub gating_kernel_size: usize,
    pub attn_conv_size: usize,
}

impl ReversoConfig {
    pub fn d_head(&self) -> usize {
        self.d_model / self.n_heads
    }

    pub fn nano() -> Self {
        Self {
            d_model: 32,
            module_list: vec!["conv", "attn"]
                .into_iter()
                .map(String::from)
                .collect(),
            seq_len: 2048,
            output_token_len: 48,
            d_intermediate: 256,
            n_heads: 4,
            gating_kernel_size: 3,
            attn_conv_size: 4,
        }
    }

    pub fn small() -> Self {
        Self {
            d_model: 64,
            module_list: vec!["conv", "attn", "conv", "attn"]
                .into_iter()
                .map(String::from)
                .collect(),
            seq_len: 2048,
            output_token_len: 48,
            d_intermediate: 256,
            n_heads: 4,
            gating_kernel_size: 3,
            attn_conv_size: 4,
        }
    }

    pub fn full() -> Self {
        let mut module_list = Vec::new();
        for _ in 0..4 {
            module_list.push("conv".to_string());
            module_list.push("attn".to_string());
        }
        Self {
            d_model: 128,
            module_list,
            seq_len: 2048,
            output_token_len: 48,
            d_intermediate: 256,
            n_heads: 4,
            gating_kernel_size: 3,
            attn_conv_size: 4,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "nano" => Some(Self::nano()),
            "small" => Some(Self::small()),
            "full" => Some(Self::full()),
            _ => None,
        }
    }
}
