#[cfg(test)]
mod tests {
    use crate::{ModelColor, ModelProvider, BackendType};

    #[test]
    fn test_model_provider_indexed_by_color() {
        // Create model providers for each color
        let providers = [
            ModelProvider::with_config(
                "moonshotai/kimi-k2-instruct-0905".to_string(),
                Some(BackendType::Groq),
                Some("https://api.groq.com".to_string()),
                Some("key1".to_string()),
            ),
            ModelProvider::with_config(
                "openai/gpt-oss-120b".to_string(),
                Some(BackendType::OpenAI),
                Some("https://api.openai.com".to_string()),
                Some("key2".to_string()),
            ),
            ModelProvider::with_config(
                "meta-llama/llama-3.1-70b-versatile".to_string(),
                Some(BackendType::Anthropic),
                Some("https://api.anthropic.com".to_string()),
                Some("key3".to_string()),
            ),
        ];

        // Test that indexing works correctly with explicit discriminants
        assert_eq!(providers[ModelColor::BluModel as usize].model_name, "moonshotai/kimi-k2-instruct-0905");
        assert_eq!(providers[ModelColor::GrnModel as usize].model_name, "openai/gpt-oss-120b");
        assert_eq!(providers[ModelColor::RedModel as usize].model_name, "meta-llama/llama-3.1-70b-versatile");

        // Test backends
        assert_eq!(providers[ModelColor::BluModel as usize].backend, Some(BackendType::Groq));
        assert_eq!(providers[ModelColor::GrnModel as usize].backend, Some(BackendType::OpenAI));
        assert_eq!(providers[ModelColor::RedModel as usize].backend, Some(BackendType::Anthropic));

        // Test API URLs
        assert_eq!(providers[ModelColor::BluModel as usize].api_url, Some("https://api.groq.com".to_string()));
        assert_eq!(providers[ModelColor::GrnModel as usize].api_url, Some("https://api.openai.com".to_string()));
        assert_eq!(providers[ModelColor::RedModel as usize].api_url, Some("https://api.anthropic.com".to_string()));

        // Test API keys
        assert_eq!(providers[ModelColor::BluModel as usize].api_key, Some("key1".to_string()));
        assert_eq!(providers[ModelColor::GrnModel as usize].api_key, Some("key2".to_string()));
        assert_eq!(providers[ModelColor::RedModel as usize].api_key, Some("key3".to_string()));
    }

    #[test]
    fn test_model_color_discriminants() {
        // Test that the discriminants are correct
        assert_eq!(ModelColor::BluModel as usize, 0);
        assert_eq!(ModelColor::GrnModel as usize, 1);
        assert_eq!(ModelColor::RedModel as usize, 2);
    }

    #[test]
    fn test_model_provider_new() {
        let provider = ModelProvider::new("test-model".to_string());
        assert_eq!(provider.model_name, "test-model");
        assert_eq!(provider.backend, None);
        assert_eq!(provider.api_url, None);
        assert_eq!(provider.api_key, None);
    }
}