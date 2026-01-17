// Test for QueryMemoryTool

use apchat_toolcore::{Tool, ToolParameters, ToolContext};
use apchat_policy::PolicyManager;
use apchat_tools::{QueryMemoryTool, StoreMemoryTool};
use std::path::PathBuf;

#[tokio::test]
async fn test_query_memory_tool_parameters() {
    let tool = QueryMemoryTool;
    
    // Test tool name
    assert_eq!(tool.name(), "query_memory");
    
    // Test description
    let desc = tool.description();
    assert!(!desc.is_empty());
    assert!(desc.contains("Search"));
    assert!(desc.contains("memories"));
    
    // Test parameters
    let params = tool.parameters();
    assert!(params.contains_key("user_id"));
    assert!(params.contains_key("query"));
    assert!(params.contains_key("limit"));
    assert!(params.contains_key("conversation_id"));
    assert!(params.contains_key("after_timestamp"));
    assert!(params.contains_key("before_timestamp"));
    
    // Verify required fields
    assert!(params["user_id"].required);
    assert!(!params["query"].required);
    assert!(!params["conversation_id"].required);
}

#[tokio::test]
async fn test_query_memory_tool_validation() {
    let tool = QueryMemoryTool;
    let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
    
    // Test missing user_id
    let mut params = ToolParameters::new();
    params.set("query", "test");
    
    let result = tool.execute(params.clone(), &context).await;
    assert!(!result.success);
    assert!(result.error.is_some());
    
    // Test empty user_id
    params.set("user_id", "");
    let result = tool.execute(params.clone(), &context).await;
    assert!(!result.success);
    assert!(result.error.is_some());
    
    // Test invalid limit (too low)
    params.set("user_id", "test-user");
    params.set("limit", 0);
    let result = tool.execute(params.clone(), &context).await;
    assert!(!result.success);
    assert!(result.error.is_some());
    
    // Test invalid limit (too high)
    params.set("limit", 1001);
    let result = tool.execute(params.clone(), &context).await;
    assert!(!result.success);
    assert!(result.error.is_some());
    
    // Test empty query when provided
    params.set("limit", 50);
    params.set("query", "");
    let result = tool.execute(params.clone(), &context).await;
    assert!(!result.success);
    assert!(result.error.is_some());
}

#[tokio::test]
async fn test_query_memory_tool_success() {
    let tool = QueryMemoryTool;
    let store_tool = StoreMemoryTool;
    let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
    
    // First, store some test memories
    for i in 1..=5 {
        let mut params = ToolParameters::new();
        params.set("user_id", "test-user-123");
        params.set("conversation_id", "test-conv-456");
        params.set("content", format!("Test memory {} for querying", i));
        
        let result = store_tool.execute(params, &context).await;
        assert!(result.success, "Failed to store test memory {}", i);
    }
    
    // Test basic query without search term
    let mut params = ToolParameters::new();
    params.set("user_id", "test-user-123");
    
    let result = tool.execute(params.clone(), &context).await;
    assert!(result.success);
    assert!(result.error.is_none());
    
    // Parse the response
    let response: serde_json::Value = serde_json::from_str(&result.content).expect("Response should be valid JSON");
    assert!(response["count"].is_number());
    assert!(response["memories"].is_array());
    assert!(response["memories"].as_array().unwrap().len() >= 5);
    
    // Test query with search term
    params.set("query", "memory");
    let result = tool.execute(params.clone(), &context).await;
    assert!(result.success);
    
    let response: serde_json::Value = serde_json::from_str(&result.content).expect("Response should be valid JSON");
    assert!(response["count"].is_number());
    
    // Test with conversation filter
    let mut params2 = ToolParameters::new();
    params2.set("user_id", "test-user-123");
    params2.set("conversation_id", "test-conv-456");
    let result = tool.execute(params2.clone(), &context).await;
    assert!(result.success);
    
    // Test with limit
    params2.set("limit", 3);
    let result = tool.execute(params2.clone(), &context).await;
    assert!(result.success);
    
    let response: serde_json::Value = serde_json::from_str(&result.content).expect("Response should be valid JSON");
    assert_eq!(response["count"], 3);
}
