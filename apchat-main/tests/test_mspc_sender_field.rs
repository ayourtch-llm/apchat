#![cfg(test)]
use std::sync::Arc;
use apchat::mspc::{MspcChannel, MspcMessage};
use apchat::input_router::TerminalInputRouter;

#[tokio::test]
async fn test_mspc_sender_field() {
    println!("\n=== Testing MSPC Sender Field ===\n");
    
    // Create channel
    let channel = Arc::new(MspcChannel::new(100));
    let router = TerminalInputRouter::new(channel.clone());
    
    // Test that messages can be created with sender information
    println!("1. Testing message creation with sender...");
    let msg = MspcMessage::UserInput(
        "test message".to_string(),
        Some("terminal".to_string())
    );
    println!("   ✓ Message created with sender");
    
    // Test sending message with sender
    println!("\n2. Testing message sending with sender...");
    let send_result = channel.send(msg).await;
    assert!(send_result.is_ok());
    println!("   ✓ Message sent successfully");
    
    // Test receiving message with sender
    println!("\n3. Testing message reception with sender...");
    match channel.try_recv().await {
        Ok(Some(MspcMessage::UserInput(content, sender))) => {
            assert_eq!(content, "test message");
            assert_eq!(sender, Some("terminal".to_string()));
            println!("   ✓ Message received with sender: {:?}", sender);
        }
        _ => panic!("Failed to receive message with sender"),
    }
    
    // Test backward compatibility - messages without sender
    println!("\n4. Testing backward compatibility...");
    let msg_no_sender = MspcMessage::UserInput(
        "test without sender".to_string(),
        None
    );
    channel.send(msg_no_sender).await.unwrap();
    
    match channel.try_recv().await {
        Ok(Some(MspcMessage::UserInput(content, sender))) => {
            assert_eq!(content, "test without sender");
            assert_eq!(sender, None);
            println!("   ✓ Message without sender handled correctly");
        }
        _ => panic!("Failed to receive message without sender"),
    }
    
    println!("\n=== Sender Field Tests Passed! ===");
}
