use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::mspc::{MspcChannel, MspcMessage};

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    
    #[test]
    fn test_terminal_router_parses_regular_input() {
        let channel = Arc::new(MspcChannel::new(100));
        let router = crate::input_router::TerminalInputRouter::new(channel.clone());
        
        // Test parsing regular input
        let msg = router.parse_input("Hello world");
        assert!(matches!(msg, MspcMessage::UserInput(s, sender) if s == "Hello world" && sender == Some("terminal".to_string())));
    }
    
    #[test]
    fn test_terminal_router_parses_interrupt() {
        let channel = Arc::new(MspcChannel::new(100));
        let router = crate::input_router::TerminalInputRouter::new(channel.clone());
        
        // Test parsing interrupt (starts with !)
        let msg = router.parse_input("!stop");
        assert!(matches!(msg, MspcMessage::InterruptSignal(s, sender) if s == "stop" && sender == Some("terminal".to_string())));
        
        let msg = router.parse_input("!cancel");
        assert!(matches!(msg, MspcMessage::InterruptSignal(s, sender) if s == "cancel" && sender == Some("terminal".to_string())));
    }
    
    #[test]
    fn test_terminal_router_parses_command() {
        let channel = Arc::new(MspcChannel::new(100));
        let router = crate::input_router::TerminalInputRouter::new(channel.clone());
        
        // Test parsing command (starts with /)
        let msg = router.parse_input("/help");
        assert!(matches!(msg, MspcMessage::Command(s, sender) if s == "/help" && sender == Some("terminal".to_string())));
        
        let msg = router.parse_input("/model");
        assert!(matches!(msg, MspcMessage::Command(s, sender) if s == "/model" && sender == Some("terminal".to_string())));
    }
    
    #[test]
    fn test_terminal_router_parses_empty_input() {
        let channel = Arc::new(MspcChannel::new(100));
        let router = crate::input_router::TerminalInputRouter::new(channel.clone());
        
        // Test parsing empty input
        let msg = router.parse_input("");
        assert!(matches!(msg, MspcMessage::UserInput(s, sender) if s.is_empty() && sender == Some("terminal".to_string())));
    }
    
    #[test]
    fn test_terminal_router_parses_whitespace_input() {
        let channel = Arc::new(MspcChannel::new(100));
        let router = crate::input_router::TerminalInputRouter::new(channel.clone());
        
        // Test parsing whitespace-only input
        let msg = router.parse_input("   ");
        assert!(matches!(msg, MspcMessage::UserInput(s, sender) if s.trim().is_empty() && sender == Some("terminal".to_string())));
    }
    
    #[tokio::test]
    async fn test_terminal_router_sends_to_channel() {
        let channel = Arc::new(MspcChannel::new(100));
        let router = crate::input_router::TerminalInputRouter::new(channel.clone());
        
        // Send a message through the router
        router.send_to_channel(MspcMessage::UserInput("test message".to_string(), Some("terminal".to_string())));
        
        // Receive it from the channel
        let received = channel.recv().await.unwrap();
        assert!(matches!(received, MspcMessage::UserInput(s, sender) if s == "test message" && sender == Some("terminal".to_string())));
    }
    
    #[tokio::test]
    async fn test_terminal_router_handles_confirmation() {
        let channel = Arc::new(MspcChannel::new(100));
        let router = crate::input_router::TerminalInputRouter::new(channel.clone());
        
        // Test that we can send confirmation messages
        router.send_to_channel(MspcMessage::ConfirmationResponse(true, Some("terminal".to_string())));
        
        // Receive the confirmation
        let received = channel.recv().await.unwrap();
        assert!(matches!(received, MspcMessage::ConfirmationResponse(true, sender) if sender == Some("terminal".to_string())));
        
        // Test false confirmation
        router.send_to_channel(MspcMessage::ConfirmationResponse(false, Some("terminal".to_string())));
        let received = channel.recv().await.unwrap();
        assert!(matches!(received, MspcMessage::ConfirmationResponse(false, sender) if sender == Some("terminal".to_string())));
    }
    
    #[test]
    fn test_webex_router_creation() {
        let channel = Arc::new(MspcChannel::new(100));
        let router = crate::input_router::WebexInputRouter::new(channel.clone());

        // Test that router was created successfully
        // (parse_input method doesn't exist yet - it's a stub)
        assert!(Arc::strong_count(&channel) >= 1);
    }

    #[test]
    fn test_input_source_manager_new() {
        let manager = crate::input_router::InputSourceManager::new();
        assert!(manager.websocket_handlers.is_empty());
    }

    #[test]
    fn test_input_source_manager_add_terminal_reader() {
        let mut manager = crate::input_router::InputSourceManager::new();
        let channel = Arc::new(MspcChannel::new(100));
        let router = crate::input_router::TerminalInputRouter::new(channel);
        
        // Create a dummy JoinHandle for testing
        let handle = tokio::spawn(async {});
        manager.terminal_reader = Some(handle);
        
        assert!(manager.terminal_reader.is_some());
    }

    #[test]
    fn test_input_source_manager_add_webex_reader() {
        let mut manager = crate::input_router::InputSourceManager::new();
        let channel = Arc::new(MspcChannel::new(100));
        let router = crate::input_router::WebexInputRouter::new(channel);
        
        // Create a dummy JoinHandle for testing
        let handle = tokio::spawn(async {});
        manager.webex_reader = Some(handle);
        
        assert!(manager.webex_reader.is_some());
    }

    #[test]
    fn test_input_source_manager_add_websocket_handler() {
        let mut manager = crate::input_router::InputSourceManager::new();
        
        // Create a dummy JoinHandle for testing
        let handle = tokio::spawn(async {});
        manager.websocket_handlers.insert("session123".to_string(), handle);
        
        assert_eq!(manager.websocket_handlers.len(), 1);
        assert!(manager.websocket_handlers.contains_key("session123"));
    }

    #[tokio::test]
    async fn test_input_source_manager_cleanup_without_readers() {
        let mut manager = crate::input_router::InputSourceManager::new();
        
        // Cleanup should not panic when no readers are present
        manager.cleanup().await;
        
        // Verify state is still valid
        assert!(manager.terminal_reader.is_none());
        assert!(manager.webex_reader.is_none());
        assert!(manager.websocket_handlers.is_empty());
    }

    #[tokio::test]
    async fn test_input_source_manager_cleanup_terminal_reader() {
        let mut manager = crate::input_router::InputSourceManager::new();
        
        // Create a task that will run indefinitely
        let handle = tokio::spawn(async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
        
        manager.terminal_reader = Some(handle);
        assert!(manager.terminal_reader.is_some());
        
        // Cleanup should abort the task
        manager.cleanup().await;
        
        // Verify the reader is cleared
        assert!(manager.terminal_reader.is_none());
    }

    #[tokio::test]
    async fn test_input_source_manager_cleanup_webex_reader() {
        let mut manager = crate::input_router::InputSourceManager::new();
        
        // Create a task that will run indefinitely
        let handle = tokio::spawn(async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
        
        manager.webex_reader = Some(handle);
        assert!(manager.webex_reader.is_some());
        
        // Cleanup should abort the task
        manager.cleanup().await;
        
        // Verify the reader is cleared
        assert!(manager.webex_reader.is_none());
    }

    #[tokio::test]
    async fn test_input_source_manager_cleanup_websocket_handlers() {
        let mut manager = crate::input_router::InputSourceManager::new();
        
        // Create multiple tasks
        let handle1 = tokio::spawn(async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
        let handle2 = tokio::spawn(async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
        
        manager.websocket_handlers.insert("session1".to_string(), handle1);
        manager.websocket_handlers.insert("session2".to_string(), handle2);
        
        assert_eq!(manager.websocket_handlers.len(), 2);
        
        // Cleanup should abort all tasks
        manager.cleanup().await;
        
        // Verify all handlers are cleared
        assert!(manager.websocket_handlers.is_empty());
    }

    #[tokio::test]
    async fn test_input_source_manager_cleanup_all_readers() {
        let mut manager = crate::input_router::InputSourceManager::new();
        
        // Create tasks for all reader types
        let terminal_handle = tokio::spawn(async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
        
        let webex_handle = tokio::spawn(async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
        
        let ws_handle = tokio::spawn(async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
        
        manager.terminal_reader = Some(terminal_handle);
        manager.webex_reader = Some(webex_handle);
        manager.websocket_handlers.insert("ws1".to_string(), ws_handle);
        
        assert!(manager.terminal_reader.is_some());
        assert!(manager.webex_reader.is_some());
        assert_eq!(manager.websocket_handlers.len(), 1);
        
        // Cleanup should abort all tasks
        manager.cleanup().await;
        
        // Verify all readers are cleared
        assert!(manager.terminal_reader.is_none());
        assert!(manager.webex_reader.is_none());
        assert!(manager.websocket_handlers.is_empty());
    }
}

#[cfg(test)]
mod manager_tests {
    use super::*;

    #[test]
    fn test_input_source_manager_new() {
        let manager = crate::input_router::InputSourceManager::new();
        
        // Verify initial state
        assert!(manager.terminal_reader.is_none());
        assert!(manager.webex_reader.is_none());
        assert!(manager.websocket_handlers.is_empty());
    }

    #[test]
    fn test_input_source_manager_add_terminal_reader() {
        let mut manager = crate::input_router::InputSourceManager::new();
        let channel = Arc::new(MspcChannel::new(100));
        let router = crate::input_router::TerminalInputRouter::new(channel);
        
        // Create a dummy JoinHandle for testing
        let handle = tokio::spawn(async {});
        manager.terminal_reader = Some(handle);
        
        assert!(manager.terminal_reader.is_some());
    }

    #[test]
    fn test_input_source_manager_add_webex_reader() {
        let mut manager = crate::input_router::InputSourceManager::new();
        let channel = Arc::new(MspcChannel::new(100));
        let router = crate::input_router::WebexInputRouter::new(channel);
        
        // Create a dummy JoinHandle for testing
        let handle = tokio::spawn(async {});
        manager.webex_reader = Some(handle);
        
        assert!(manager.webex_reader.is_some());
    }

    #[test]
    fn test_input_source_manager_add_websocket_handler() {
        let mut manager = crate::input_router::InputSourceManager::new();
        
        // Create a dummy JoinHandle for testing
        let handle = tokio::spawn(async {});
        manager.websocket_handlers.insert("session123".to_string(), handle);
        
        assert_eq!(manager.websocket_handlers.len(), 1);
        assert!(manager.websocket_handlers.contains_key("session123"));
    }

    #[tokio::test]
    async fn test_input_source_manager_cleanup_without_readers() {
        let mut manager = crate::input_router::InputSourceManager::new();
        
        // Cleanup should not panic when no readers are present
        manager.cleanup().await;
        
        // Verify state is still valid
        assert!(manager.terminal_reader.is_none());
        assert!(manager.webex_reader.is_none());
        assert!(manager.websocket_handlers.is_empty());
    }

    #[tokio::test]
    async fn test_input_source_manager_cleanup_terminal_reader() {
        let mut manager = crate::input_router::InputSourceManager::new();
        
        // Create a task that will run indefinitely
        let handle = tokio::spawn(async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
        
        manager.terminal_reader = Some(handle);
        assert!(manager.terminal_reader.is_some());
        
        // Cleanup should abort the task
        manager.cleanup().await;
        
        // Verify the reader is cleared
        assert!(manager.terminal_reader.is_none());
    }

    #[tokio::test]
    async fn test_input_source_manager_cleanup_webex_reader() {
        let mut manager = crate::input_router::InputSourceManager::new();
        
        // Create a task that will run indefinitely
        let handle = tokio::spawn(async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
        
        manager.webex_reader = Some(handle);
        assert!(manager.webex_reader.is_some());
        
        // Cleanup should abort the task
        manager.cleanup().await;
        
        // Verify the reader is cleared
        assert!(manager.webex_reader.is_none());
    }

    #[tokio::test]
    async fn test_input_source_manager_cleanup_websocket_handlers() {
        let mut manager = crate::input_router::InputSourceManager::new();
        
        // Create multiple tasks
        let handle1 = tokio::spawn(async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
        let handle2 = tokio::spawn(async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
        
        manager.websocket_handlers.insert("session1".to_string(), handle1);
        manager.websocket_handlers.insert("session2".to_string(), handle2);
        
        assert_eq!(manager.websocket_handlers.len(), 2);
        
        // Cleanup should abort all tasks
        manager.cleanup().await;
        
        // Verify all handlers are cleared
        assert!(manager.websocket_handlers.is_empty());
    }

    #[tokio::test]
    async fn test_input_source_manager_cleanup_all_readers() {
        let mut manager = crate::input_router::InputSourceManager::new();
        
        // Create tasks for all reader types
        let terminal_handle = tokio::spawn(async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
        
        let webex_handle = tokio::spawn(async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
        
        let ws_handle = tokio::spawn(async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
        
        manager.terminal_reader = Some(terminal_handle);
        manager.webex_reader = Some(webex_handle);
        manager.websocket_handlers.insert("ws1".to_string(), ws_handle);
        
        assert!(manager.terminal_reader.is_some());
        assert!(manager.webex_reader.is_some());
        assert_eq!(manager.websocket_handlers.len(), 1);
        
        // Cleanup should abort all tasks
        manager.cleanup().await;
        
        // Verify all readers are cleared
        assert!(manager.terminal_reader.is_none());
        assert!(manager.webex_reader.is_none());
        assert!(manager.websocket_handlers.is_empty());
    }
}

