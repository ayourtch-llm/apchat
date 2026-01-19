use std::sync::Arc;

use crate::mspc::{MspcChannel, MspcMessage};

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    
    #[test]
    fn test_terminal_router_parses_regular_input() {
        let channel = Arc::new(MspcChannel::new());
        let router = crate::input_router::TerminalInputRouter::new(channel.clone());
        
        // Test parsing regular input
        let msg = router.parse_input("Hello world");
        assert!(matches!(msg, MspcMessage::UserInput(s, sender) if s == "Hello world" && sender == Some("terminal".to_string())));
    }
    
    #[test]
    fn test_terminal_router_parses_interrupt() {
        let channel = Arc::new(MspcChannel::new());
        let router = crate::input_router::TerminalInputRouter::new(channel.clone());
        
        // Test parsing interrupt (starts with !)
        let msg = router.parse_input("!stop");
        assert!(matches!(msg, MspcMessage::InterruptSignal(s, sender) if s == "stop" && sender == Some("terminal".to_string())));
        
        let msg = router.parse_input("!cancel");
        assert!(matches!(msg, MspcMessage::InterruptSignal(s, sender) if s == "cancel" && sender == Some("terminal".to_string())));
    }
    
    #[test]
    fn test_terminal_router_parses_command() {
        let channel = Arc::new(MspcChannel::new());
        let router = crate::input_router::TerminalInputRouter::new(channel.clone());
        
        // Test parsing command (starts with /)
        let msg = router.parse_input("/help");
        assert!(matches!(msg, MspcMessage::Command(s, sender) if s == "/help" && sender == Some("terminal".to_string())));
        
        let msg = router.parse_input("/model");
        assert!(matches!(msg, MspcMessage::Command(s, sender) if s == "/model" && sender == Some("terminal".to_string())));
    }
    
    #[test]
    fn test_terminal_router_parses_empty_input() {
        let channel = Arc::new(MspcChannel::new());
        let router = crate::input_router::TerminalInputRouter::new(channel.clone());
        
        // Test parsing empty input
        let msg = router.parse_input("");
        assert!(matches!(msg, MspcMessage::UserInput(s, sender) if s.is_empty() && sender == Some("terminal".to_string())));
    }
    
    #[test]
    fn test_terminal_router_parses_whitespace_input() {
        let channel = Arc::new(MspcChannel::new());
        let router = crate::input_router::TerminalInputRouter::new(channel.clone());
        
        // Test parsing whitespace-only input
        let msg = router.parse_input("   ");
        assert!(matches!(msg, MspcMessage::UserInput(s, sender) if s.trim().is_empty() && sender == Some("terminal".to_string())));
    }
    
    #[tokio::test]
    async fn test_terminal_router_sends_to_channel() {
        let channel = Arc::new(MspcChannel::new());
        let router = crate::input_router::TerminalInputRouter::new(channel.clone());
        
        // Send a message through the router
        router.send_to_channel(MspcMessage::UserInput("test message".to_string(), Some("terminal".to_string())));
        
        // Receive it from the channel
        let received = channel.recv().unwrap();
        assert!(matches!(received, MspcMessage::UserInput(s, sender) if s == "test message" && sender == Some("terminal".to_string())));
    }
    
    #[tokio::test]
    async fn test_terminal_router_handles_confirmation() {
        let channel = Arc::new(MspcChannel::new());
        let router = crate::input_router::TerminalInputRouter::new(channel.clone());
        
        // Test that we can send confirmation messages
        router.send_to_channel(MspcMessage::ConfirmationResponse(true, Some("terminal".to_string())));
        
        // Receive the confirmation
        let received = channel.recv().unwrap();
        assert!(matches!(received, MspcMessage::ConfirmationResponse(true, sender) if sender == Some("terminal".to_string())));
        
        // Test false confirmation
        router.send_to_channel(MspcMessage::ConfirmationResponse(false, Some("terminal".to_string())));
        let received = channel.recv().unwrap();
        assert!(matches!(received, MspcMessage::ConfirmationResponse(false, sender) if sender == Some("terminal".to_string())));
    }
    
    #[test]
    fn test_webex_router_stub() {
        let channel = Arc::new(MspcChannel::new());
        let router = crate::input_router::WebexInputRouter::new(channel.clone());
        
        // For now, just verify it can be created
        assert!(router.channel.send(MspcMessage::UserInput("test".to_string(), Some("webex".to_string()))).is_ok());
    }
}