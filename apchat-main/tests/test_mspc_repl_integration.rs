// Test to verify MSPC-based input handling in REPL
use apchat::mspc::{MspcChannel, MspcMessage};
use apchat_models::ModelColor;

#[tokio::test]
async fn test_mspc_message_handling() {
    // Create an MSPC channel
    let channel = MspcChannel::new(10);
    
    // Send a test message
    channel.send(MspcMessage::UserInput("Hello from MSPC!".to_string(), Some("test".to_string()))).await.unwrap();
    
    // Try to receive it non-blockingly
    match channel.try_recv().await {
        Ok(Some(MspcMessage::UserInput(content, sender))) => {
            assert_eq!(content, "Hello from MSPC!");
            assert_eq!(sender, Some("test".to_string()));
            println!("✓ MSPC message received correctly");
        }
        _ => panic!("Failed to receive MSPC message"),
    }
    
    // Test interrupt message
    channel.send(MspcMessage::InterruptSignal("test interrupt".to_string(), Some("test".to_string()))).await.unwrap();
    
    match channel.try_recv().await {
        Ok(Some(msg)) => {
            assert!(channel.is_interrupt(&msg));
            println!("✓ Interrupt message handled correctly");
        }
        _ => panic!("Failed to receive interrupt message"),
    }
    
    // Test command message
    channel.send(MspcMessage::Command("/model blu".to_string(), Some("test".to_string()))).await.unwrap();
    
    match channel.try_recv().await {
        Ok(Some(msg)) => {
            assert!(channel.is_command(&msg));
            println!("✓ Command message handled correctly");
        }
        _ => panic!("Failed to receive command message"),
    }
}

#[tokio::test]
async fn test_channel_empty() {
    // Create an MSPC channel
    let channel = MspcChannel::new(10);
    
    // Try to receive from empty channel
    match channel.try_recv().await {
        Ok(None) => println!("✓ Empty channel handled correctly"),
        Err(_) => println!("✓ Channel error handled correctly"),
        Ok(Some(_)) => panic!("Should not receive message from empty channel"),
    }
}
