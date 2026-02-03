// Output Destination Types
//
// Concrete implementations of the OutputDestination trait for various output targets

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::io::Write;
use async_trait::async_trait;
use serde_json::Value;
use std::error::Error as StdError;

use super::{OutputDestination, OutputMessage, TextOutput};
use apchat_mspc::MspcMessage;
use tokio::sync::broadcast;
use tokio::sync::Mutex;

/// Custom error type for output operations
#[derive(Debug)]
pub struct DestinationError(pub String);

impl std::fmt::Display for DestinationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StdError for DestinationError {}

/// Destination for Readline integration
/// Receives TextOutput messages through a broadcast channel and emits MspcMessage::EmojiText
#[derive(Clone)]
pub struct ReadlineDestination {
    id: String,
    active: Arc<AtomicBool>,
    channel_tx: broadcast::Sender<TextOutput>,
}

impl ReadlineDestination {
    /// Create a new Readline destination
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this destination
    ///
    /// # Returns
    ///
    /// A new ReadlineDestination instance
    pub fn new(id: &str) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            id: id.to_string(),
            active: Arc::new(AtomicBool::new(true)),
            channel_tx: tx,
        }
    }

    /// Get a receiver for the broadcast channel
    ///
    /// # Returns
    ///
    /// A broadcast receiver that will receive TextOutput messages
    pub fn subscribe(&self) -> broadcast::Receiver<TextOutput> {
        self.channel_tx.subscribe()
    }

    /// Deactivate this destination
    pub fn deactivate(&self) {
        self.active.store(false, Ordering::Relaxed);
    }

    /// Activate this destination
    pub fn activate(&self) {
        self.active.store(true, Ordering::Relaxed);
    }
}

#[async_trait]
impl OutputDestination for ReadlineDestination {
    async fn send_output(&self, message: &OutputMessage) -> Result<(), Box<dyn StdError + Send>> {
        if !self.active.load(Ordering::Relaxed) {
            return Err(Box::new(DestinationError("Readline destination not active".to_string())));
        }

        match message {
            OutputMessage::TextOutput(ref text) => {
                let _ = self.channel_tx.send(text.clone());
                Ok(())
            }
            OutputMessage::AssistantResponse(text) => {
                let _ = self.channel_tx.send(TextOutput::new("🤖", text, true));
                Ok(())
            }
            OutputMessage::ToolResult(text) => {
                let _ = self.channel_tx.send(TextOutput::new("🔧", text, true));
                Ok(())
            }
            OutputMessage::SystemMessage(text) => {
                let _ = self.channel_tx.send(TextOutput::new("⚙️", text, true));
                Ok(())
            }
            OutputMessage::Error(text) => {
                let _ = self.channel_tx.send(TextOutput::new("❌", text, true));
                Ok(())
            }
            _ => {
                // Ignore other message types for readline
                Ok(())
            }
        }
    }

    fn dest_id(&self) -> String {
        self.id.clone()
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
}

/// Destination for direct terminal output
/// Writes text directly to stdout with emoji prefixes
#[derive(Clone)]
pub struct TerminalDestination {
    id: String,
    active: Arc<AtomicBool>,
}

impl TerminalDestination {
    /// Create a new terminal destination
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this destination
    ///
    /// # Returns
    ///
    /// A new TerminalDestination instance
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            active: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Deactivate this destination
    pub fn deactivate(&self) {
        self.active.store(false, Ordering::Relaxed);
    }

