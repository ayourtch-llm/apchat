use std::sync::mpsc::{Sender, Receiver};

pub enum MspcMessage {
    UserInput(String),
    SystemPrompt(String),
    ConfirmationRequest(String),
    ConfirmationResponse(bool),
    InterruptSignal(String),
    Command(String),
}

#[derive(Debug)]
pub struct ChannelError;

impl std::fmt::Display for ChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Channel error")
    }
}

impl std::error::Error for ChannelError {}
