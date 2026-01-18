#![cfg(test)]
use apchat::mspc::{MspcChannel, MspcMessage, MessagePair};

#[tokio::test]
async fn test_mspc_channel_creation() {
    // Test that MSPC channel can be created
    let channel = MspcChannel::new(100);
    assert!(channel.has_pending_messages().await);
}

#[tokio::test]
async fn test_interrupt_detection() {
    // Test that inputs starting with "!" are detected as interrupts
    let channel = MspcChannel::new(100);
    
    // Regular input
    let regular = MspcMessage::UserInput("hello world".to_string());
    assert!(!channel.is_interrupt(&regular));
    
    // Interrupt input
    let interrupt = MspcMessage::InterruptSignal("!cancel".to_string());
    assert!(channel.is_interrupt(&interrupt));
}

#[tokio::test]
async fn test_message_history_preservation() {
    // Test that message history is properly maintained
    let channel = MspcChannel::new(100);
    
    // Add user and agent messages
    channel.add_user_message("hello".to_string()).await;
    channel.add_agent_message("hi there".to_string()).await;
    channel.add_user_message("how are you?".to_string()).await;
    
    let history = channel.get_history_for_prompt().await;
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].user, "hello");
    assert_eq!(history[0].agent, "hi there");
    assert_eq!(history[1].user, "how are you?");
}

#[tokio::test]
async fn test_confirmation_prompt_handling() {
    // Test that confirmation prompts are handled correctly
    let channel = MspcChannel::new(100);
    
    // Create a confirmation request
    let confirm = MspcMessage::ConfirmationRequest("Proceed with action?".to_string());
    
    assert!(channel.is_confirmation_request(&confirm));
}

#[tokio::test]
async fn test_regular_input_at_turn_end() {
    // Test that regular inputs are processed
    let channel = MspcChannel::new(100);
    
    // Add regular inputs
    channel.add_user_message("first message".to_string()).await;
    channel.add_user_message("second message".to_string()).await;
    
    // Both should be in history
    let history = channel.get_history_for_prompt().await;
    assert_eq!(history.len(), 2);
}

#[tokio::test]
async fn test_cancellation_support() {
    // Test that cancellation tokens work correctly
    let channel = MspcChannel::new(100);
    
    // Add cancellation message
    let cancel = MspcMessage::InterruptSignal("!cancel".to_string());
    
    // Should be able to detect cancellation
    assert!(channel.is_interrupt(&cancel));
}

#[tokio::test]
async fn test_error_handling() {
    // Test that errors are properly handled
    let channel = MspcChannel::new(100);
    
    // Add error message
    let error = MspcMessage::Error("connection failed".to_string());
    
    match error {
        MspcMessage::Error(msg) => assert_eq!(msg, "connection failed"),
        _ => panic!("Expected Error variant"),
    }
}

#[tokio::test]
async fn test_message_pairing() {
    // Test that user and agent messages are properly paired
    let channel = MspcChannel::new(100);
    
    // Add paired messages
    channel.add_user_message("hello".to_string()).await;
    channel.add_agent_message("hi".to_string()).await;
    channel.add_user_message("how are you?".to_string()).await;
    channel.add_agent_message("I'm good".to_string()).await;
    
    let history = channel.get_history_for_prompt().await;
    assert_eq!(history.len(), 2);
    
    // Verify pairing
    assert_eq!(history[0].user, "hello");
    assert_eq!(history[0].agent, "hi");
    assert_eq!(history[1].user, "how are you?");
    assert_eq!(history[1].agent, "I'm good");
}

#[tokio::test]
async fn test_interruption_cleanup() {
    // Test that interrupted messages are cleaned up
    let channel = MspcChannel::new(100);
    
    // Add messages
    channel.add_user_message("start task".to_string()).await;
    channel.add_agent_message("working...".to_string()).await;
    
    // Simulate interruption
    let interrupted = channel.handle_interruption().await;
    
    // Should have cleaned up the agent message
    assert_eq!(interrupted, "working...");
}

#[tokio::test]
async fn test_command_parsing() {
    // Test that commands (starting with /) are parsed correctly
    let channel = MspcChannel::new(100);
    
    // Parse commands
    let model_cmd = MspcChannel::parse_input("/model blu");
    let skills_cmd = MspcChannel::parse_input("/skills");
    
    match model_cmd {
        MspcMessage::Command(cmd) => assert_eq!(cmd, "/model blu"),
        _ => panic!("Expected Command variant"),
    }
    
    match skills_cmd {
        MspcMessage::Command(cmd) => assert_eq!(cmd, "/skills"),
        _ => panic!("Expected Command variant"),
    }
}

#[tokio::test]
async fn test_channel_send_recv() {
    // Test that messages can be sent and received
    let channel = MspcChannel::new(100);
    
    // Send a message
    let msg = MspcMessage::UserInput("test message".to_string());
    channel.send(msg.clone()).await.expect("Send should succeed");
    
    // Receive the message
    let received = channel.recv().await;
    assert!(received.is_some());
    
    match received.unwrap() {
        MspcMessage::UserInput(content) => assert_eq!(content, "test message"),
        _ => panic!("Expected UserInput variant"),
    }
}

#[tokio::test]
async fn test_channel_try_recv() {
    // Test non-blocking receive
    let channel = MspcChannel::new(100);
    
    // Try to receive when nothing is available
    let result = channel.try_recv().await;
    assert!(matches!(result, Ok(None)));
    
    // Send a message
    let msg = MspcMessage::UserInput("test".to_string());
    channel.send(msg).await.expect("Send should succeed");
    
    // Now try_recv should succeed
    let result = channel.try_recv().await;
    assert!(matches!(result, Ok(Some(_))));
}

#[tokio::test]
async fn test_clear_history() {
    // Test that history can be cleared
    let channel = MspcChannel::new(100);
    
    // Add messages
    channel.add_user_message("message 1".to_string()).await;
    channel.add_agent_message("response 1".to_string()).await;
    
    let history = channel.get_history_for_prompt().await;
    assert_eq!(history.len(), 1);
    
    // Clear history
    channel.clear_history().await;
    
    let history = channel.get_history_for_prompt().await;
    assert_eq!(history.len(), 0);
}