    /// Activate this destination
    pub fn activate(&self) {
        self.active.store(true, Ordering::Relaxed);
    }
}

#[async_trait]
impl OutputDestination for TerminalDestination {
    async fn send_output(&self, message: &OutputMessage) -> Result<(), Box<dyn StdError + Send>> {
        if !self.active.load(Ordering::Relaxed) {
            return Err(Box::new(DestinationError("Terminal destination not active".to_string())));
        }

        let _ = || -> Result<(), std::io::Error> {
            let mut stdout = std::io::stdout().lock();

            match message {
                OutputMessage::TextOutput(ref text) => {
                    if !text.emoji.is_empty() && !text.content.is_empty() {
                        if text.newline {
                            writeln!(stdout, "{} {}", text.emoji, text.content)?;
                        } else {
                            write!(stdout, "{}", text.content)?;
                        }
                    } else if !text.content.is_empty() {
                        if text.newline {
                            writeln!(stdout, "{}", text.content)?;
                        } else {
                            write!(stdout, "{}", text.content)?;
                        }
                    }
                }
                OutputMessage::AssistantResponse(text) => {
                    writeln!(stdout, "🤖 {}", text)?;
                }
                OutputMessage::ToolResult(text) => {
                    writeln!(stdout, "🔧 {}", text)?;
                }
                OutputMessage::SystemMessage(text) => {
                    writeln!(stdout, "⚙️ {}", text)?;
                }
                OutputMessage::Error(text) => {
                    writeln!(stdout, "❌ {}", text)?;
                }
                OutputMessage::ToolCall { name, args } => {
                    writeln!(stdout, "📞 Tool call: {}({})", name, args)?;
                }
                OutputMessage::UserMessage { sender, text } => {
                    writeln!(stdout, "👤 [{}]: {}", sender, text)?;
                }
            }

            stdout.flush()?;
            Ok(())
        }().map_err(|e| Box::new(e) as Box<dyn StdError + Send>);

        Ok(())
    }

    fn dest_id(&self) -> String {
        self.id.clone()
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
}

/// Destination for file output
/// Appends text to a specified file with emoji prefixes
#[derive(Clone)]
pub struct FileDestination {
    id: String,
    active: Arc<AtomicBool>,
    file_path: Arc<Mutex<std::path::PathBuf>>,
}

impl FileDestination {
    /// Create a new file destination
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this destination
    /// * `file_path` - Path to the output file
    ///
    /// # Returns
    ///
    /// A new FileDestination instance
    pub fn new(id: &str, file_path: std::path::PathBuf) -> Self {
        Self {
            id: id.to_string(),
            active: Arc::new(AtomicBool::new(true)),
            file_path: Arc::new(Mutex::new(file_path)),
        }
    }

    /// Deactivate this destination
    pub fn deactivate(&self) {
        self.active.store(false, Ordering::Relaxed);
    }

    /// Activate this destination
    pub fn activate(&self) {
        self.active.store(true, Ordering::Relaxed);
    }
}

#[async_trait]
impl OutputDestination for FileDestination {
    async fn send_output(&self, message: &OutputMessage) -> Result<(), Box<dyn StdError + Send>> {
        if !self.active.load(Ordering::Relaxed) {
            return Err(Box::new(DestinationError("File destination not active".to_string())));
        }

        let path = self.file_path.lock().await.clone();
        let result = || -> Result<(), std::io::Error> {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;

            match message {
                OutputMessage::TextOutput(ref text) => {
                    if !text.emoji.is_empty() && !text.content.is_empty() {
                        if text.newline {
                            writeln!(file, "{} {}", text.emoji, text.content)?;
                        } else {
                            write!(file, "{} {}", text.emoji, text.content)?;
                        }
                    } else if !text.content.is_empty() {
                        if text.newline {
                            writeln!(file, "{}", text.content)?;
                        } else {
                            write!(file, "{}", text.content)?;
                        }
                    }
                }
                OutputMessage::AssistantResponse(text) => {
                    writeln!(file, "🤖 {}", text)?;
                }
                OutputMessage::ToolResult(text) => {
                    writeln!(file, "🔧 {}", text)?;
                }
                OutputMessage::SystemMessage(text) => {
                    writeln!(file, "⚙️ {}", text)?;
                }
                OutputMessage::Error(text) => {
                    writeln!(file, "❌ {}", text)?;
                }
                OutputMessage::ToolCall { name, args } => {
                    writeln!(file, "📞 Tool call: {}({})", name, args)?;
                }
                OutputMessage::UserMessage { sender, text } => {
                    writeln!(file, "👤 [{}]: {}", sender, text)?;
                }
            }

            file.flush()?;
            Ok(())
        }().map_err(|e| Box::new(e) as Box<dyn StdError + Send>);

        result
    }

