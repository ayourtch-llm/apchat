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
    async fn test_llm_oneshot_with_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;
        
        // Create a temporary file with content
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "File content to append").unwrap();
        let file_path = temp_file.path().to_str().unwrap().to_string();
        
        let tool = LlmCallTool;
        
        // Create ToolParameters with required fields and file path
        let mut params = ToolParameters::new();
        params.set("model_color", "blu");
        params.set("instruction", "Original instruction");
        params.set("file_path", file_path);
        
        let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
        
        let result = tool.execute(params, &context).await;
        
        // Check that we get the expected error since client access isn't implemented yet
        assert!(!result.success, "Expected error result, got success");
        assert!(result.error.is_some(), "Expected error message");
        assert!(result.error.unwrap().contains("not yet implemented"), "Expected 'not yet implemented' error");
    }
}



