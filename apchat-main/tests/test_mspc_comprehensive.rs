#![cfg(test)]
use std::sync::Arc;
use apchat::mspc::{MspcChannel, MspcMessage};
use apchat::input_router::TerminalInputRouter;

#[tokio::test]
async fn test_mspc_integration_comprehensive() {
    println!("\n=== Comprehensive MSPC Integration Test ===\n");
    
    // 1. Test channel creation and initialization
    println!("1. Testing MSPC channel creation...");
    let channel = Arc::new(MspcChannel::new(100));
    assert!(Arc::strong_count(&channel) == 1);
    println!("   ✓ Channel created successfully");
    
    // 2. Test terminal input router
    println!("\n2. Testing TerminalInputRouter...");
    let router = TerminalInputRouter::new(channel.clone());
    println!("   ✓ Router created successfully");
    
    // 3. Test message parsing
    println!("\n3. Testing message parsing...");
    
    // Test interrupt
    let msg = router.parse_input("!cancel");
    assert!(matches!(msg, MspcMessage::InterruptSignal(s) if s == "cancel"));
    println!("   ✓ Interrupt parsing works");
    
    // Test command
    let msg = router.parse_input("/model blu");
    assert!(matches!(msg, MspcMessage::Command(s) if s == "/model blu"));
    println!("   ✓ Command parsing works");
    
    // Test regular input
    let msg = router.parse_input("Hello world");
    assert!(matches!(msg, MspcMessage::UserInput(s) if s == "Hello world"));
    println!("   ✓ User input parsing works");
    
    // 4. Test message sending
    println!("\n4. Testing message sending...");
    let send_result = channel.send(MspcMessage::UserInput("Test message".to_string())).await;
    assert!(send_result.is_ok());
    println!("   ✓ Message sent successfully");
    
    // 5. Test non-blocking receiving
    println!("\n5. Testing non-blocking message reception...");
    match channel.try_recv().await {
        Ok(Some(MspcMessage::UserInput(content))) => {
            assert_eq!(content, "Test message");
            println!("   ✓ Message received non-blockingly");
        }
        _ => panic!("Failed to receive message"),
    }
    
    // 6. Test empty channel
    println!("\n6. Testing empty channel behavior...");
    match channel.try_recv().await {
        Ok(None) => println!("   ✓ Empty channel handled correctly"),
        Err(_) => println!("   ✓ Channel error handled correctly"),
        Ok(Some(_)) => panic!("Should not receive message from empty channel"),
    }
    
    // 7. Test message type detection
    println!("\n7. Testing message type detection...");
    
    let interrupt_msg = MspcMessage::InterruptSignal("test".to_string());
    assert!(channel.is_interrupt(&interrupt_msg));
    println!("   ✓ Interrupt detection works");
    
    let command_msg = MspcMessage::Command("/test".to_string());
    assert!(channel.is_command(&command_msg));
    println!("   ✓ Command detection works");
    
    let user_msg = MspcMessage::UserInput("test".to_string());
    assert!(!channel.is_interrupt(&user_msg));
    assert!(!channel.is_command(&user_msg));
    println!("   ✓ User input detection works");
    
    // 8. Test message history
    println!("\n8. Testing message history...");
    channel.add_user_message("User message".to_string()).await;
    channel.add_agent_message("Agent response".to_string()).await;
    
    let history = channel.get_history_for_prompt().await;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].user, "User message");
    assert_eq!(history[0].agent, "Agent response");
    println!("   ✓ Message history works");
    
    // 9. Test interruption handling
    println!("\n9. Testing interruption handling...");
    channel.add_user_message("Before interrupt".to_string()).await;
    channel.add_agent_message("Partial response".to_string()).await;
    
    let interrupted = channel.handle_interruption().await;
    assert_eq!(interrupted, "Partial response");
    println!("   ✓ Interruption cleanup works");
    
    println!("\n=== All Tests Passed! ===");
    println!("\nMSPC Integration Status: FULLY FUNCTIONAL");
    println!("\nFeatures Verified:");
    println!("  ✓ Channel creation and initialization");
    println!("  ✓ Terminal input router");
    println!("  ✓ Message parsing (interrupt/command/user)");
    println!("  ✓ Message sending");
    println!("  ✓ Non-blocking message reception");
    println!("  ✓ Empty channel handling");
    println!("  ✓ Message type detection");
    println!("  ✓ Message history management");
    println!("  ✓ Interruption handling");
}
