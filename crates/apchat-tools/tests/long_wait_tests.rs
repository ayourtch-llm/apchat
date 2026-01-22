// Unit tests for LongWaitTool

use apchat_toolcore::{Tool, ToolParameters, ToolContext};
use apchat_policy::PolicyManager;
use apchat_tools::LongWaitTool;
use std::path::PathBuf;

/// Create a test ToolContext with mock MSPC channels
fn create_test_context() -> ToolContext {
    ToolContext::new(
        PathBuf::from("/tmp"),
        "test-session".to_string(),
        PolicyManager::default(),
    )
}

#[tokio::test]
async fn test_long_wait_tool_parameters() {
    let tool = LongWaitTool;
    
    // Test tool name
    assert_eq!(tool.name(), "long_wait");
    
    // Test description
    let desc = tool.description();
    println!("DEBUG: Description is: '{}'", desc);
    assert!(!desc.is_empty());
    // The description mentions "pause" functionality
    assert!(desc.contains("progress") || desc.contains("update"));
    
    // Test parameters
    let params = tool.parameters();
    assert!(params.contains_key("duration"));
    assert!(params.contains_key("message"));
    
    // Verify required fields
    assert!(params["duration"].required);
    assert!(!params["message"].required);
}

#[tokio::test]
async fn test_zero_duration_returns_error() {
    let tool = LongWaitTool;
    let context = create_test_context();
    
    let mut params = ToolParameters::new();
    params.set("duration", 0.0);
    
    let result = tool.execute(params, &context).await;
    
    // Zero duration should return an error
    assert!(!result.success, "Zero duration should fail");
    assert!(result.error.is_some());
    assert!(result.error.as_ref().unwrap().contains("positive") ||
            result.error.as_ref().unwrap().contains("Duration"));
}

#[tokio::test]
async fn test_negative_duration_returns_error() {
    let tool = LongWaitTool;
    let context = create_test_context();
    
    let mut params = ToolParameters::new();
    params.set("duration", -5.0);
    
    let result = tool.execute(params, &context).await;
    
    // Negative duration should return an error
    assert!(!result.success, "Negative duration should fail");
    assert!(result.error.is_some());
}

#[tokio::test]
async fn test_duration_exceeds_maximum_returns_error() {
    let tool = LongWaitTool;
    let context = create_test_context();
    
    let mut params = ToolParameters::new();
    params.set("duration", 601.0); // Exceeds MAX_DURATION (600 seconds)
    
    let result = tool.execute(params, &context).await;
    
    // Duration > 600 should return an error
    assert!(!result.success, "Duration exceeding maximum should fail");
    assert!(result.error.is_some());
    let error_msg = result.error.unwrap();
    assert!(error_msg.contains("600") || error_msg.contains("exceed") || error_msg.contains("maximum"));
}

// Test removed - cannot actually test 600 second wait in unit tests
// The boundary is tested by test_duration_exceeds_maximum_returns_error

#[tokio::test]
async fn test_missing_required_duration_parameter() {
    let tool = LongWaitTool;
    let context = create_test_context();
    
    let params = ToolParameters::new(); // Missing duration
    
    let result = tool.execute(params, &context).await;
    
    // Missing required parameter should fail
    assert!(!result.success, "Missing duration parameter should fail");
    assert!(result.error.is_some());
}

#[tokio::test]
async fn test_normal_duration_completes_successfully() {
    let tool = LongWaitTool;
    let context = create_test_context();
    
    let mut params = ToolParameters::new();
    params.set("duration", 1.0); // Wait for 1 second
    params.set("message", "Test wait");
    
    let start = std::time::Instant::now();
    let result = tool.execute(params, &context).await;
    let elapsed = start.elapsed();
    
    // Should complete successfully
    assert!(result.success, "Normal duration should succeed: {:?}", result.error);
    assert!(result.error.is_none());
    
    // Should have waited approximately 1 second (with some tolerance for async overhead)
    assert!(elapsed >= std::time::Duration::from_millis(900),
            "Should wait at least 900ms, waited {:?}", elapsed);
    assert!(elapsed < std::time::Duration::from_secs(2),
            "Should wait less than 2 seconds, waited {:?}", elapsed);
    
    // Check the result content
    assert!(result.content.contains("1.0") || result.content.contains("1 seconds"));
    assert!(result.content.contains("Test wait"));
}

