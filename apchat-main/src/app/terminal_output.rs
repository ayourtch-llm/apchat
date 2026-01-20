// Terminal output destination for MSPC
use std::error::Error as StdError;
use std::fmt;
use async_trait::async_trait;
use colored::Colorize;
use crate::mspc::OutputMessage;
use crate::mspc::OutputDestination;
use super::vty_output::{print_heart_yellow, print_heart_red};

/// Custom error type for terminal output
#[derive(Debug)]
struct TerminalOutputError(String);

impl fmt::Display for TerminalOutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StdError for TerminalOutputError {}

/// Terminal output destination for MSPC
#[derive(Clone)]
pub struct TerminalOutputDestination {
    active: bool,
}

impl TerminalOutputDestination {
    /// Create a new terminal output destination
    pub fn new() -> Self {
        Self {
            active: true,
        }
    }
    
    /// Set active state
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }
}

#[async_trait]
impl OutputDestination for TerminalOutputDestination {
    async fn send_output(&self, message: &OutputMessage) -> Result<(), Box<dyn StdError + Send>> {
        if !self.active {
            return Err(Box::new(TerminalOutputError("Terminal output not active".to_string())));
        }
        
        match message {
            OutputMessage::UserMessage { sender, text } => {
                let sender_prefix = if sender == "terminal" {
                    "".to_string()
                } else {
                    format!("[{}] ", sender)
                };
                print_heart_red(&format!("{} {}{}", "You:".bright_green().bold(), sender_prefix, text), false);
            }
            OutputMessage::AssistantResponse(text) => {
                print_heart_red(&format!("{} {}", "Assistant:".bright_blue().bold(), text), true);
            }
            OutputMessage::ToolCall { name, args } => {
                print_heart_red(&format!("{} {} {}", "🔧".bright_yellow(), name, args), true);
            }
            OutputMessage::ToolResult(text) => {
                print_heart_red(&format!("{} {}", "✅ Tool result:".bright_green(), text), true);
            }
            OutputMessage::SystemMessage(text) => {
                print_heart_red(&format!("{} {}", "ℹ️".bright_black(), text), true);
            }
            OutputMessage::Error(text) => {
                print_heart_yellow(&format!("{} {}", "❌ Error:".bright_red(), text), true);
            }
        }
        
        Ok(())
    }
    
    fn dest_id(&self) -> String {
        "terminal".to_string()
    }
    
    fn is_active(&self) -> bool {
        self.active
    }
}