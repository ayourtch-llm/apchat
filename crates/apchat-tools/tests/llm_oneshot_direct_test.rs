#[cfg(test)]
mod llm_oneshot_direct_test {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;
    use apchat_toolcore::{ToolRegistry, ToolParameters, ToolContext};
    use apchat_tools::llm_oneshot::LlmCallTool;
    use apchat_models::types::ModelColor;
    use apchat_llm_api::client::{LlmClient, ChatMessage, ToolDefinition};
    use async_trait::async_trait;

    // Mock LLM Client for testing
    #[derive(Debug)]
    struct MockLlmClient;

    #[async_trait]
    impl LlmClient for MockLlmClient {
        async fn chat(&self, _messages: Vec<ChatMessage>, _tools: Vec<ToolDefinition>) -> anyhow::Result<apchat_llm_api::client::LlmResponse> {
            // Return a successful response with a test message
            Ok(apchat_llm_api::client::LlmResponse {
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: "Test response from mock LLM".to_string(),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                    reasoning: None,
                },
                usage: None,
                finish_reason: Some("stop".to_string()),
            })
        }

        async fn chat_completion(&self, _messages: &[ChatMessage]) -> anyhow::Result<String> {
            Ok("Test response from mock LLM".to_string())
        }
    }

    #[tokio::test]
    async fn test_llm_oneshot_direct_call() {
        println!("Testing llm_oneshot tool...");

        // Create a tool registry
        let mut registry = ToolRegistry::new();
        registry.register(LlmCallTool);

        // Verify the tool is registered
        assert!(registry.has_tool("llm_oneshot"), "llm_oneshot tool should be registered");
        println!("✓ Tool registered successfully");

        // Create LLM clients map with proper trait object
        let mut clients = HashMap::new();
        clients.insert(ModelColor::GrnModel, Arc::new(MockLlmClient) as Arc<dyn LlmClient>);

        // Create tool context with the mock client
        let policy_manager = apchat_policy::PolicyManager::new();
        let context = ToolContext::new(
            PathBuf::from("."),
            "test_session".to_string(),
            policy_manager,
        ).with_llm_clients(clients);

        // Create tool parameters
        let mut params = ToolParameters::new();
        params.set("model_color", "grn");
        params.set("instruction", "Hello! Can you respond with a simple test message?");

        // Execute the tool
        println!("Executing llm_oneshot tool...");
        let result = registry.execute_tool("llm_oneshot", params, &context).await;

        // Check the result
        assert!(result.success, "Tool execution should succeed");
        println!("✓ Tool executed successfully!");
        println!("Response: {}", result.content);
        
        // Verify the response content
        assert_eq!(result.content, "Test response from mock LLM");
        println!("✓ Test completed successfully!");
    }

    #[tokio::test]
    async fn test_llm_oneshot_with_file() {
        println!("Testing llm_oneshot tool with file parameter...");

        // Create a test file
        let test_file_path = "./test_file.txt";
        std::fs::write(test_file_path, "This is test file content").unwrap();

        // Create a tool registry
        let mut registry = ToolRegistry::new();
        registry.register(LlmCallTool);

        // Create LLM clients map with proper trait object
        let mut clients = HashMap::new();
        clients.insert(ModelColor::GrnModel, Arc::new(MockLlmClient) as Arc<dyn LlmClient>);

        // Create tool context with the mock client
        let policy_manager = apchat_policy::PolicyManager::new();
        let context = ToolContext::new(
            PathBuf::from("."),
            "test_session".to_string(),
            policy_manager,
        ).with_llm_clients(clients);

        // Create tool parameters with file_path
        let mut params = ToolParameters::new();
        params.set("model_color", "grn");
        params.set("instruction", "Hello! Can you respond with a simple test message?");
        params.set("file_path", test_file_path);

        // Execute the tool
        println!("Executing llm_oneshot tool with file...");
        let result = registry.execute_tool("llm_oneshot", params, &context).await;

        // Check the result
        assert!(result.success, "Tool execution should succeed with file");
        println!("✓ Tool executed successfully with file!");
        println!("Response: {}", result.content);
        
        // Clean up test file
        std::fs::remove_file(test_file_path).unwrap();
        println!("✓ File test completed successfully!");
    }
}
