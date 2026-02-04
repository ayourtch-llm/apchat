/// Candle backend implementation for embeddings (optional feature)
/// 
/// This backend provides an alternative to FastEmbed for embedding generation
/// using the Candle ML library. It is disabled by default.
/// 
/// To enable, add the `candle` feature to your Cargo.toml:
/// ```toml
/// apchat-skills = { path = "...", features = ["candle"] }
/// ```
/// 
/// Note: Enabling this feature requires the `embeddings` feature to be enabled as well.
use anyhow::Result;
use super::EmbeddingBackend;

/// Candle-based embedding backend
/// 
/// This backend is currently disabled. To use embeddings, enable the `embeddings` feature
/// which uses FastEmbed as the default backend.
pub struct CandleBackend {
    dimension: usize,
}

impl CandleBackend {
    /// Create a new Candle backend
    /// 
    /// # Error
    /// Returns an error indicating that Candle backend requires the `embeddings` feature.
    /// Suggests using FastEmbed as an alternative.
    pub fn new() -> Result<Self> {
        // This is a stub implementation that provides clear guidance
        // The actual Candle backend would need:
        // 1. candle and candle-transformers dependencies in Cargo.toml
        // 2. Sentence-transformers model loading
        // 3. Tokenizer integration
        // 4. Model inference implementation
        anyhow::bail!(
            "Candle backend is not enabled.\n\
            \n\
            To use embeddings, enable the 'embeddings' feature which provides FastEmbed backend:\n\
            `apchat-skills = {{ path = \"...\", features = [\"embeddings\"] }}`\n\
            \n\
            For Candle backend support, enable both features:\n\
            `apchat-skills = {{ path = \"...\", features = [\"embeddings\", \"candle\"] }}`\n\
            \n\
            Note: Candle backend requires additional dependencies and implementation."
        );
    }
}

impl Default for CandleBackend {
    fn default() -> Self {
        // Return a default instance for type compatibility
        // This will still fail on embed() but allows compilation
        Self { dimension: 384 } // Common embedding dimension
    }
}

impl EmbeddingBackend for CandleBackend {
    fn embed(&self, _text: &str) -> Result<Vec<f32>> {
        anyhow::bail!(
            "Candle backend is not enabled.\n\
            \n\
            This backend requires the 'candle' feature which is not currently available.\n\
            Use the FastEmbed backend by enabling the 'embeddings' feature instead:\n\
            `apchat-skills = {{ path = \"...\", features = [\"embeddings\"] }}`\n\
            \n\
            The Candle backend implementation would need:\n\
            - candle-core for tensor operations\n\
            - candle-transformers for model definitions\n\
            - tokenizers for text tokenization\n\
            - A pre-trained sentence-transformers model"
        );
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn backend_name(&self) -> &str {
        "candle"
    }
}

// Implementation notes for future work:
// 
// To fully implement the Candle backend:
// 
// 1. Add dependencies to Cargo.toml:
// ```toml
// [features]
// embeddings = ["fastembed"]
// candle = ["embeddings", "candle-core", "candle-transformers", "tokenizers"]
// 
// candle-core = { version = "0.7", features = ["metal", "cudnn"] }  # Auto-select GPU
// candle-transformers = "0.7"
// tokenizers = "0.19"
// ```
// 
// 2. Load model (e.g., all-MiniLM-L6-v2):
// ```rust
// use candle_core::{Device, Tensor};
// use candle_transformers::models::bert::{BertModel, Config};
// use tokenizers::Tokenizer;
// 
// pub struct CandleBackend {
//     model: BertModel,
//     tokenizer: Tokenizer,
//     device: Device,
//     dimension: usize,
// }
// ```
// 
// 3. Tokenize input and run inference:
// ```rust
// let tokens = self.tokenizer.encode(text, true)?;
// let token_ids = Tensor::new(tokens.get_ids(), &self.device)?;
// let embeddings = self.model.forward(&token_ids)?;
// // Extract [CLS] token embedding or average pooling
// ```
