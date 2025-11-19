#[cfg(test)]
mod model_config_tests {
    use crate::config::{parse_model_attings, BackendType, get_default_url_for_backend, get_default_model_for_backend};

    #[test]
    fn test_parse_model_full_format() {
        let (model, backend, url) = parse_model_attings("llama-3.1-70b@anthropic(https://api.anthropic.com)");
        
        assert_eq!(model, "llama-3.1-70b");
        assert_eq!(backend, Some(BackendType::Anthropic));
        assert_eq!(url, Some("https://api.anthropic.com".to_string()));
    }

    #[test]
    fn test_parse_model_backend_only() {
        let (model, backend, url) = parse_model_attings("gpt-4@openai");
        
        assert_eq!(model, "gpt-4");
        assert_eq!(backend, Some(BackendType::OpenAI));
        assert_eq!(url, None); // URL should be None for backend-only format
    }

    #[test]
    fn test_parse_model_only() {
        let (model, backend, url) = parse_model_attings("llama-3.1-70b");
        
        assert_eq!(model, "llama-3.1-70b");
        assert_eq!(backend, None);
        assert_eq!(url, None);
    }

    #[test]
    fn test_parse_model_groq_backend() {
        let (model, backend, url) = parse_model_attings("llama-3.1-70b@groq");
        
        assert_eq!(model, "llama-3.1-70b");
        assert_eq!(backend, Some(BackendType::Groq));
        assert_eq!(url, None);
    }

    #[test]
    fn test_parse_model_with_custom_url() {
        let (model, backend, url) = parse_model_attings("custom-model@llama(http://localhost:8080/completions)");
        
        assert_eq!(model, "custom-model");
        assert_eq!(backend, Some(BackendType::Llama));
        assert_eq!(url, Some("http://localhost:8080/completions".to_string()));
    }

    #[test]
    fn test_parse_model_empty_model() {
        let (model, backend, url) = parse_model_attings("@anthropic");
        
        assert_eq!(model, "claude-3-5-sonnet-20241022"); // Should now return default model
        assert_eq!(backend, Some(BackendType::Anthropic));
        assert_eq!(url, Some("https://api.anthropic.com".to_string())); // Should now return default URL
    }

    #[test]
    fn test_parse_model_multiple_at_symbols() {
        let (model, backend, url) = parse_model_attings("model@with@multiple@anthropic");
        
        // Should split on first @ only and treat "with@multiple@anthropic" as backend
        assert_eq!(model, "model");
        assert_eq!(backend, None); // "with@multiple@anthropic" is not a valid backend
        assert_eq!(url, None);
    }

    #[test]
    fn test_parse_model_malformed_parentheses() {
        let (model, backend, url) = parse_model_attings("model@anthropic(https://example.com");
        
        // Should handle malformed parentheses gracefully by parsing model part and ignoring backend
        // because parentheses are malformed (missing closing ')')
        assert_eq!(model, "model");
        assert_eq!(backend, None);
        assert_eq!(url, None);
    }

    #[test]
    fn test_parse_model_case_insensitive_backend() {
        let (model, backend, url) = parse_model_attings("model@ANTHROPIC");
        
        assert_eq!(model, "model");
        assert_eq!(backend, Some(BackendType::Anthropic));
        assert_eq!(url, None);
    }

    #[test]
    fn test_parse_model_claude_alias() {
        let (model, backend, url) = parse_model_attings("claude-3.5-sonnet@claude");
        
        assert_eq!(model, "claude-3.5-sonnet");
        assert_eq!(backend, Some(BackendType::Anthropic));
        assert_eq!(url, None);
    }

    #[test]
    fn test_parse_model_llama_aliases() {
        // Test various llama backend aliases
        let test_cases = vec![
            ("model@llama", BackendType::Llama),
            ("model@llamacpp", BackendType::Llama),
            ("model@llama.cpp", BackendType::Llama),
            ("model@llama-cpp", BackendType::Llama),
        ];

        for (input, expected_backend) in test_cases {
            let (model, backend, url) = parse_model_attings(input);
            assert_eq!(model, "model");
            assert_eq!(backend, Some(expected_backend));
            assert_eq!(url, None);
        }
    }

