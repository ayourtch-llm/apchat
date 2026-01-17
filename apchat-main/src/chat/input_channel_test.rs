[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_input_message_default() {
        let msg = InputMessage::new("test content".to_string());
        
        assert_eq!(msg.content, "test content");
        assert!(!msg.interrupt);
        assert_eq!(msg.priority, MessagePriority::Normal);
        assert_eq!(msg.source, MessageSource::StdIn);
        
        // Timestamp should be recent (within last second)
        let elapsed = msg.timestamp.elapsed().unwrap();
        assert!(elapsed < Duration::from_secs(1));
    }

    #[test]
    fn test_input_message_with_interrupt() {
        let msg = InputMessage::new("interrupt".to_string())
            .with_interrupt(true);
        
        assert!(msg.interrupt);
        assert_eq!(msg.content, "interrupt");
    }

    #[test]
    fn test_input_message_high_priority() {
        let msg = InputMessage::new("high prio".to_string())
            .with_priority(MessagePriority::High);
        
        assert_eq!(msg.priority, MessagePriority::High);
    }

    #[test]
    fn test_input_message_custom_source() {
        let msg = InputMessage::new("file input".to_string())
            .with_source(MessageSource::File("test.txt".to_string()));
        
        match &msg.source {
            MessageSource::File(path) => assert_eq!(path, "test.txt"),
            _ => panic!("Expected File source"),
        }
    }

    #[test]
    fn test_high_priority_interrupt_constructor() {
        let msg = InputMessage::high_priority_interrupt("urgent!".to_string());
        
        assert!(msg.interrupt);
        assert_eq!(msg.priority, MessagePriority::High);
        assert_eq!(msg.content, "urgent!");
    }

    #[test]
    fn test_message_priority_ordering() {
        assert!(MessagePriority::High > MessagePriority::Normal);
        assert!(MessagePriority::Normal < MessagePriority::High);
    }

    #[test]
    fn test_default_impls() {
        let msg: InputMessage = InputMessage::default();
        assert_eq!(msg.content, "");
        assert!(!msg.interrupt);
        assert_eq!(msg.priority, MessagePriority::Normal);
        assert_eq!(msg.source, MessageSource::StdIn);
    }
}
