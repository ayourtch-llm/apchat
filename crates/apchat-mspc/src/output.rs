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

/// Text output with optional emoji prefix and newline control
#[derive(Debug, Clone)]
pub struct TextOutput {
    pub emoji: String,
    pub content: String,
    pub newline: bool,
}

impl TextOutput {
    pub fn new(emoji: &str, content: &str, newline: bool) -> Self {
        Self {
            emoji: emoji.to_string(),
            content: content.to_string(),
            newline,
        }
    }
}

/// Enum representing different types of output messages
#[derive(Debug, Clone)]
pub enum OutputMessage {
    UserMessage { sender: String, text: String },
    AssistantResponse(String),
    ToolCall { name: String, args: Value },
    ToolResult(String),
    SystemMessage(String),
    Error(String),
    TextOutput(TextOutput),
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

    #[tokio_test::tokio::test]
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

        let text_output = OutputMessage::TextOutput(TextOutput::new("✓", "Task completed", true));
        assert!(matches!(text_output, OutputMessage::TextOutput(_)));
    }

    #[tokio_test::tokio::test]
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

    #[tokio_test::tokio::test]
    async fn test_text_output_fields() {
        let text_output = TextOutput::new("✓", "Task completed", true);
        assert_eq!(text_output.emoji, "✓");
        assert_eq!(text_output.content, "Task completed");
        assert_eq!(text_output.newline, true);

        let text_output_no_newline = TextOutput::new("⚠", "Warning", false);
        assert_eq!(text_output_no_newline.emoji, "⚠");
        assert_eq!(text_output_no_newline.content, "Warning");
        assert_eq!(text_output_no_newline.newline, false);
    }

    #[tokio_test::tokio::test]
    async fn test_text_output_message_variant() {
        let output_message = OutputMessage::TextOutput(TextOutput::new("📋", "Clipboard content", true));
        
        if let OutputMessage::TextOutput(text) = output_message {
            assert_eq!(text.emoji, "📋");
            assert_eq!(text.content, "Clipboard content");
            assert_eq!(text.newline, true);
        } else {
            panic!("Expected TextOutput variant");
        }
    }
}
