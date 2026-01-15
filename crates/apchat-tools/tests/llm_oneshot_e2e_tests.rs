#![cfg(test)]
mod llm_oneshot_e2e_tests {
    use apchat_tools::llm_oneshot::LlmCallTool;
    use apchat_toolcore::tool_registry::ToolRegistry;
    use apchat_toolcore::ToolParameters;

    #[test]
    fn test_tool_can_be_registered_and_discovered() {
        let mut registry = ToolRegistry::new();
        registry.register(LlmCallTool);
        
        assert!(registry.has_tool("llm_oneshot"));
        
        let tool = registry.get_tool("llm_oneshot").unwrap();
        assert_eq!(tool.name(), "llm_oneshot");
        assert_eq!(tool.description(), "Make a one-shot call to an LLM model. Accepts model color (red/grn/blu), instruction, and optionally a file path to append to the instruction.");
    }

    #[test]
    fn test_tool_parameter_parsing() {
        // Test that ToolParameters can be created and parsed correctly
        let mut params = ToolParameters::new();
        params.set("model_color", "grn");
        params.set("instruction", "Hello, this is a test");
        
        // Verify parameters are set using get_required
        assert_eq!(params.get_required::<String>("model_color").unwrap(), "grn");
        assert_eq!(params.get_required::<String>("instruction").unwrap(), "Hello, this is a test");
    }

    #[test]
    fn test_tool_parameter_parsing_with_file() {
        // Test that ToolParameters with file_path can be parsed correctly
        let mut params = ToolParameters::new();
        params.set("model_color", "blu");
        params.set("instruction", "Analyze this code");
        params.set("file_path", "src/main.rs");
        
        // Verify all parameters are set
        assert_eq!(params.get_required::<String>("model_color").unwrap(), "blu");
        assert_eq!(params.get_required::<String>("instruction").unwrap(), "Analyze this code");
        assert_eq!(params.get_required::<String>("file_path").unwrap(), "src/main.rs");
    }

    #[test]
    fn test_tool_parameter_parsing_optional() {
        // Test that optional file_path can be parsed correctly
        let mut params = ToolParameters::new();
        params.set("model_color", "grn");
        params.set("instruction", "Hello, this is a test");
        // file_path is not set
        
        // Verify required parameters are set
        assert_eq!(params.get_required::<String>("model_color").unwrap(), "grn");
        assert_eq!(params.get_required::<String>("instruction").unwrap(), "Hello, this is a test");
        
        // Verify optional parameter returns None when not set
        assert_eq!(params.get_optional::<String>("file_path").unwrap(), None);
    }
}


