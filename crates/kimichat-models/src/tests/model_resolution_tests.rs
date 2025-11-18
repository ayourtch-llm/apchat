#[cfg(test)]
mod model_resolution_tests {
    use crate::types::ModelType;

    #[test]
    fn test_default_model_resolution() {
        assert_eq!(
            ModelType::BluModel.as_str_default(),
            "moonshotai/kimi-k2-instruct-0905"
        );
        assert_eq!(ModelType::GrnModel.as_str_default(), "openai/gpt-oss-120b");
        assert_eq!(
            ModelType::RedModel.as_str_default(),
            "meta-llama/llama-3.1-70b-versatile"
        );
        assert_eq!(
            ModelType::AnthropicModel.as_str_default(),
            "claude-3-5-sonnet-20241022"
        );
    }

    #[test]
    fn test_blu_model_override() {
        let custom_blu = Some("custom/blu-model");
        let no_grn = None;
        let no_red = None;

        assert_eq!(
            ModelType::BluModel.as_str(custom_blu, no_grn, no_red),
            "custom/blu-model"
        );
    }

    #[test]
    fn test_grn_model_override() {
        let no_blu = None;
        let custom_grn = Some("custom/grn-model");
        let no_red = None;

        assert_eq!(
            ModelType::GrnModel.as_str(no_blu, custom_grn, no_red),
            "custom/grn-model"
        );
    }

    #[test]
    fn test_red_model_override() {
        let no_blu = None;
        let no_grn = None;
        let custom_red = Some("custom/red-model");

        assert_eq!(
            ModelType::RedModel.as_str(no_blu, no_grn, custom_red),
            "custom/red-model"
        );
    }

    #[test]
    fn test_no_override_fallback() {
        let no_override = None;

        assert_eq!(
            ModelType::BluModel.as_str(no_override, no_override, no_override),
            "moonshotai/kimi-k2-instruct-0905"
        );
        assert_eq!(
            ModelType::GrnModel.as_str(no_override, no_override, no_override),
            "openai/gpt-oss-120b"
        );
        assert_eq!(
            ModelType::RedModel.as_str(no_override, no_override, no_override),
            "meta-llama/llama-3.1-70b-versatile"
        );
    }

    #[test]
    fn test_anthropic_model_no_override() {
        let no_override = None;

        // AnthropicModel should always return default since there's no override field
        assert_eq!(
            ModelType::AnthropicModel.as_str(no_override, no_override, no_override),
            "claude-3-5-sonnet-20241022"
        );
    }

    #[test]
    fn test_custom_model() {
        let no_override = None;
        let custom_model = ModelType::Custom("my-custom-model".to_string());

        // Custom models should ignore overrides and return their name
        assert_eq!(
            custom_model.as_str(Some("override"), no_override, no_override),
            "my-custom-model"
        );
        assert_eq!(
            custom_model.as_str(no_override, no_override, no_override),
            "my-custom-model"
        );
    }

    #[test]
    fn test_override_isolation() {
        let blu_override = Some("blu-override");
        let grn_override = Some("grn-override");
        let red_override = Some("red-override");

        // Each model should only use its own override
        assert_eq!(
            ModelType::BluModel.as_str(blu_override, grn_override, red_override),
            "blu-override"
        );
        assert_eq!(
            ModelType::GrnModel.as_str(blu_override, grn_override, red_override),
            "grn-override"
        );
        assert_eq!(
            ModelType::RedModel.as_str(blu_override, grn_override, red_override),
            "red-override"
        );
        assert_eq!(
            ModelType::AnthropicModel.as_str(blu_override, grn_override, red_override),
            "claude-3-5-sonnet-20241022" // No override for Anthropic
        );
    }

    #[test]
    fn test_mixed_override_scenarios() {
        let blu_override = Some("custom-blu");
        let no_grn = None;
        let red_override = Some("custom-red");

        // Test various combinations
        assert_eq!(
            ModelType::BluModel.as_str(blu_override, no_grn, red_override),
            "custom-blu"
        );
        assert_eq!(
            ModelType::GrnModel.as_str(blu_override, no_grn, red_override),
            "openai/gpt-oss-120b" // Falls back to default
        );
        assert_eq!(
            ModelType::RedModel.as_str(blu_override, no_grn, red_override),
            "custom-red"
        );
    }

    #[test]
    fn test_empty_string_overrides() {
        let empty_override = Some("");
        let no_override = None;

        // Empty string overrides should be used (not fall back to default)
        assert_eq!(
            ModelType::BluModel.as_str(empty_override, no_override, no_override),
            ""
        );
    }
}
