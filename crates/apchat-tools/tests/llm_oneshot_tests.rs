#[cfg(test)]
mod llm_oneshot_tests {
    use apchat_tools::llm_oneshot::LlmCallTool;
    use apchat_toolcore::{ToolParameters, ToolContext, Tool};
    
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
        let mut params = ToolParameters::new();
        params.set("model_color", "grn");
        params.set("instruction", "Hello, world!");
        
        let context = ToolContext::new(std::env::current_dir().unwrap(), "test_session".to_string(), apchat_policy::PolicyManager::new());
        
        let result = tool.execute(params, &context).await;
        
        if !result.success {
            panic!("Tool execution failed: {:?}", result.error);
        }
    }
}