    #[test]
    fn test_parse_model_unknown_backend() {
        let (model, backend, url) = parse_model_attings("model@unknown");
        
        assert_eq!(model, "model");
        assert_eq!(backend, None); // Unknown backend should return None
        assert_eq!(url, None);
    }

    #[test]
    fn test_get_default_url_anthropic() {
        let url = get_default_url_for_backend(&BackendType::Anthropic);
        assert_eq!(url, Some("https://api.anthropic.com".to_string()));
    }

    #[test]
    fn test_get_default_url_groq() {
        let url = get_default_url_for_backend(&BackendType::Groq);
        assert_eq!(url, Some("https://api.groq.com/openai/v1/chat/completions".to_string()));
    }

    #[test]
    fn test_get_default_url_openai() {
        let url = get_default_url_for_backend(&BackendType::OpenAI);
        assert_eq!(url, Some("https://api.openai.com/v1/chat/completions".to_string()));
    }

    #[test]
    fn test_get_default_url_llama() {
        let url = get_default_url_for_backend(&BackendType::Llama);
        assert_eq!(url, None); // Llama has no default URL
    }

    #[test]
    fn test_parse_model_integration_with_default_urls() {
        // Test the complete flow: parse model config, then get default URL
        
        // Case 1: model@anthropic should get Anthropic default URL
        let (model, backend, url) = parse_model_attings("foo@anthropic");
        assert_eq!(model, "foo");
        assert_eq!(backend, Some(BackendType::Anthropic));
        assert_eq!(url, None); // URL is None after parsing
        
        let default_url = get_default_url_for_backend(&backend.unwrap());
        assert_eq!(default_url, Some("https://api.anthropic.com".to_string()));
        
        // Case 2: model@groq should get Groq default URL
        let (model, backend, url) = parse_model_attings("bar@groq");
        assert_eq!(model, "bar");
        assert_eq!(backend, Some(BackendType::Groq));
        assert_eq!(url, None);
        
        let default_url = get_default_url_for_backend(&backend.unwrap());
        assert_eq!(default_url, Some("https://api.groq.com/openai/v1/chat/completions".to_string()));
        
        // Case 3: model@openai should get OpenAI default URL
        let (model, backend, url) = parse_model_attings("baz@openai");
        assert_eq!(model, "baz");
        assert_eq!(backend, Some(BackendType::OpenAI));
        assert_eq!(url, None);
        
        let default_url = get_default_url_for_backend(&backend.unwrap());
        assert_eq!(default_url, Some("https://api.openai.com/v1/chat/completions".to_string()));
        
        // Case 4: model@llama should get no default URL
        let (model, backend, url) = parse_model_attings("qux@llama");
        assert_eq!(model, "qux");
        assert_eq!(backend, Some(BackendType::Llama));
        assert_eq!(url, None);
        
        let default_url = get_default_url_for_backend(&backend.unwrap());
        assert_eq!(default_url, None);
    }

    #[test]
    fn test_edge_cases() {
        // Empty string
        let (model, backend, url) = parse_model_attings("");
        assert_eq!(model, "");
        assert_eq!(backend, None);
        assert_eq!(url, None);
        
        // Just @ symbol (invalid backend)
        let (model, backend, url) = parse_model_attings("@");
        assert_eq!(model, "@"); // Should fallback to treating as model name
        assert_eq!(backend, None);
        assert_eq!(url, None);
        
        // Multiple @ symbols with empty backend
        let (model, backend, url) = parse_model_attings("model@@");
        assert_eq!(model, "model");
        assert_eq!(backend, None);
        assert_eq!(url, None);
    }

    // ===== NEW TESTS FOR @backend SYNTAX =====

    #[test]
    fn test_parse_backend_only_syntax() {
        // Test @anthropic
        let (model, backend, url) = parse_model_attings("@anthropic");
        assert_eq!(model, "claude-3-5-sonnet-20241022");
        assert_eq!(backend, Some(BackendType::Anthropic));
        assert_eq!(url, Some("https://api.anthropic.com".to_string()));

        // Test @openai
        let (model, backend, url) = parse_model_attings("@openai");
        assert_eq!(model, "gpt-4o-mini");
        assert_eq!(backend, Some(BackendType::OpenAI));
        assert_eq!(url, Some("https://api.openai.com/v1/chat/completions".to_string()));

        // Test @groq
        let (model, backend, url) = parse_model_attings("@groq");
        assert_eq!(model, "llama-3.1-8b-instant");
        assert_eq!(backend, Some(BackendType::Groq));
        assert_eq!(url, Some("https://api.groq.com/openai/v1/chat/completions".to_string()));

        // Test @llama
        let (model, backend, url) = parse_model_attings("@llama");
        assert_eq!(model, "llama3.1");
        assert_eq!(backend, Some(BackendType::Llama));
        assert_eq!(url, None); // Llama has no default URL
    }

