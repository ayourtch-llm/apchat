// Test for DeleteScheduledInstructionTool

use apchat_toolcore::{Tool, ToolParameters, ToolContext};
use apchat_policy::PolicyManager;
use apchat_tools::{AddScheduledInstructionTool, DeleteScheduledInstructionTool, ListScheduledInstructionsTool};
use std::path::PathBuf;
use chrono::Utc;

#[tokio::test]
async fn test_delete_scheduled_instruction_tool_parameters() {
    let tool = DeleteScheduledInstructionTool;
    
    // Test tool name
    assert_eq!(tool.name(), "delete_scheduled_instruction");
    
    // Test description
    let desc = tool.description();
    assert!(!desc.is_empty());
    
    // Test parameters
    let params = tool.parameters();
    assert!(params.contains_key("id"));
}

#[tokio::test]
async fn test_delete_scheduled_instruction_tool_validation() {
    let tool = DeleteScheduledInstructionTool;
    let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
    
    // Test missing id
    let mut params = ToolParameters::new();
    
    let result = tool.execute(params.clone(), &context).await;
    assert!(!result.success);
    assert!(result.error.is_some());
    
    // Test empty id
    let mut params = ToolParameters::new();
    params.set("id", "");
    
    let result = tool.execute(params, &context).await;
    assert!(!result.success);
    assert!(result.error.is_some());
}

#[tokio::test]
async fn test_delete_scheduled_instruction_tool_success() {
    let add_tool = AddScheduledInstructionTool;
    let delete_tool = DeleteScheduledInstructionTool;
    let list_tool = ListScheduledInstructionsTool;
    let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
    
    // First add a scheduled instruction
    let mut params = ToolParameters::new();
    let future_time = Utc::now().timestamp() + 3600;
    params.set("scheduled_time", future_time);
    params.set("content", "Test instruction to delete");
    
    let add_result = add_tool.execute(params, &context).await;
    assert!(add_result.success, "Failed to add scheduled instruction: {:?}", add_result.error);
    
    // Parse the response to get the ID
    let add_response: serde_json::Value = serde_json::from_str(&add_result.content).expect("Response should be valid JSON");
    let instruction_id = add_response["id"].as_str().expect("ID should be a string");
    
    // Verify the instruction exists
    let mut params = ToolParameters::new();
    params.set("limit", 10);
    
    let list_result = list_tool.execute(params, &context).await;
    assert!(list_result.success);
    
    let list_response: serde_json::Value = serde_json::from_str(&list_result.content).expect("Response should be valid JSON");
    let instructions = list_response["instructions"].as_array().unwrap();
    let exists = instructions.iter().any(|i| i["id"] == instruction_id);
    assert!(exists, "Instruction should exist before deletion");
    
    // Delete the instruction
    let mut params = ToolParameters::new();
    params.set("id", instruction_id);
    
    let delete_result = delete_tool.execute(params, &context).await;
    assert!(delete_result.success, "Delete failed: {:?}", delete_result.error);
    
    // Parse the response to verify success
    let delete_response: serde_json::Value = serde_json::from_str(&delete_result.content).expect("Response should be valid JSON");
    assert_eq!(delete_response["message"], "Scheduled instruction deleted successfully");
    
    // Verify the instruction was deleted
    let mut params = ToolParameters::new();
    params.set("limit", 10);
    
    let list_result = list_tool.execute(params, &context).await;
    assert!(list_result.success);
    
    let list_response: serde_json::Value = serde_json::from_str(&list_result.content).expect("Response should be valid JSON");
    let instructions = list_response["instructions"].as_array().unwrap();
    let exists = instructions.iter().any(|i| i["id"] == instruction_id);
    assert!(!exists, "Instruction should not exist after deletion");
}

#[tokio::test]
async fn test_delete_scheduled_instruction_tool_not_found() {
    let delete_tool = DeleteScheduledInstructionTool;
    let context = ToolContext::new(PathBuf::from("/tmp"), "test-session".to_string(), PolicyManager::default());
    
    // Try to delete a non-existent instruction
    let mut params = ToolParameters::new();
    params.set("id", "non-existent-id-12345");
    
    let result = delete_tool.execute(params, &context).await;
    assert!(!result.success);
    assert!(result.error.is_some());
    assert!(result.error.unwrap().contains("not found"));
}
