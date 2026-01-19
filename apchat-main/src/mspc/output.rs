use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use std::error::Error as StdError;
use std::fmt;

/// Custom error type for output operations
#[derive(Debug)]
struct OutputError(String);

impl fmt::Display for OutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StdError for OutputError {}

/// Enum representing different types of output messages
#[derive(Debug, Clone)]
pub enum OutputMessage {
    UserMessage { sender: String, text: String },
    AssistantResponse(String),
    ToolCall { name: String, args: Value },
    ToolResult(String),
    SystemMessage(String),
    Error(String),
}

/// Trait for output destinations
#[async_trait]
pub trait OutputDestination: Send + Sync {
    /// Send an output message to this destination
    async fn send_output(&self, message: &OutputMessage) -> Result<(), Box<dyn StdError + Send>>;

    /// Unique identifier for this destination
    fn dest_id(&self) -> String;

    /// Check if the destination is active/connected
    fn is_active(&self) -> bool;
}

/// Broadcast a message to all destinations
pub async fn broadcast_to_all(
    destinations: &[Box<dyn OutputDestination>],
    message: OutputMessage,
) {
    for dest in destinations {
        if dest.is_active() {
            if let Err(e) = dest.send_output(&message).await {
                eprintln!("Failed to send to {}: {}", dest.dest_id(), e);
            }
        }
    }
}

/// Mock output destination for testing
#[derive(Clone)]
struct MockOutputDestination {
    id: String,
    active: bool,
    sent_messages: Arc<tokio::sync::Mutex<Vec<OutputMessage>>>,
}

impl MockOutputDestination {
    fn new(id: &str, active: bool) -> Self {
        Self {
            id: id.to_string(),
            active,
            sent_messages: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl OutputDestination for MockOutputDestination {
    async fn send_output(&self, message: &OutputMessage) -> Result<(), Box<dyn StdError + Send>> {
        if !self.active {
            return Err(Box::new(OutputError("Destination not active".to_string())));
        }
        let mut messages = self.sent_messages.lock().await;
        messages.push(message.clone());
        Ok(())
    }

    fn dest_id(&self) -> String {
        self.id.clone()
    }

    fn is_active(&self) -> bool {
        self.active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_output_message_variants() {
        // Test all OutputMessage variants can be created
        let user_msg = OutputMessage::UserMessage {
            sender: "terminal".to_string(),
            text: "Hello".to_string(),
        };
        assert!(matches!(user_msg, OutputMessage::UserMessage { .. }));

        let assistant_msg = OutputMessage::AssistantResponse("Hi there!".to_string());
        assert!(matches!(assistant_msg, OutputMessage::AssistantResponse(_)));

        let tool_call = OutputMessage::ToolCall {
            name: "list_files".to_string(),
            args: serde_json::json!({"pattern": "*"}),
        };
        assert!(matches!(tool_call, OutputMessage::ToolCall { .. }));

        let tool_result = OutputMessage::ToolResult("Found 10 files".to_string());
        assert!(matches!(tool_result, OutputMessage::ToolResult(_)));

        let system_msg = OutputMessage::SystemMessage("System alert".to_string());
        assert!(matches!(system_msg, OutputMessage::SystemMessage(_)));

        let error_msg = OutputMessage::Error("Something went wrong".to_string());
        assert!(matches!(error_msg, OutputMessage::Error(_)));
    }

    #[tokio::test]
    async fn test_mock_output_destination_active() {
        let dest = MockOutputDestination::new("test", true);
        assert_eq!(dest.dest_id(), "test");
        assert!(dest.is_active());

        let msg = OutputMessage::UserMessage {
            sender: "sender".to_string(),
            text: "test".to_string(),
        };

        let result = dest.send_output(&msg).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_output_destination_inactive() {
        let dest = MockOutputDestination::new("inactive", false);
        assert!(!dest.is_active());

        let msg = OutputMessage::UserMessage {
            sender: "sender".to_string(),
            text: "test".to_string(),
        };

        let result = dest.send_output(&msg).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_broadcast_to_all_active_destinations() {
        let dest1 = MockOutputDestination::new("dest1", true);
        let dest2 = MockOutputDestination::new("dest2", true);
        let dest3 = MockOutputDestination::new("dest3", false);

        // Clone dest3 to keep ownership while also storing in vector
        let dest3_clone = dest3.clone();

        let destinations: Vec<Box<dyn OutputDestination>> = vec![
            Box::new(dest1.clone()),
            Box::new(dest2.clone()),
            Box::new(dest3),
        ];

        let msg = OutputMessage::AssistantResponse("Broadcast message".to_string());
        broadcast_to_all(&destinations, msg).await;

        // Check that active destinations received the message
        let messages1 = dest1.sent_messages.lock().await;
        assert_eq!(messages1.len(), 1);
        
        let messages2 = dest2.sent_messages.lock().await;
        assert_eq!(messages2.len(), 1);
        
        // Inactive destination should not receive message
        let messages3 = dest3_clone.sent_messages.lock().await;
        assert_eq!(messages3.len(), 0);
    }

    #[tokio::test]
    async fn test_broadcast_to_empty_list() {
        let destinations: Vec<Box<dyn OutputDestination>> = vec![];
        let msg = OutputMessage::SystemMessage("Test".to_string());
        broadcast_to_all(&destinations, msg).await;
        // Should not panic
    }

    #[tokio::test]
    async fn test_output_message_clone() {
        let original = OutputMessage::ToolCall {
            name: "test".to_string(),
            args: serde_json::json!({"key": "value"}),
        };
        
        let cloned = original.clone();
        assert_eq!(
            match &original {
                OutputMessage::ToolCall { name, args } => (name.as_str(), args),
                _ => panic!("Wrong variant"),
            },
            match &cloned {
                OutputMessage::ToolCall { name, args } => (name.as_str(), args),
                _ => panic!("Wrong variant"),
            }
        );
    }
}
