// Test for UpdateMemoryTool

use apchat_toolcore::{Tool, ToolParameters, ToolContext};
use apchat_policy::PolicyManager;
use apchat_memory::tools::UpdateMemoryTool;
use std::path::PathBuf;

#[tokio::test]
async fn test_update_memory_tool_parameters() {
    let tool = UpdateMemoryTool;
    
    // Test tool name
    assert_eq!(tool.name(), "update_memory");
    
    // Test description
    let desc = tool.description();
    assert!(!desc.is_empty());
    assert!(desc.contains("update"));
    
    // Test parameters
    let params = tool.parameters();
    assert!(params.contains_key("memory_id"));
    assert!(params.contains_key("user_id"));
    assert!(params.contains_key("content"));
    assert!(params.contains_key("metadata"));
    
    // Verify required parameters
    assert!(params["memory_id"].required);
    assert!(params["user_id"].required);
    assert!(!params["content"].required);
    assert!(!params["metadata"].required);
    
    // Verify parameter types
    assert_eq!(params["memory_id"].param_type, "string");
    assert_eq!(params["user_id"].param_type, "string");
    assert_eq!(params["content"].param_type, "string");
    assert_eq!(params["metadata"].param_type, "string");
}

#[tokio::test]
#[ignore] // FIXME-TEST: figure out a better way to test memory without littering the main memories
async fn test_update_memory_tool_validation() {
    let tool = UpdateMemoryTool;
    let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
    
    // Test 1: Missing required parameter (memory_id)
    let mut params = ToolParameters::new();
    params.set("user_id", "user123");
    params.set("content", "New content");
    
    let result = tool.execute(params, &context).await;
    assert!(!result.success);
    assert!(result.error.unwrap().contains("memory_id"));
    
    // Test 2: Missing required parameter (user_id)
    let mut params = ToolParameters::new();
    params.set("memory_id", "mem123");
    params.set("content", "New content");
    
    let result = tool.execute(params, &context).await;
    assert!(!result.success);
    assert!(result.error.unwrap().contains("user_id"));
    
    // Test 3: Empty memory_id
    let mut params = ToolParameters::new();
    params.set("memory_id", "");
    params.set("user_id", "user123");
    
    let result = tool.execute(params, &context).await;
    assert!(!result.success);
    assert!(result.error.unwrap().contains("memory_id cannot be empty"));
    
    // Test 4: Empty user_id
    let mut params = ToolParameters::new();
    params.set("memory_id", "mem123");
    params.set("user_id", "");
    
    let result = tool.execute(params, &context).await;
    assert!(!result.success);
    assert!(result.error.unwrap().contains("user_id cannot be empty"));
    
    // Test 5: Empty content when provided
    let mut params = ToolParameters::new();
    params.set("memory_id", "mem123");
    params.set("user_id", "user123");
    params.set("content", "");
    
    let result = tool.execute(params, &context).await;
    assert!(!result.success);
    assert!(result.error.unwrap().contains("content cannot be empty if provided"));
    
    // Test 6: Content too long (> 100,000 characters)
    let mut params = ToolParameters::new();
    params.set("memory_id", "mem123");
    params.set("user_id", "user123");
    params.set("content", "a".repeat(100001));
    
    let result = tool.execute(params, &context).await;
    assert!(!result.success);
    assert!(result.error.unwrap().contains("content cannot exceed 100,000 characters"));
}

#[tokio::test]
async fn test_update_memory_tool_nonexistent_memory() {
    let tool = UpdateMemoryTool;
    let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
    
    // Test with valid parameters but non-existent memory
    let mut params = ToolParameters::new();
    params.set("memory_id", "nonexistent-123");
    params.set("user_id", "user123");
    params.set("content", "Valid content");
    
    let result = tool.execute(params, &context).await;
    // Should fail because memory doesn't exist
    assert!(!result.success);
    assert!(result.error.unwrap().contains("Memory with ID 'nonexistent-123' not found"));
}

#[tokio::test]
#[ignore] // FIXME-TEST - need to figure a better way to test without polluting main memory
async fn test_update_memory_tool_wrong_owner() {
    let tool = UpdateMemoryTool;
    let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
    
    // First, store a memory
    use apchat_memory::StoreMemoryTool;
    let store_tool = StoreMemoryTool;
    let mut store_params = ToolParameters::new();
    store_params.set("user_id", "owner-user");
    store_params.set("conversation_id", "test-conv");
    store_params.set("content", "Original memory");
    
    let store_result = store_tool.execute(store_params, &context).await;
    assert!(store_result.success);
    
    // Parse the response to get the memory_id
    let response: serde_json::Value = serde_json::from_str(&store_result.content).expect("Response should be valid JSON");
    let memory_id = response["memory_id"].as_str().unwrap();
    
    // Try to update the memory with a different user_id
    let mut update_params = ToolParameters::new();
    update_params.set("memory_id", memory_id);
    update_params.set("user_id", "wrong-user");
    update_params.set("content", "Updated content");
    
    let result = tool.execute(update_params, &context).await;
    // Should fail because user doesn't own the memory
    assert!(!result.success);
    assert!(result.error.unwrap().contains("You can only update memories that belong to you"));
}
