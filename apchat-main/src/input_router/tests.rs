use std::sync::Arc;

use apchat::mspc::{MspcChannel, MspcMessage};

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    
    #[test]
    fn test_terminal_router_parses_regular_input() {
        let channel = Arc::new(MspcChannel::new(100));
        let router = apchat::input_router::TerminalInputRouter::new(channel.clone());
        
        // Test parsing regular input
        let msg = router.parse_input("Hello world");
        assert!(matches!(msg, MspcMessage::UserInput(s) if s == "Hello world"));
    }
    
    #[test]
    fn test_terminal_router_parses_interrupt() {
        let channel = Arc::new(MspcChannel::new(100));
        let router = apchat::input_router::TerminalInputRouter::new(channel.clone());
        
        // Test parsing interrupt (starts with !)
        let msg = router.parse_input("!stop");
        assert!(matches!(msg, MspcMessage::InterruptSignal(s) if s == "stop"));
        
        let msg = router.parse_input("!cancel");
        assert!(matches!(msg, MspcMessage::InterruptSignal(s) if s == "cancel"));
    }
    
    #[test]
    fn test_terminal_router_parses_command() {
        let channel = Arc::new(MspcChannel::new(100));
        let router = apchat::input_router::TerminalInputRouter::new(channel.clone());
        
        // Test parsing command (starts with /)
        let msg = router.parse_input("/help");
        assert!(matches!(msg, MspcMessage::Command(s) if s == "/help"));
        
        let msg = router.parse_input("/model");
        assert!(matches!(msg, MspcMessage::Command(s) if s == "/model"));
    }
    
    #[test]
    fn test_terminal_router_parses_empty_input() {
        let channel = Arc::new(MspcChannel::new(100));
        let router = apchat::input_router::TerminalInputRouter::new(channel.clone());
        
        // Test parsing empty input
        let msg = router.parse_input("");
        assert!(matches!(msg, MspcMessage::UserInput(s) if s.is_empty()));
    }
    
    #[test]
    fn test_terminal_router_parses_whitespace_input() {
        let channel = Arc::new(MspcChannel::new(100));
        let router = apchat::input_router::TerminalInputRouter::new(channel.clone());
        
        // Test parsing whitespace-only input
        let msg = router.parse_input("   ");
        assert!(matches!(msg, MspcMessage::UserInput(s) if s.trim().is_empty()));
    }
    
    #[tokio::test]
    async fn test_terminal_router_sends_to_channel() {
        let channel = Arc::new(MspcChannel::new(100));
        let router = apchat::input_router::TerminalInputRouter::new(channel.clone());
        
        // Send a message through the router
        router.send_message(MspcMessage::UserInput("test message".to_string())).await.unwrap();
        
        // Receive it from the channel
        let received = channel.recv().await.unwrap();
        assert!(matches!(received, MspcMessage::UserInput(s) if s == "test message"));
    }
    
    #[tokio::test]
    async fn test_terminal_router_handles_confirmation() {
        let channel = Arc::new(MspcChannel::new(100));
        let router = apchat::input_router::TerminalInputRouter::new(channel.clone());
        
        // Test that we can send confirmation messages
        router.send_message(MspcMessage::ConfirmationResponse(true)).await.unwrap();
        
        // Receive the confirmation
        let received = channel.recv().await.unwrap();
        assert!(matches!(received, MspcMessage::ConfirmationResponse(true)));
        
        // Test false confirmation
        router.send_message(MspcMessage::ConfirmationResponse(false)).await.unwrap();
        let received = channel.recv().await.unwrap();
        assert!(matches!(received, MspcMessage::ConfirmationResponse(false)));
    }
    
    #[test]
    fn test_webex_router_stub() {
        let channel = Arc::new(MspcChannel::new(100));
        let router = apchat::input_router::WebexInputRouter::new(channel.clone());
        
        // For now, just verify it can be created
        assert!(router.channel.send(MspcMessage::UserInput("test".to_string())).await.is_ok());
    }
}
