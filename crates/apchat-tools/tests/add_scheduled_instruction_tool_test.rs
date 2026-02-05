// Test for AddScheduledInstructionTool

use apchat_toolcore::{Tool, ToolParameters, ToolContext};
use apchat_policy::PolicyManager;
use apchat_tools::AddScheduledInstructionTool;
use std::path::PathBuf;
use chrono::Utc;

#[tokio::test]
#[ignore = "Scheduled instructions require --delayed-instructions flag to be enabled"]
async fn test_add_scheduled_instruction_tool_parameters() {
    let tool = AddScheduledInstructionTool;
    
    // Test tool name
    assert_eq!(tool.name(), "add_scheduled_instruction");
    
    // Test description
    let desc = tool.description();
    assert!(!desc.is_empty());
    
    // Test parameters
    let params = tool.parameters();
    assert!(params.contains_key("scheduled_time"));
    assert!(params.contains_key("content"));
}

#[tokio::test]
#[ignore = "Scheduled instructions require --delayed-instructions flag to be enabled"]
async fn test_add_scheduled_instruction_tool_validation() {
    let tool = AddScheduledInstructionTool;
    let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
    
    // Test missing scheduled_time
    let mut params = ToolParameters::new();
    params.set("content", "Test instruction");
    
    let result = tool.execute(params.clone(), &context).await;
    assert!(!result.success);
    assert!(result.error.is_some());
    
    // Test missing content
    let mut params = ToolParameters::new();
    let future_time = Utc::now().timestamp() + 3600;
    params.set("scheduled_time", future_time);
    
    let result = tool.execute(params.clone(), &context).await;
    assert!(!result.success);
    assert!(result.error.is_some());
    
    // Test empty content
    let mut params = ToolParameters::new();
    params.set("scheduled_time", future_time);
    params.set("content", "");
    
    let result = tool.execute(params.clone(), &context).await;
    assert!(!result.success);
    assert!(result.error.is_some());
}

#[tokio::test]
#[ignore = "Scheduled instructions require --delayed-instructions flag to be enabled"]
async fn test_add_scheduled_instruction_tool_future_validation() {
    let tool = AddScheduledInstructionTool;
    let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
    
    // Test past time (should fail)
    let mut params = ToolParameters::new();
    let past_time = Utc::now().timestamp() - 3600;
    params.set("scheduled_time", past_time);
    params.set("content", "This should fail");
    
    let result = tool.execute(params, &context).await;
    assert!(!result.success);
    assert!(result.error.is_some());
    assert!(result.error.unwrap().contains("future"));
}

#[tokio::test]
#[ignore = "Scheduled instructions require --delayed-instructions flag to be enabled"]
async fn test_add_scheduled_instruction_tool_success() {
    let tool = AddScheduledInstructionTool;
    let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
    
    // Test successful scheduling
    let mut params = ToolParameters::new();
    let future_time = Utc::now().timestamp() + 3600;
    params.set("scheduled_time", future_time);
    params.set("content", "This is a test scheduled instruction");
    
    let result = tool.execute(params, &context).await;
    assert!(result.success);
    assert!(result.error.is_none());
    
    // Parse the response to verify it contains expected fields
    let response: serde_json::Value = serde_json::from_str(&result.content).expect("Response should be valid JSON");
    assert_eq!(response["message"], "Scheduled instruction created successfully");
    assert!(response["id"].is_string());
    assert_eq!(response["scheduled_time"], future_time);
    assert_eq!(response["content"], "This is a test scheduled instruction");
    assert!(response["created_at"].is_number());
}
