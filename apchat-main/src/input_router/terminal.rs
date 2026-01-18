use std::sync::Arc;
use std::io::{self, Write};
use crate::mspc::{MspcChannel, MspcMessage};

pub struct TerminalInputRouter {
    pub channel: Arc<MspcChannel>,
}

impl TerminalInputRouter {
    pub fn new(channel: Arc<MspcChannel>) -> Self {
        Self { channel }
    }
    
    /// Parse input string into appropriate MSPC message type
    /// - Input starting with "!" becomes InterruptSignal
    /// - Input starting with "/" becomes Command
    /// - All other input becomes UserInput
    pub fn parse_input(&self, input: &str) -> MspcMessage {
        let trimmed = input.trim();
        
        if trimmed.starts_with('!') {
            // Remove the "!" prefix and return as interrupt signal
            MspcMessage::InterruptSignal(trimmed[1..].to_string())
        } else if trimmed.starts_with('/') {
            // Return as command
            MspcMessage::Command(trimmed.to_string())
        } else {
            // Regular user input
            MspcMessage::UserInput(trimmed.to_string())
        }
    }
    
    /// Send a message to the MSPC channel
    pub fn send_to_channel(&self, message: MspcMessage) {
        let _ = self.channel.send(message);
    }
    
    /// Handle confirmation prompts by reading from stdin
    /// Returns true if user confirms, false otherwise
    pub fn handle_confirmation_prompt(&self, prompt: &str) -> bool {
        loop {
            print!("{}", prompt);
            io::stdout().flush().unwrap();
            
            let mut response = String::new();
            if let Ok(_) = io::stdin().read_line(&mut response) {
                let response = response.trim().to_lowercase();
                
                if response == "y" || response == "yes" {
                    return true;
                } else if response == "n" || response == "no" {
                    return false;
                } else if response.is_empty() {
                    // Default to false for empty response
                    return false;
                }
            }
            
            println!("Please enter 'y' or 'n'");
        }
    }
}
