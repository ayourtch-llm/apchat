// Test for ListScheduledInstructionsTool

use apchat_toolcore::{Tool, ToolParameters, ToolContext};
use apchat_policy::PolicyManager;
use apchat_tools::{AddScheduledInstructionTool, ListScheduledInstructionsTool};
use std::path::PathBuf;
use chrono::Utc;

#[tokio::test]
async fn test_list_scheduled_instructions_tool_parameters() {
    let tool = ListScheduledInstructionsTool;
    
    // Test tool name
    assert_eq!(tool.name(), "list_scheduled_instructions");
    
    // Test description
    let desc = tool.description();
    assert!(!desc.is_empty());
    
    // Test parameters
    let params = tool.parameters();
    assert!(params.contains_key("status"));
    assert!(params.contains_key("limit"));
    assert!(params.contains_key("offset"));
}

#[tokio::test]
async fn test_list_scheduled_instructions_tool_empty() {
    let add_tool = AddScheduledInstructionTool;
    let list_tool = ListScheduledInstructionsTool;
    let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
    
    // First add a scheduled instruction
    let mut params = ToolParameters::new();
    let future_time = Utc::now().timestamp() + 3600;
    params.set("scheduled_time", future_time);
    params.set("content", "Test instruction for listing");
    
    let add_result = add_tool.execute(params, &context).await;
    assert!(add_result.success, "Failed to add scheduled instruction: {:?}", add_result.error);
    
    // Now list all scheduled instructions
    let mut params = ToolParameters::new();
    params.set("limit", 10);
    
    let result = list_tool.execute(params, &context).await;
    assert!(result.success);
    
    // Parse the response
    let response: serde_json::Value = serde_json::from_str(&result.content).expect("Response should be valid JSON");
    assert!(response["total"].is_number());
    assert!(response["instructions"].is_array());
    
    let instructions = response["instructions"].as_array().expect("instructions should be an array");
    assert!(instructions.len() >= 1, "Should have at least 1 instruction");
    
    // Check that the first instruction has expected fields
    let first = &instructions[0];
    assert!(first["id"].is_string());
    assert!(first["scheduled_time"].is_number());
    assert!(first["content"].is_string());
    assert!(first["created_at"].is_number());
    assert!(first["status"].is_string());
}

#[tokio::test]
async fn test_list_scheduled_instructions_tool_filter_by_status() {
    let add_tool = AddScheduledInstructionTool;
    let list_tool = ListScheduledInstructionsTool;
    let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
    
    // Add a scheduled instruction
    let mut params = ToolParameters::new();
    let future_time = Utc::now().timestamp() + 3600;
    params.set("scheduled_time", future_time);
    params.set("content", "Test instruction for filtering");
    
    let add_result = add_tool.execute(params, &context).await;
    assert!(add_result.success, "Failed to add scheduled instruction");
    
    // List only pending instructions
    let mut params = ToolParameters::new();
    params.set("status", "pending");
    
    let result = list_tool.execute(params, &context).await;
    assert!(result.success);
    
    // Parse the response
    let response: serde_json::Value = serde_json::from_str(&result.content).expect("Response should be valid JSON");
    let instructions = &response["instructions"];
    
    // All instructions should have status "pending"
    for instruction in instructions.as_array().unwrap() {
        assert_eq!(instruction["status"], "pending");
    }
}
