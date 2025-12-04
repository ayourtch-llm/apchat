/// Embedding backend abstraction for skill similarity search
use anyhow::Result;

/// Trait for embedding model backends
/// Allows swapping between fastembed, candle, or other implementations
pub trait EmbeddingBackend: Send + Sync {
    /// Generate an embedding vector for the given text
    fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Generate embeddings for multiple texts (batch operation)
    /// Default implementation calls embed() for each text
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        texts.iter()
            .map(|text| self.embed(text))
            .collect()
    }

    /// Get the dimension of the embedding vectors
    fn dimension(&self) -> usize;

    /// Get the name of the backend for debugging
    fn backend_name(&self) -> &str;
}

/// Compute cosine similarity between two embedding vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

// Feature-gated backend implementations
#[cfg(feature = "fastembed")]
pub mod fastembed_backend;

#[cfg(feature = "candle")]
pub mod candle_backend;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 0.001);

        let a = vec![1.0, 1.0, 0.0];
        let b = vec![1.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
    }
}
