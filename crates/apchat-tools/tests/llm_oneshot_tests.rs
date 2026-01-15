#[cfg(test)]
mod llm_oneshot_tests {
    use apchat_tools::llm_oneshot::LlmCallTool;
    use apchat_toolcore::{ToolParameters, Tool};
    use apchat_toolcore::tool_context::ToolContext;
    use std::path::PathBuf;
    use apchat_policy::PolicyManager;
    use apchat_models::types::ModelColor;
    use apchat_llm_api::client::{LlmClient, ChatMessage};
    use std::sync::Arc;
    use std::collections::HashMap;
    use async_trait::async_trait;
    
    /// Mock LLM client for testing
    struct MockLlmClient;
    
    #[async_trait]
    impl LlmClient for MockLlmClient {
        async fn chat(&self, _messages: Vec<ChatMessage>, _tools: Vec<apchat_llm_api::client::ToolDefinition>) -> Result<apchat_llm_api::client::LlmResponse, anyhow::Error> {
            // Not used by llm_oneshot tool
            Err(anyhow::anyhow!("chat method not implemented for testing"))
        }

        async fn chat_completion(&self, messages: &[ChatMessage]) -> Result<String, anyhow::Error> {
            // Check if the prompt contains the expected content
            let prompt = &messages[0].content;
            
            if prompt.contains("Original instruction") && prompt.contains("File contents:") && prompt.contains("File content to append") {
                Ok("Mock response: I received the instruction with file contents".to_string())
            } else if prompt.contains("Hello, world!") {
                Ok("Mock response: Hello, world!".to_string())
            } else {
                Err(anyhow::anyhow!("Unexpected prompt: {}", prompt))
            }
        }
    }
    
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
        
        // Create ToolParameters without file path
        let mut params = ToolParameters::new();
        params.set("model_color", "grn");
        params.set("instruction", "Hello, world!");
        
        // Create context with mock client
        let mut context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
        let mut clients: HashMap<ModelColor, Arc<dyn LlmClient>> = HashMap::new();
        clients.insert(ModelColor::GrnModel, Arc::new(MockLlmClient));
        context = context.with_llm_clients(clients);
        
        let result = tool.execute(params, &context).await;
        
        // Check that we get a successful result
        assert!(result.success, "Expected success result, got error: {:?}", result.error);
        assert_eq!(result.content, "Mock response: Hello, world!");
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
        
        // Create context with mock client
        let mut context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
        let mut clients: HashMap<ModelColor, Arc<dyn LlmClient>> = HashMap::new();
        clients.insert(ModelColor::BluModel, Arc::new(MockLlmClient));
        context = context.with_llm_clients(clients);
        
        let result = tool.execute(params, &context).await;
        
        // Check that we get a successful result
        assert!(result.success, "Expected success result, got error: {:?}", result.error);
        assert_eq!(result.content, "Mock response: I received the instruction with file contents");
    }
    
    #[tokio::test]
    async fn test_llm_oneshot_no_client() {
        let tool = LlmCallTool;
        
        // Create ToolParameters
        let mut params = ToolParameters::new();
        params.set("model_color", "grn");
        params.set("instruction", "Hello, world!");
        
        // Create context without any clients
        let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
        
        let result = tool.execute(params, &context).await;
        
        // Check that we get an error about missing client
        assert!(!result.success, "Expected error result, got success");
        assert!(result.error.is_some(), "Expected error message");
        assert!(result.error.unwrap().contains("No LLM client configured"), "Expected 'No LLM client configured' error");
    }
    
    #[tokio::test]
    async fn test_llm_oneshot_invalid_model_color() {
        let tool = LlmCallTool;
        
        // Create ToolParameters with invalid model color
        let mut params = ToolParameters::new();
        params.set("model_color", "invalid");
        params.set("instruction", "Hello, world!");
        
        let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
        
        let result = tool.execute(params, &context).await;
        
        // Check that we get an error about invalid model color
        assert!(!result.success, "Expected error result, got success");
        assert!(result.error.is_some(), "Expected error message");
        assert!(result.error.unwrap().contains("Invalid model color"), "Expected 'Invalid model color' error");
    }
    
    #[tokio::test]
    async fn test_llm_oneshot_file_read_error() {
        let tool = LlmCallTool;
        
        // Create ToolParameters with non-existent file path
        let mut params = ToolParameters::new();
        params.set("model_color", "grn");
        params.set("instruction", "Hello, world!");
        params.set("file_path", "/non/existent/file.txt");
        
        let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
        
        let result = tool.execute(params, &context).await;
        
        // Check that we get an error about file read failure
        assert!(!result.success, "Expected error result, got success");
        assert!(result.error.is_some(), "Expected error message");
        assert!(result.error.unwrap().contains("Failed to read file"), "Expected 'Failed to read file' error");
    }
}



