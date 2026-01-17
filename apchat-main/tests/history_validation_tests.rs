#[cfg(test)]
mod history_validation_tests {
    use apchat::chat::history::{validate_history, fix_interrupted_history, insert_bogus_message};
    use apchat_models::Message;

    #[test]
    fn test_validate_history_exists() {
        // Test that validate_history function exists and is callable
        let valid_messages = vec![
            Message {
                role: "system".to_string(),
                content: "You are a helpful assistant".to_string(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                reasoning: None,
            },
            Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                reasoning: None,
            },
            Message {
                role: "assistant".to_string(),
                content: "Hi there!".to_string(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                reasoning: None,
            },
        ];

        assert!(validate_history(&valid_messages).is_ok());
    }

    #[test]
    fn test_fix_interrupted_history_exists() {
        // Test that fix_interrupted_history function exists and is callable
        let interrupted_messages = vec![
            Message {
                role: "assistant".to_string(),
                content: "First response".to_string(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                reasoning: None,
            },
            Message {
                role: "assistant".to_string(),
                content: "Second response".to_string(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                reasoning: None,
            },
        ];

        let (fixed, changed) = fix_interrupted_history(&interrupted_messages);
        assert!(changed);
        assert_eq!(fixed.len(), 3); // Should insert a recovery message
    }

    #[test]
    fn test_insert_bogus_message_exists() {
        // Test that insert_bogus_message function exists and is callable
        let messages = vec![
            Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                reasoning: None,
            },
        ];

        let modified = insert_bogus_message(&messages, 0, "user", "test bogus");
        assert_eq!(modified.len(), 2);
        assert!(modified[0].content.starts_with("[BOGUS:"));
    }
}
