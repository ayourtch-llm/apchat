// Test file to verify InputMessage changes compile correctly
#[cfg(test)]
mod input_message_tests {
    use apchat::chat::input_channel::*;
    use std::time::Duration;

    #[test]
    fn test_backward_compatibility() {
        // Old code that just creates a new InputMessage with content should still work
        let msg = InputMessage {
            content: "test".to_string(),
            timestamp: std::time::SystemTime::now(),
            interrupt: false,
            priority: MessagePriority::Normal,
            source: MessageSource::StdIn,
        };
        
        assert_eq!(msg.content, "test");
    }

    #[test]
    fn test_new_features() {
        let msg = InputMessage::new("test".to_string())
            .with_interrupt(true)
            .with_priority(MessagePriority::High)
            .with_source(MessageSource::File("test.txt".to_string()));
        
        assert!(msg.interrupt);
        assert_eq!(msg.priority, MessagePriority::High);
        
        match &msg.source {
            MessageSource::File(path) => assert_eq!(path, "test.txt"),
            _ => panic!("Expected File source"),
        }
    }

    #[test]
    fn test_high_priority_interrupt() {
        let msg = InputMessage::high_priority_interrupt("urgent!".to_string());
        assert!(msg.interrupt);
        assert_eq!(msg.priority, MessagePriority::High);
    }

    #[test]
    fn test_defaults() {
        let msg: InputMessage = InputMessage::default();
        assert!(!msg.interrupt);
        assert_eq!(msg.priority, MessagePriority::Normal);
        assert_eq!(msg.source, MessageSource::StdIn);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(MessagePriority::High > MessagePriority::Normal);
        assert!(MessagePriority::Normal < MessagePriority::High);
    }

    #[test]
    fn test_message_source_variants() {
        let sources = vec![
            MessageSource::StdIn,
            MessageSource::File("file.txt".to_string()),
            MessageSource::Pipe,
            MessageSource::Api,
            MessageSource::Internal("test".to_string()),
            MessageSource::Custom("custom".to_string()),
        ];
        
        for source in sources {
            let msg = InputMessage::new("test".to_string()).with_source(source.clone());
            assert_eq!(msg.source, source);
        }
    }
}

    #[test]
    fn test_backward_compatibility() {
        // Old code that just creates a new InputMessage with content should still work
        let msg = InputMessage {
            content: "test".to_string(),
            timestamp: std::time::SystemTime::now(),
            interrupt: false,
            priority: MessagePriority::Normal,
            source: MessageSource::StdIn,
        };
        
        assert_eq!(msg.content, "test");
    }

    #[test]
    fn test_new_features() {
        let msg = InputMessage::new("test".to_string())
            .with_interrupt(true)
            .with_priority(MessagePriority::High)
            .with_source(MessageSource::File("test.txt".to_string()));
        
        assert!(msg.interrupt);
        assert_eq!(msg.priority, MessagePriority::High);
        
        match &msg.source {
            MessageSource::File(path) => assert_eq!(path, "test.txt"),
            _ => panic!("Expected File source"),
        }
    }

    #[test]
    fn test_high_priority_interrupt() {
        let msg = InputMessage::high_priority_interrupt("urgent!".to_string());
        assert!(msg.interrupt);
        assert_eq!(msg.priority, MessagePriority::High);
    }

    #[test]
    fn test_defaults() {
        let msg: InputMessage = InputMessage::default();
        assert!(!msg.interrupt);
        assert_eq!(msg.priority, MessagePriority::Normal);
        assert_eq!(msg.source, MessageSource::StdIn);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(MessagePriority::High > MessagePriority::Normal);
        assert!(MessagePriority::Normal < MessagePriority::High);
    }

    #[test]
    fn test_message_source_variants() {
        let sources = vec![
            MessageSource::StdIn,
            MessageSource::File("file.txt".to_string()),
            MessageSource::Pipe,
            MessageSource::Api,
            MessageSource::Internal("test".to_string()),
            MessageSource::Custom("custom".to_string()),
        ];
        
        for source in sources {
            let msg = InputMessage::new("test".to_string()).with_source(source.clone());
            assert_eq!(msg.source, source);
        }
    }
}
