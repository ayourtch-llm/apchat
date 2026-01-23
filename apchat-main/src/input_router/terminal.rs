use std::sync::Arc;
use std::io::{self, Write};
use crate::mspc::{MspcChannel, MspcMessage};
use apchat_vty::{print_heart_yellow, print_heart_red};
use tokio::sync::mpsc as tokio_mpsc;

pub struct TerminalInputRouter {
    pub channel: Arc<MspcChannel>,
    pub sender_id: String,
    /// Optional receiver for signals (e.g., confirmation requests) from the main loop
    pub signal_receiver: Option<tokio_mpsc::Receiver<MspcMessage>>,
}

impl TerminalInputRouter {
    pub fn new(channel: Arc<MspcChannel>) -> Self {
        Self {
            channel,
            sender_id: "terminal".to_string(),
            signal_receiver: None,
        }
    }

    /// Set the signal receiver for this router
    pub fn with_signal_receiver(mut self, receiver: tokio_mpsc::Receiver<MspcMessage>) -> Self {
        self.signal_receiver = Some(receiver);
        self
    }
    
    /// Parse input string into appropriate MSPC message type
    /// - "yes"/"no" (case-insensitive) becomes ConfirmationResponse
    /// - Input starting with "!" becomes InterruptSignal
    /// - Input starting with "/" becomes Command
    /// - All other input becomes UserInput
    pub fn parse_input(&self, input: &str) -> MspcMessage {
        let trimmed = input.trim().to_lowercase();

        // Check for confirmation responses first
        if trimmed == "yes" || trimmed == "y" {
            return MspcMessage::ConfirmationResponse(true, Some(self.sender_id.clone()));
        } else if trimmed == "no" || trimmed == "n" {
            return MspcMessage::ConfirmationResponse(false, Some(self.sender_id.clone()));
        }

        let trimmed = input.trim(); // Use original case for other messages

        if trimmed.starts_with('!') {
            // Remove the "!" prefix and return as interrupt signal
            MspcMessage::InterruptSignal(trimmed[1..].to_string(), Some(self.sender_id.clone()))
        } else if trimmed.starts_with('/') {
            // Return as command
            MspcMessage::Command(trimmed.to_string(), Some(self.sender_id.clone()))
        } else {
            // Regular user input
            MspcMessage::UserInput(trimmed.to_string(), Some(self.sender_id.clone()))
        }
    }
    
    /// Send a message to the MSPC channel
    pub async fn send_to_channel(&self, message: MspcMessage) -> Result<(), tokio::sync::mpsc::error::SendError<MspcMessage>> {
        self.channel.send(message).await
    }
    
    /// Handle confirmation prompts by reading from stdin
    /// Returns true if user confirms, false otherwise
    pub fn handle_confirmation_prompt(&self, prompt: &str) -> bool {
        loop {
            print_heart_red(&format!("{}", prompt), false);
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

            print_heart_red(&format!("Please enter 'y' or 'n'"), true);
        }
    }

    /// Take the signal receiver, leaving None in its place
    /// This is used when passing the receiver to readline
    pub fn take_signal_receiver(&mut self) -> Option<tokio_mpsc::Receiver<MspcMessage>> {
        self.signal_receiver.take()
    }
}