    fn dest_id(&self) -> String {
        self.id.clone()
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_readline_destination_send_text() {
        let dest = ReadlineDestination::new("test_readline");
        let mut rx = dest.subscribe();

        let text = TextOutput::new("✓", "Test message", true);
        let msg = OutputMessage::TextOutput(text.clone());

        let result = dest.send_output(&msg).await;
        assert!(result.is_ok());

        // Should receive the message
        let received = rx.recv().await;
        assert!(received.is_ok());
        let received_text = received.unwrap();
        assert_eq!(received_text.emoji, "✓");
        assert_eq!(received_text.content, "Test message");
    }

    #[tokio::test]
    async fn test_readline_destination_inactive() {
        let dest = ReadlineDestination::new("test_inactive");
        dest.deactivate();

        let text = TextOutput::new("✓", "Test message", true);
        let msg = OutputMessage::TextOutput(text);

        let result = dest.send_output(&msg).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_readline_destination_send_assistant_response() {
        let dest = ReadlineDestination::new("test_assistant");
        let mut rx = dest.subscribe();

        let msg = OutputMessage::AssistantResponse("Hello from assistant".to_string());

        let result = dest.send_output(&msg).await;
        assert!(result.is_ok());

        let received = rx.recv().await;
        assert!(received.is_ok());
        let received_text = received.unwrap();
        assert_eq!(received_text.emoji, "🤖");
        assert_eq!(received_text.content, "Hello from assistant");
    }

    #[tokio::test]
    async fn test_readline_destination_send_error() {
        let dest = ReadlineDestination::new("test_error");
        let mut rx = dest.subscribe();

        let msg = OutputMessage::Error("Something went wrong".to_string());

        let result = dest.send_output(&msg).await;
        assert!(result.is_ok());

        let received = rx.recv().await;
        assert!(received.is_ok());
        let received_text = received.unwrap();
        assert_eq!(received_text.emoji, "❌");
    }

    #[tokio::test]
    async fn test_terminal_destination_send_text() {
        let dest = TerminalDestination::new("test_terminal");

        let text = TextOutput::new("✓", "Test message", true);
        let msg = OutputMessage::TextOutput(text);

        let result = dest.send_output(&msg).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_terminal_destination_inactive() {
        let dest = TerminalDestination::new("test_inactive");
        dest.deactivate();

        let text = TextOutput::new("✓", "Test message", true);
        let msg = OutputMessage::TextOutput(text);

        let result = dest.send_output(&msg).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_file_destination_send_text() {
        // This test creates a temporary file
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_output_136.txt");

        // Clean up any existing file
        let _ = std::fs::remove_file(&file_path);

        let dest = FileDestination::new("test_file", file_path.clone());

        let text = TextOutput::new("✓", "Test message", true);
        let msg = OutputMessage::TextOutput(text);

        let result = dest.send_output(&msg).await;
        assert!(result.is_ok());

        // Verify file was created and contains the message
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("✓ Test message"));

        // Clean up
        let _ = std::fs::remove_file(&file_path);
    }

    #[tokio::test]
    async fn test_file_destination_send_assistant_response() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_output_136_assistant.txt");

        let _ = std::fs::remove_file(&file_path);

        let dest = FileDestination::new("test_assistant", file_path.clone());

        let msg = OutputMessage::AssistantResponse("Hello from assistant".to_string());

        let result = dest.send_output(&msg).await;
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("🤖"));

        let _ = std::fs::remove_file(&file_path);
    }

    #[tokio::test]
    async fn test_file_destination_inactive() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_inactive.txt");

        let dest = FileDestination::new("test_inactive", file_path);
        dest.deactivate();

        let text = TextOutput::new("✓", "Test message", true);
        let msg = OutputMessage::TextOutput(text);

        let result = dest.send_output(&msg).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_all_destinations_unique_ids() {
        let readline = ReadlineDestination::new("readline_test");
        let terminal = TerminalDestination::new("terminal_test");
        let file = FileDestination::new("file_test", std::path::PathBuf::from("/tmp/test.txt"));

        assert_ne!(readline.dest_id(), terminal.dest_id());
        assert_ne!(terminal.dest_id(), file.dest_id());
        assert_ne!(readline.dest_id(), file.dest_id());

        assert_eq!(readline.dest_id(), "readline_test");
        assert_eq!(terminal.dest_id(), "terminal_test");
        assert_eq!(file.dest_id(), "file_test");
    }
}