    #[test]
    fn test_parse_backend_with_custom_url_syntax() {
        // Test @anthropic(custom_url)
        let (model, backend, url) = parse_model_attings("@anthropic(https://custom.anthropic.com)");
        assert_eq!(model, "claude-3-5-sonnet-20241022");
        assert_eq!(backend, Some(BackendType::Anthropic));
        assert_eq!(url, Some("https://custom.anthropic.com".to_string()));

        // Test @groq(custom_url)
        let (model, backend, url) = parse_model_attings("@groq(https://custom.groq.com/v1/chat/completions)");
        assert_eq!(model, "llama-3.1-8b-instant");
        assert_eq!(backend, Some(BackendType::Groq));
        assert_eq!(url, Some("https://custom.groq.com/v1/chat/completions".to_string()));

        // Test @llama(custom_url)
        let (model, backend, url) = parse_model_attings("@llama(http://localhost:8080/completions)");
        assert_eq!(model, "llama3.1");
        assert_eq!(backend, Some(BackendType::Llama));
        assert_eq!(url, Some("http://localhost:8080/completions".to_string()));
    }

    #[test]
    fn test_invalid_backend_syntax_fallback() {
        // Test invalid @backend should fallback to treating as model name
        let (model, backend, url) = parse_model_attings("@invalidbackend");
        assert_eq!(model, "@invalidbackend");
        assert_eq!(backend, None);
        assert_eq!(url, None);

        // Test @backend with malformed parentheses should fallback
        let (model, backend, url) = parse_model_attings("@anthropic(https://example.com");
        assert_eq!(model, "@anthropic(https://example.com");
        assert_eq!(backend, None);
        assert_eq!(url, None);
    }

    #[test]
    fn test_get_default_model_for_backend() {
        assert_eq!(get_default_model_for_backend(&BackendType::Anthropic), "claude-3-5-sonnet-20241022");
        assert_eq!(get_default_model_for_backend(&BackendType::OpenAI), "gpt-4o-mini");
        assert_eq!(get_default_model_for_backend(&BackendType::Groq), "llama-3.1-8b-instant");
        assert_eq!(get_default_model_for_backend(&BackendType::Llama), "llama3.1");
    }

    #[test]
    fn test_case_insensitive_backend_syntax() {
        // Test case insensitive @backend syntax
        let (model, backend, url) = parse_model_attings("@ANTHROPIC");
        assert_eq!(model, "claude-3-5-sonnet-20241022");
        assert_eq!(backend, Some(BackendType::Anthropic));
        assert_eq!(url, Some("https://api.anthropic.com".to_string()));

        let (model, backend, url) = parse_model_attings("@GROQ");
        assert_eq!(model, "llama-3.1-8b-instant");
        assert_eq!(backend, Some(BackendType::Groq));
        assert_eq!(url, Some("https://api.groq.com/openai/v1/chat/completions".to_string()));
    }

    #[test]
    fn test_backend_alias_syntax() {
        // Test claude alias for anthropic
        let (model, backend, url) = parse_model_attings("@claude");
        assert_eq!(model, "claude-3-5-sonnet-20241022");
        assert_eq!(backend, Some(BackendType::Anthropic));
        assert_eq!(url, Some("https://api.anthropic.com".to_string()));

        // Test various llama aliases
        let test_cases = vec![
            ("@llama", "llama3.1"),
            ("@llamacpp", "llama3.1"),
            ("@llama.cpp", "llama3.1"),
            ("@llama-cpp", "llama3.1"),
        ];

        for (input, expected_model) in test_cases {
            let (model, backend, url) = parse_model_attings(input);
            assert_eq!(model, expected_model);
            assert_eq!(backend, Some(BackendType::Llama));
            assert_eq!(url, None);
        }
    }
}