use super::EmbeddingBackend;
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
use anyhow::{Context, Result};
use apchat_common::ApChatPaths;
use candle_core::{DType, Device, Tensor};
use candle_transformers::models::bert::{BertModel, Config};
use std::sync::Arc;
use tokenizers::Tokenizer;

/// Candle-based embedding backend
/// Uses BERT-based sentence-transformers models
pub struct CandleBackend {
    model: Arc<BertModel>,
    tokenizer: Tokenizer,
    device: Device,
    dimension: usize,
}

impl CandleBackend {
    /// Get or create the cache directory for Candle models
    /// Returns ~/.cache/apchat/candle
    fn get_cache_dir() -> Result<std::path::PathBuf> {
        let cache_dir = ApChatPaths::candle_dir();

        // Create directory if it doesn't exist
        if !cache_dir.exists() {
            ApChatPaths::ensure_dir(&cache_dir).context("Failed to create cache directory")?;
        }

        Ok(cache_dir)
    }

    /// Create a new Candle backend with the all-MiniLM-L6-v2 model
    /// This is a small, efficient model with 384-dimensional embeddings
    pub fn new() -> Result<Self> {
        Self::with_model("sentence-transformers/all-MiniLM-L6-v2")
    }

    /// Create a new Candle backend with a specific model from Hugging Face Hub
    pub fn with_model(model_name: &str) -> Result<Self> {
        print_heart_yellow(
            format!("Loading Candle model: {}", model_name).as_str(),
            true,
        );

        // Get cache directory
        let cache_dir = Self::get_cache_dir()?;
        print_heart_yellow(
            format!("Using cache directory: {:?}", cache_dir).as_str(),
            true,
        );

        // Download and load the model
        let model_path = Self::download_model(model_name, &cache_dir)?;

        // Load tokenizer
        let tokenizer_path = model_path.join("tokenizer.json");
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .with_context(|| format!("Failed to load tokenizer from {:?}", tokenizer_path))?;

        // Load model config
        let config_path = model_path.join("config.json");
        let config_content = std::fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config.json from {:?}", config_path))?;
        let config: Config = serde_json::from_str(&config_content)
            .with_context(|| format!("Failed to parse config.json from {:?}", config_path))?;

        // Determine device (prefer CUDA if available, otherwise CPU)
        let device = Device::cuda_if_available(0)?;

        print_heart_yellow(
            format!("Loading model weights on {:?}...", device).as_str(),
            true,
        );

        // Load model weights from safetensors
        let weights_path = model_path.join("model.safetensors");
        let weights = unsafe { candle_core::safetensors::MmapedFile::new(weights_path)? };
        let weights = weights.deserialize()?;
        let vb = unsafe {
            candle_core::nn::VarBuilder::from_safetensors(vec![weights], DType::F32, &device)?
        };

        // Create model
        let model = BertModel::load(vb, &config).context("Failed to load BERT model")?;

        // Get dimension from config
        let dimension = config.hidden_size;

        print_heart_yellow(
            format!(
                "Candle model loaded successfully (dimension: {})",
                dimension
            )
            .as_str(),
            true,
        );

        Ok(Self {
            model: Arc::new(model),
            tokenizer,
            device,
            dimension,
        })
    }

    /// Download a model from Hugging Face Hub
    /// Returns the path to the model directory
    fn download_model(model_name: &str, cache_dir: &std::path::Path) -> Result<std::path::PathBuf> {
        let model_dir = cache_dir.join(model_name.replace('/', "--"));

        // Check if model already exists
        let tokenizer_exists = model_dir.join("tokenizer.json").exists();
        let config_exists = model_dir.join("config.json").exists();
        let weights_exists = model_dir.join("model.safetensors").exists();

        if tokenizer_exists && config_exists && weights_exists {
            print_heart_yellow(
                format!("Using cached model at {:?}", model_dir).as_str(),
                true,
            );
            return Ok(model_dir);
        }

        // Create model directory
        std::fs::create_dir_all(&model_dir).context("Failed to create model directory")?;

        print_heart_yellow(
            format!("Downloading model {}...", model_name).as_str(),
            true,
        );

        // Download tokenizer.json
        let tokenizer_url = format!(
            "https://huggingface.co/{}/resolve/main/tokenizer.json",
            model_name
        );
        let tokenizer_path = model_dir.join("tokenizer.json");
        Self::download_file(&tokenizer_url, &tokenizer_path)?;

        // Download config.json
        let config_url = format!(
            "https://huggingface.co/{}/resolve/main/config.json",
            model_name
        );
        let config_path = model_dir.join("config.json");
        Self::download_file(&config_url, &config_path)?;

        // Download model.safetensors
        let weights_url = format!(
            "https://huggingface.co/{}/resolve/main/model.safetensors",
            model_name
        );
        let weights_path = model_dir.join("model.safetensors");
        Self::download_file(&weights_url, &weights_path)?;

        print_heart_yellow(
            format!("Model downloaded to {:?}", model_dir).as_str(),
            true,
        );

        Ok(model_dir)
    }

