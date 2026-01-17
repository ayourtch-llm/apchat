// Test for StoreMemoryTool

use apchat_toolcore::{Tool, ToolParameters, ToolContext};
use apchat_policy::PolicyManager;
use apchat_tools::StoreMemoryTool;
use std::path::PathBuf;

#[tokio::test]
async fn test_store_memory_tool_parameters() {
    let tool = StoreMemoryTool;
    
    // Test tool name
    assert_eq!(tool.name(), "store_memory");
    
    // Test description
    let desc = tool.description();
    assert!(!desc.is_empty());
    
    // Test parameters
    let params = tool.parameters();
    assert!(params.contains_key("user_id"));
    assert!(params.contains_key("conversation_id"));
    assert!(params.contains_key("content"));
    assert!(params.contains_key("metadata"));
}

#[tokio::test]
async fn test_store_memory_tool_validation() {
    let tool = StoreMemoryTool;
    let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
    
    // Test missing user_id
    let mut params = ToolParameters::new();
    params.set("conversation_id", "conv-123");
    params.set("content", "Test memory");
    
    let result = tool.execute(params.clone(), &context).await;
    assert!(!result.success);
    assert!(result.error.is_some());
    
    // Test empty user_id
    params.set("user_id", "");
    let result = tool.execute(params.clone(), &context).await;
    assert!(!result.success);
    assert!(result.error.is_some());
}

#[tokio::test]
async fn test_store_memory_tool_success() {
    let tool = StoreMemoryTool;
    let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
    
    // Test successful storage
    let mut params = ToolParameters::new();
    params.set("user_id", "test-user-123");
    params.set("conversation_id", "test-conv-456");
    params.set("content", "This is a test memory for the StoreMemoryTool");
    params.set("metadata", r#"{"source": "test", "priority": "high"}"#);
    
    let result = tool.execute(params, &context).await;
    assert!(result.success);
    assert!(result.error.is_none());
    
    // Parse the response to verify it contains expected fields
    let response: serde_json::Value = serde_json::from_str(&result.content).expect("Response should be valid JSON");
    assert_eq!(response["message"], "Memory stored successfully");
    assert!(response["memory_id"].is_string());
    assert!(response["timestamp"].is_number());
    assert_eq!(response["user_id"], "test-user-123");
    assert_eq!(response["conversation_id"], "test-conv-456");
}