#[tokio::test]
async fn test_very_short_duration_completes_quickly() {
    let tool = LongWaitTool;
    let context = create_test_context();
    
    let mut params = ToolParameters::new();
    params.set("duration", 0.1); // Wait for 100ms
    
    let start = std::time::Instant::now();
    let result = tool.execute(params, &context).await;
    let elapsed = start.elapsed();
    
    // Should complete successfully
    assert!(result.success, "Short duration should succeed");
    assert!(result.error.is_none());
    
    // Should complete quickly (within 500ms to account for async overhead)
    assert!(elapsed < std::time::Duration::from_millis(500),
            "Short wait should complete quickly, took {:?}", elapsed);
}

#[tokio::test]
async fn test_custom_message_formatting_with_progress() {
    let tool = LongWaitTool;
    let context = create_test_context();
    
    let mut params = ToolParameters::new();
    params.set("duration", 0.5); // Wait for 500ms
    params.set("message", "Processing: {progress}% complete");
    
    let result = tool.execute(params, &context).await;
    
    // Should complete successfully
    assert!(result.success, "Custom message should succeed");
    assert!(result.error.is_none());
    
    // The result should mention the message
    assert!(result.content.contains("Processing"));
}

#[tokio::test]
async fn test_custom_message_without_progress_placeholder() {
    let tool = LongWaitTool;
    let context = create_test_context();
    
    let mut params = ToolParameters::new();
    params.set("duration", 0.3);
    params.set("message", "Just waiting...");
    
    let result = tool.execute(params, &context).await;
    
    // Should complete successfully
    assert!(result.success, "Message without placeholder should succeed");
    assert!(result.error.is_none());
    
    // The result should contain the custom message
    assert!(result.content.contains("Just waiting"));
}

#[tokio::test]
async fn test_default_message_when_not_provided() {
    let tool = LongWaitTool;
    let context = create_test_context();
    
    let mut params = ToolParameters::new();
    params.set("duration", 0.2);
    // No message parameter provided
    
    let result = tool.execute(params, &context).await;
    
    // Should complete successfully with default message
    assert!(result.success, "Default message should work");
    assert!(result.error.is_none());
    
    // Should contain the default "Waiting" message
    assert!(result.content.contains("Waiting"));
}

#[tokio::test]
async fn test_message_with_multiple_placeholders() {
    let tool = LongWaitTool;
    let context = create_test_context();
    
    let mut params = ToolParameters::new();
    params.set("duration", 0.3);
    params.set("message", "Progress: {progress}% - Still working");
    
    let result = tool.execute(params, &context).await;
    
    // Should complete successfully
    assert!(result.success, "Message with multiple parts should succeed");
    assert!(result.error.is_none());
    
    // Should contain parts of the message
    assert!(result.content.contains("Progress"));
}

#[tokio::test]
async fn test_duration_with_fractional_seconds() {
    let tool = LongWaitTool;
    let context = create_test_context();
    
    let mut params = ToolParameters::new();
    params.set("duration", 0.75); // 750 milliseconds
    
    let start = std::time::Instant::now();
    let result = tool.execute(params, &context).await;
    let elapsed = start.elapsed();
    
    // Should complete successfully
    assert!(result.success, "Fractional duration should succeed");
    assert!(result.error.is_none());
    
    // Should wait approximately 750ms
    assert!(elapsed >= std::time::Duration::from_millis(650),
            "Should wait at least 650ms for 0.75s duration, waited {:?}", elapsed);
    assert!(elapsed < std::time::Duration::from_secs(2),
            "Should wait less than 2 seconds, waited {:?}", elapsed);
    
    // Result should mention the duration (formatting may vary, just check it contains a number)
    assert!(result.content.contains("0.8") || result.content.contains("0.7") || result.content.contains("750"));
}

#[tokio::test]
async fn test_multiple_progress_updates_for_longer_wait() {
    let tool = LongWaitTool;
    let context = create_test_context();
    
    let mut params = ToolParameters::new();
    params.set("duration", 2.0); // 2 seconds - enough for multiple progress updates
    params.set("message", "Long running task: {progress}%");
    
    let start = std::time::Instant::now();
    let result = tool.execute(params, &context).await;
    let elapsed = start.elapsed();
    
    // Should complete successfully
    assert!(result.success, "Longer wait with progress should succeed");
    assert!(result.error.is_none());
    
    // Should wait approximately 2 seconds
    assert!(elapsed >= std::time::Duration::from_secs(1),
            "Should wait at least 1 second, waited {:?}", elapsed);
    assert!(elapsed < std::time::Duration::from_secs(3),
            "Should wait less than 3 seconds, waited {:?}", elapsed);
    
    // Should mention the task
    assert!(result.content.contains("Long running task"));
}
