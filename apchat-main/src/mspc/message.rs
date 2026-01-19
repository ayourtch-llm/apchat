use std::sync::mpsc::{Sender, Receiver};

pub enum MspcMessage {
    UserInput(String, Option<String>),      // Content, Sender
    SystemPrompt(String, Option<String>),   // Content, Sender
    ConfirmationRequest(String, Option<String>), // Content, Sender
    ConfirmationResponse(bool, Option<String>),   // Response, Sender
    InterruptSignal(String, Option<String>),    // Content, Sender
    Command(String, Option<String>),         // Content, Sender
}

#[derive(Debug)]
pub struct ChannelError;

impl std::fmt::Display for ChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Channel error")
    }
}

impl std::error::Error for ChannelError {}
