pub mod channel;
pub mod message;

#[cfg(test)]
mod tests {
    use crate::mspc::{MspcChannel, MspcMessage};
    use crate::mspc::message::ChannelError;

    #[test]
    fn test_channel_creation() {
        let channel = MspcChannel::new();
        assert!(channel.send(MspcMessage::UserInput("test".to_string())).is_ok());
    }

    #[test]
    fn test_send_and_recv() {
        let channel = MspcChannel::new();
        
        channel.send(MspcMessage::UserInput("hello".to_string())).unwrap();
        channel.send(MspcMessage::UserInput("world".to_string())).unwrap();
        
        let msg1 = channel.recv().unwrap();
        let msg2 = channel.recv().unwrap();
        
        match (msg1, msg2) {
            (MspcMessage::UserInput(s1), MspcMessage::UserInput(s2)) => {
                assert_eq!(s1, "hello");
                assert_eq!(s2, "world");
            }
            _ => panic!("Unexpected message types"),
        }
    }

    #[test]
    fn test_try_recv_empty() {
        let channel = MspcChannel::new();
        let result = channel.try_recv().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_try_recv_with_message() {
        let channel = MspcChannel::new();
        
        channel.send(MspcMessage::UserInput("test".to_string())).unwrap();
        
        let result = channel.try_recv().unwrap();
        assert!(result.is_some());
        
        if let Some(MspcMessage::UserInput(s)) = result {
            assert_eq!(s, "test");
        } else {
            panic!("Unexpected message type");
        }
    }

    #[test]
    fn test_message_history_user_message() {
        let mut channel = MspcChannel::new();
        
        channel.add_user_message("hello".to_string());
        
        let history = channel.get_history_for_prompt();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].user, "hello");
        assert_eq!(history[0].agent, "");
    }

    #[test]
    fn test_message_history_agent_message() {
        let mut channel = MspcChannel::new();
        
        channel.add_user_message("hello".to_string());
        channel.add_agent_message("hi there".to_string());
        
        let history = channel.get_history_for_prompt();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].user, "hello");
        assert_eq!(history[0].agent, "hi there");
    }

    #[test]
    fn test_message_history_multiple_pairs() {
        let mut channel = MspcChannel::new();
        
        channel.add_user_message("first".to_string());
        channel.add_agent_message("response 1".to_string());
        
        channel.add_user_message("second".to_string());
        channel.add_agent_message("response 2".to_string());
        
        let history = channel.get_history_for_prompt();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].user, "first");
        assert_eq!(history[0].agent, "response 1");
        assert_eq!(history[1].user, "second");
        assert_eq!(history[1].agent, "response 2");
    }

    #[test]
    fn test_handle_interruption() {
        let mut channel = MspcChannel::new();
        
        channel.add_user_message("hello".to_string());
        channel.add_agent_message("this is a long response".to_string());
        
        channel.handle_interruption();
        
        let history = channel.get_history_for_prompt();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].user, "hello");
        assert_eq!(history[0].agent, "");
    }

    #[test]
    fn test_all_message_variants() {
        let channel = MspcChannel::new();
        
        // Test all message variants can be sent
        channel.send(MspcMessage::UserInput("input".to_string())).unwrap();
        channel.send(MspcMessage::SystemPrompt("prompt".to_string())).unwrap();
        channel.send(MspcMessage::ConfirmationRequest("confirm?".to_string())).unwrap();
        channel.send(MspcMessage::InterruptSignal("stop!".to_string())).unwrap();
        channel.send(MspcMessage::Command("/help".to_string())).unwrap();
        channel.send(MspcMessage::ConfirmationResponse(true)).unwrap();
        
        // Test all can be received in the correct order
        let msg1 = channel.recv().unwrap();
        match msg1 {
            MspcMessage::UserInput(s) => assert_eq!(s, "input"),
            _ => panic!("Expected UserInput"),
        }
        
        let msg2 = channel.recv().unwrap();
        match msg2 {
            MspcMessage::SystemPrompt(s) => assert_eq!(s, "prompt"),
            _ => panic!("Expected SystemPrompt"),
        }
        
        let msg3 = channel.recv().unwrap();
        match msg3 {
            MspcMessage::ConfirmationRequest(s) => assert_eq!(s, "confirm?"),
            _ => panic!("Expected ConfirmationRequest"),
        }
        
        let msg4 = channel.recv().unwrap();
        match msg4 {
            MspcMessage::InterruptSignal(s) => assert_eq!(s, "stop!"),
            _ => panic!("Expected InterruptSignal"),
        }
        
        let msg5 = channel.recv().unwrap();
        match msg5 {
            MspcMessage::Command(s) => assert_eq!(s, "/help"),
            _ => panic!("Expected Command"),
        }
        
        let msg6 = channel.recv().unwrap();
        match msg6 {
            MspcMessage::ConfirmationResponse(b) => assert_eq!(b, true),
            _ => panic!("Expected ConfirmationResponse"),
        }
    }
}

pub use channel::MspcChannel;
pub use message::MspcMessage;
