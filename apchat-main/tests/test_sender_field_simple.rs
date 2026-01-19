#![cfg(test)]
use std::sync::Arc;
use apchat::mspc::{MspcChannel, MspcMessage};

#[tokio::test]
async fn test_sender_field_basic() {
    // Create channel
    let channel = Arc::new(MspcChannel::new(10));
    
    // Test sending message with sender
    let msg_with_sender = MspcMessage::UserInput(
        "Hello with sender".to_string(),
        Some("terminal".to_string())
    );
    channel.send(msg_with_sender).await.unwrap();
    
    // Test sending message without sender
    let msg_without_sender = MspcMessage::UserInput(
        "Hello without sender".to_string(),
        None
    );
    channel.send(msg_without_sender).await.unwrap();
    
    // Receive and verify messages
    match channel.recv().await {
        Some(MspcMessage::UserInput(content, sender)) => {
            assert_eq!(content, "Hello with sender");
            assert_eq!(sender, Some("terminal".to_string()));
        }
        _ => panic!("Failed to receive first message"),
    }
    
    match channel.recv().await {
        Some(MspcMessage::UserInput(content, sender)) => {
            assert_eq!(content, "Hello without sender");
            assert_eq!(sender, None);
        }
        _ => panic!("Failed to receive second message"),
    }
    
    println!("✓ Sender field test passed!");
}