    /// Download a file from a URL
    fn download_file(url: &str, path: &std::path::Path) -> Result<()> {
        use std::io::Write;

        let response = reqwest::blocking::get(url)
            .with_context(|| format!("Failed to download from {}", url))?;

        let bytes = response
            .bytes()
            .with_context(|| format!("Failed to read bytes from {}", url))?;

        let mut file = std::fs::File::create(path)
            .with_context(|| format!("Failed to create {}", path.display()))?;

        file.write_all(&bytes)
            .with_context(|| format!("Failed to write to {}", path.display()))?;

        Ok(())
    }
}

impl EmbeddingBackend for CandleBackend {
    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Tokenize input
        let encodings = self
            .tokenizer
            .encode(text, true)
            .context("Failed to encode text")?;

        let token_ids: Vec<u32> = encodings.get_ids().to_vec();
        let token_ids: Vec<i64> = token_ids.iter().map(|&x| x as i64).collect();

        // Create tensor from token IDs
        let token_ids_tensor = Tensor::new(token_ids.as_slice(), &self.device)?;

        // Add batch dimension
        let token_ids_tensor = token_ids_tensor.unsqueeze(0)?;

        // Create attention mask (all ones for simplicity, excluding padding)
        let seq_len = token_ids_tensor.d()[1];
        let attention_mask = Tensor::ones((1, seq_len), DType::F32, &self.device)?;

        // Run model inference
        // The BertModel forward method returns (sequence_output, pooled_output)
        let (_, pooled_output) = self.model.forward(&token_ids_tensor, &attention_mask)?;

        // Extract pooled output (the [CLS] token embedding)
        let embeddings = pooled_output;

        // Convert to CPU and extract as Vec<f32>
        let embeddings_cpu = embeddings.to_device(&Device::Cpu)?;
        let embeddings_data = embeddings_cpu
            .to_vec2::<f32>()
            .context("Failed to convert embeddings to Vec")?;

        // Return the pooled embedding (first element for BERT)
        Ok(embeddings_data[0].clone())
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        texts.iter().map(|text| self.embed(text)).collect()
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn backend_name(&self) -> &str {
        "candle"
    }
}

impl std::fmt::Debug for CandleBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CandleBackend")
            .field("dimension", &self.dimension)
            .field("device", &format!("{:?}", self.device))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Ignore by default as it downloads model
    fn test_candle_backend() {
        let backend = CandleBackend::new().unwrap();

        let embedding = backend.embed("test query").unwrap();
        assert_eq!(embedding.len(), backend.dimension());

        // Test that similar texts have similar embeddings
        let emb1 = backend.embed("debug a problem").unwrap();
        let emb2 = backend.embed("fix a bug").unwrap();
        let emb3 = backend.embed("cook a meal").unwrap();

        use crate::embeddings::cosine_similarity;

        let sim_similar = cosine_similarity(&emb1, &emb2);
        let sim_different = cosine_similarity(&emb1, &emb3);

        assert!(
            sim_similar > sim_different,
            "Similar texts should have higher similarity"
        );
    }
}

// Implementation notes for Candle backend:
//
// The implementation uses:
// 1. candle-core for tensor operations and device management
// 2. candle-transformers for BERT model definitions
// 3. tokenizers for text tokenization
// 4. Hugging Face Hub for downloading pre-trained models
//
// The all-MiniLM-L6-v2 model is a small, efficient sentence-transformers model
// with 384-dimensional embeddings that works well for general-purpose embedding tasks.
