#[cfg(test)]
mod llm_oneshot_tests {
    use apchat_tools::llm_oneshot::LlmCallTool;
    use apchat_toolcore::{ToolParameters, Tool};
    use apchat_toolcore::tool_context::ToolContext;
    use std::path::PathBuf;
    use apchat_policy::PolicyManager;
    
    #[tokio::test]
    async fn test_llm_oneshot_tool_parameters() {
        let tool = LlmCallTool;
        let params = tool.parameters();
        
        // Check that required parameters exist
        assert!(params.contains_key("model_color"));
        assert!(params.contains_key("instruction"));
        assert!(params.contains_key("file_path"));
    }
    
    #[tokio::test]
    async fn test_llm_oneshot_without_file() {
        let tool = LlmCallTool;
        
        // Create ToolParameters with required fields
        let mut params = ToolParameters::new();
        params.set("model_color", "grn");
        params.set("instruction", "Hello, world!");
        
        let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
        
        let result = tool.execute(params, &context).await;
        
        // Check that we get an error (expected since implementation is not complete)
        assert!(!result.success, "Expected error result, got success");
        assert!(result.error.is_some(), "Expected error message");
    }
}



