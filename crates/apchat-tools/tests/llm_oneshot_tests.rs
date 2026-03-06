#[cfg(test)]
mod llm_oneshot_tests {
    use apchat_tools::llm_oneshot::LlmCallTool;
    use apchat_toolcore::{ToolParameters, Tool};
    use apchat_toolcore::tool_context::ToolContext;
    use std::path::PathBuf;
    use apchat_policy::PolicyManager;
    use apchat_models::types::ModelColor;
    use apchat_models::types::ContentPart;
    use apchat_llm_api::client::{LlmClient, ChatMessage};
    use std::sync::Arc;
    use std::collections::HashMap;
    use async_trait::async_trait;
    use apchat_toolcore::tool_registry::ToolRegistry;
    
    /// Mock LLM client for testing
    #[derive(Debug)]
    struct MockLlmClient;
    
    #[async_trait]
    impl LlmClient for MockLlmClient {
        async fn chat(&self, _messages: Vec<ChatMessage>, _tools: Vec<apchat_llm_api::client::ToolDefinition>) -> Result<apchat_llm_api::client::LlmResponse, anyhow::Error> {
            // Not used by llm_oneshot tool
            Err(anyhow::anyhow!("chat method not implemented for testing"))
        }

        async fn chat_completion(&self, messages: &[ChatMessage]) -> Result<String, anyhow::Error> {
            // Extract text content from ContentPart
            let prompt = messages[0].content.iter()
                .filter_map(|part| {
                    if let ContentPart::Text(text) = part {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("");
            
            if prompt.contains("Original instruction") && prompt.contains("File contents:") && prompt.contains("File content to append") {
                Ok("Mock response: I received the instruction with file contents".to_string())
            } else if prompt.contains("Hello, world!") {
                Ok("Mock response: Hello, world!".to_string())
            } else if prompt.contains("Hello from registry!") {
                Ok("Mock response: Hello from registry!".to_string())
            } else if prompt.contains("Test instruction") && !prompt.contains("File contents:") {
                Ok("Mock response: Test instruction".to_string())
            } else if prompt.contains("Registry instruction") && prompt.contains("File contents:") && prompt.contains("Registry file content") {
                Ok("Mock response: I received the instruction with file contents".to_string())
            } else if prompt.contains("Original instruction") && prompt.contains("File contents:") {
                Ok("Mock response: I received the instruction with file contents".to_string())
            } else if prompt.contains("\n\nFile contents:\n") && !prompt.contains("File content to append") {
                Ok("Mock response: Instruction with empty file".to_string())
            } else {
                Ok(format!("Mock response: {}", prompt))
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
        
        // Check parameter properties
        let model_color_param = params.get("model_color").unwrap();
        assert!(model_color_param.required);
        assert_eq!(model_color_param.param_type, "string");
        
        let instruction_param = params.get("instruction").unwrap();
        assert!(instruction_param.required);
        assert_eq!(instruction_param.param_type, "string");
        
        let file_path_param = params.get("file_path").unwrap();
        assert!(!file_path_param.required); // file_path is optional
        assert_eq!(file_path_param.param_type, "string");
    }
    
    #[tokio::test]
    async fn test_llm_oneshot_missing_required_parameter() {
        let tool = LlmCallTool;
        
        // Create ToolParameters missing required model_color
        let mut params = ToolParameters::new();
        params.set("instruction", "Hello, world!");
        
        let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
        
        let result = tool.execute(params, &context).await;
        
        // Check that we get an error about missing parameter
        assert!(!result.success, "Expected error result, got success");
        assert!(result.error.is_some(), "Expected error message");
        let error_msg = result.error.unwrap();
        assert!(error_msg.contains("Missing required parameter") || error_msg.contains("model_color"),
            "Expected 'Missing required parameter' error, got: {}", error_msg);
    }
    
    #[tokio::test]
    async fn test_llm_oneshot_missing_instruction() {
        let tool = LlmCallTool;
        
        // Create ToolParameters missing required instruction
        let mut params = ToolParameters::new();
        params.set("model_color", "grn");
        
        let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
        
        let result = tool.execute(params, &context).await;
        
        // Check that we get an error about missing parameter
        assert!(!result.success, "Expected error result, got success");
        assert!(result.error.is_some(), "Expected error message");
        let error_msg = result.error.unwrap();
        assert!(error_msg.contains("Missing required parameter") || error_msg.contains("instruction"),
            "Expected 'Missing required parameter' error, got: {}", error_msg);
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
    async fn test_llm_oneshot_invalid_model_colors() {
        let tool = LlmCallTool;
        
        // Test various invalid model colors
        let invalid_colors = vec!["redd", "green", "blue", "purple", "yellow", "", "  "];
        
        for invalid_color in invalid_colors {
            let mut params = ToolParameters::new();
            params.set("model_color", invalid_color);
            params.set("instruction", "Test instruction");
            
            let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
            
            let result = tool.execute(params, &context).await;
            
            // Check that we get an error about invalid model color
            assert!(!result.success, "Expected error result for color '{}', got success", invalid_color);
            assert!(result.error.is_some(), "Expected error message for color '{}'", invalid_color);
            assert!(result.error.unwrap().contains("Invalid model color"), 
                "Expected 'Invalid model color' error for color '{}'", invalid_color);
        }
    }
    
    #[tokio::test]
    async fn test_llm_oneshot_valid_model_colors() {
        let tool = LlmCallTool;
        
        // Test all valid model colors
        let valid_colors = vec![("red", ModelColor::RedModel), ("grn", ModelColor::GrnModel), ("blu", ModelColor::BluModel)];
        
        for (color_str, _color_enum) in valid_colors {
            let mut params = ToolParameters::new();
            params.set("model_color", color_str);
            params.set("instruction", "Test instruction");
            
            // Create context with all clients
            let mut context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
            let mut clients: HashMap<ModelColor, Arc<dyn LlmClient>> = HashMap::new();
            clients.insert(ModelColor::RedModel, Arc::new(MockLlmClient));
            clients.insert(ModelColor::GrnModel, Arc::new(MockLlmClient));
            clients.insert(ModelColor::BluModel, Arc::new(MockLlmClient));
            context = context.with_llm_clients(clients);
            
            let result = tool.execute(params, &context).await;
            
            // Should succeed for valid colors
            assert!(result.success, "Expected success for valid color '{}', got error: {:?}", color_str, result.error);
        }
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
    
    #[tokio::test]
    async fn test_llm_oneshot_empty_file() {
        use tempfile::NamedTempFile;
        
        // Create a temporary empty file
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_str().unwrap().to_string();
        
        let tool = LlmCallTool;
        
        // Create ToolParameters with required fields and empty file path
        let mut params = ToolParameters::new();
        params.set("model_color", "grn");
        params.set("instruction", "Original instruction");
        params.set("file_path", file_path);
        
        // Create context with mock client
        let mut context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
        let mut clients: HashMap<ModelColor, Arc<dyn LlmClient>> = HashMap::new();
        clients.insert(ModelColor::GrnModel, Arc::new(MockLlmClient));
        context = context.with_llm_clients(clients);
        
        let result = tool.execute(params, &context).await;
        
        // Check that we get a successful result
        assert!(result.success, "Expected success result, got error: {:?}", result.error);
        // The response should contain the instruction or be successful (empty file should work)
        assert!(result.success, "Tool should succeed with empty file");
    }
    
    #[tokio::test]
    async fn test_llm_oneshot_permissions_error() {
        use tempfile::NamedTempFile;
        use std::os::unix::fs::PermissionsExt;
        
        // Create a temporary file with no read permissions
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_str().unwrap().to_string();
        
        // Set permissions to 0 (no read)
        std::fs::set_permissions(temp_file.path(), std::fs::Permissions::from_mode(0o000)).unwrap();
        
        let tool = LlmCallTool;
        
        // Create ToolParameters with required fields and restricted file path
        let mut params = ToolParameters::new();
        params.set("model_color", "grn");
        params.set("instruction", "Test instruction");
        params.set("file_path", file_path);
        
        let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
        
        let result = tool.execute(params, &context).await;
        
        // Check that we get an error about file read failure
        assert!(!result.success, "Expected error result, got success");
        assert!(result.error.is_some(), "Expected error message");
        assert!(result.error.unwrap().contains("Failed to read file"), "Expected 'Failed to read file' error");
    }
    
    #[tokio::test]
    async fn test_llm_oneshot_tool_registry_integration() {
        // Test that the tool can be registered and discovered in ToolRegistry
        let mut registry = ToolRegistry::new();
        
        // Register the tool
        registry.register(LlmCallTool);
        
        // Verify the tool can be discovered
        assert!(registry.has_tool("llm_oneshot"), "Tool 'llm_oneshot' should be registered");
        
        // Verify we can get the tool
        let retrieved_tool = registry.get_tool("llm_oneshot");
        assert!(retrieved_tool.is_some(), "Should be able to retrieve 'llm_oneshot' tool");
        
        // Verify the tool has the correct name
        let tool = retrieved_tool.unwrap();
        assert_eq!(tool.name(), "llm_oneshot");
    }
    
    #[tokio::test]
    async fn test_llm_oneshot_tool_registry_execution() {
        // Test that the tool can be executed through ToolRegistry
        let mut registry = ToolRegistry::new();
        
        // Register the tool
        registry.register(LlmCallTool);
        
        // Create ToolParameters without file path
        let mut params = ToolParameters::new();
        params.set("model_color", "grn");
        params.set("instruction", "Hello from registry!");
        
        // Create context with mock client
        let mut context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
        let mut clients: HashMap<ModelColor, Arc<dyn LlmClient>> = HashMap::new();
        clients.insert(ModelColor::GrnModel, Arc::new(MockLlmClient));
        context = context.with_llm_clients(clients);
        
        // Execute through registry
        let result = registry.execute_tool("llm_oneshot", params, &context).await;
        
        // Check that we get a successful result
        assert!(result.success, "Expected success result, got error: {:?}", result.error);
        assert_eq!(result.content, "Mock response: Hello from registry!");
    }
    
    #[tokio::test]
    async fn test_llm_oneshot_tool_registry_openai_definition() {
        // Test that the tool can be converted to OpenAI format through ToolRegistry
        let mut registry = ToolRegistry::new();
        
        // Register the tool
        registry.register(LlmCallTool);
        
        // Get OpenAI tool definitions
        let definitions = registry.get_openai_tool_definitions();
        
        // Find our tool
        let llm_tool = definitions.iter().find(|def| {
            if let Some(name) = def.get("function").and_then(|f| f.get("name")) {
                if let Some(s) = name.as_str() {
                    return s == "llm_oneshot";
                }
            }
            false
        });
        
        assert!(llm_tool.is_some(), "Should find llm_oneshot in OpenAI definitions");
        
        let tool_def = llm_tool.unwrap();
        let function = tool_def.get("function").unwrap();
        
        // Verify function name
        assert_eq!(function.get("name").unwrap().as_str().unwrap(), "llm_oneshot");
        
        // Verify parameters exist
        let params = function.get("parameters").unwrap().get("properties").unwrap().as_object().unwrap();
        assert!(params.contains_key("model_color"));
        assert!(params.contains_key("instruction"));
        assert!(params.contains_key("file_path"));
    }
    
    #[tokio::test]
    async fn test_llm_oneshot_tool_registry_with_file() {
        // Test that the tool can be executed through ToolRegistry with a file
        use std::io::Write;
        use tempfile::NamedTempFile;
        
        // Create a temporary file with content
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Registry file content").unwrap();
        let file_path = temp_file.path().to_str().unwrap().to_string();
        
        let mut registry = ToolRegistry::new();
        registry.register(LlmCallTool);
        
        // Create ToolParameters with file path
        let mut params = ToolParameters::new();
        params.set("model_color", "blu");
        params.set("instruction", "Registry instruction");
        params.set("file_path", file_path);
        
        // Create context with mock client
        let mut context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
        let mut clients: HashMap<ModelColor, Arc<dyn LlmClient>> = HashMap::new();
        clients.insert(ModelColor::BluModel, Arc::new(MockLlmClient));
        context = context.with_llm_clients(clients);
        
        // Execute through registry
        let result = registry.execute_tool("llm_oneshot", params, &context).await;
        
        // Check that we get a successful result
        assert!(result.success, "Expected success result, got error: {:?}", result.error);
        assert_eq!(result.content, "Mock response: I received the instruction with file contents");
    }
    
    #[tokio::test]
    async fn test_llm_oneshot_tool_registry_error_propagation() {
        // Test that errors are properly propagated through ToolRegistry
        let mut registry = ToolRegistry::new();
        registry.register(LlmCallTool);
        
        // Create ToolParameters with invalid model color
        let mut params = ToolParameters::new();
        params.set("model_color", "invalid");
        params.set("instruction", "Test instruction");
        
        let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
        
        // Execute through registry
        let result = registry.execute_tool("llm_oneshot", params, &context).await;
        
        // Check that we get an error
        assert!(!result.success, "Expected error result, got success");
        assert!(result.error.is_some(), "Expected error message");
        assert!(result.error.unwrap().contains("Invalid model color"), "Expected 'Invalid model color' error");
    }
}