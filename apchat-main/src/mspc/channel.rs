use std::sync::Arc;
use tokio::sync::{mpsc as tokio_mpsc, Mutex as TokioMutex};
use anyhow::Result;

/// Message types for MSPC channel
#[derive(Debug, Clone)]
pub enum MspcMessage {
    UserInput(String, Option<String>),      // Content, Sender
    SystemPrompt(String, Option<String>),   // Content, Sender
    ConfirmationRequest(String, Option<String>), // Content, Sender
    ConfirmationResponse(bool, Option<String>),   // Response, Sender
    InterruptSignal(String, Option<String>),    // Content, Sender
    Command(String, Option<String>),         // Content, Sender
    ToolResult(String, Option<String>),      // Content, Sender
    Error(String, Option<String>),           // Content, Sender
}

/// Pair of user and agent messages for history
#[derive(Debug, Clone)]
pub struct MessagePair {
    pub user: String,
    pub agent: String,
}

/// MSPC Channel for handling multiple input sources
#[derive(Debug, Clone)]
pub struct MspcChannel {
    sender: tokio_mpsc::Sender<MspcMessage>,
    receiver: Arc<TokioMutex<tokio_mpsc::Receiver<MspcMessage>>>,
    message_history: Arc<TokioMutex<Vec<MessagePair>>>,
}

impl MspcChannel {
    /// Create a new MSPC channel with bounded capacity
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = tokio_mpsc::channel::<MspcMessage>(capacity);
        
        Self {
            sender,
            receiver: Arc::new(TokioMutex::new(receiver)),
            message_history: Arc::new(TokioMutex::new(Vec::new())),
        }
    }
    
    /// Send a message through the channel
    pub async fn send(&self, message: MspcMessage) -> Result<(), tokio_mpsc::error::SendError<MspcMessage>> {
        self.sender.send(message).await
    }
    
    /// Try to receive a message non-blockingly
    pub async fn try_recv(&self) -> Result<Option<MspcMessage>, tokio_mpsc::error::TryRecvError> {
        let mut receiver = self.receiver.lock().await;
        receiver.try_recv().map(Some)
    }
    
    /// Receive a message blocking
    pub async fn recv(&self) -> Option<MspcMessage> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await
    }
    
    /// Add a user message to history
    pub async fn add_user_message(&self, content: String) {
        let mut history = self.message_history.lock().await;
        history.push(MessagePair {
            user: content,
            agent: String::new(),
        });
    }
    
    /// Add an agent message to history
    pub async fn add_agent_message(&self, content: String) {
        let mut history = self.message_history.lock().await;
        if let Some(last) = history.last_mut() {
            if last.agent.is_empty() {
                last.agent = content;
            } else {
                // Start a new pair if agent already has content
                history.push(MessagePair {
                    user: String::new(),
                    agent: content,
                });
            }
        } else {
            // No user message, create orphaned agent message
            history.push(MessagePair {
                user: String::new(),
                agent: content,
            });
        }
    }
    
    /// Handle interruption - clean up current agent message
    pub async fn handle_interruption(&self) -> String {
        let mut history = self.message_history.lock().await;
        if let Some(last) = history.last_mut() {
            if last.agent.is_empty() {
                // No agent message to clean up
                return String::new();
            } else {
                // Save the interrupted message
                let interrupted = std::mem::take(&mut last.agent);
                return interrupted;
            }
        }
        String::new()
    }
    
    /// Get message history for LLM prompt
    pub async fn get_history_for_prompt(&self) -> Vec<MessagePair> {
        let history = self.message_history.lock().await;
        history.clone()
    }
    
    /// Clear message history
    pub async fn clear_history(&self) {
        let mut history = self.message_history.lock().await;
        history.clear();
    }
    
    /// Check if there are pending messages
    pub async fn has_pending_messages(&self) -> bool {
        let receiver = self.receiver.lock().await;
        !receiver.is_closed()
    }
    
    /// Parse input to determine message type
    pub fn parse_input(input: &str, sender: Option<&str>) -> MspcMessage {
        if input.starts_with('!') {
            MspcMessage::InterruptSignal(input.to_string(), sender.map(|s| s.to_string()))
        } else if input.starts_with('/') {
            MspcMessage::Command(input.to_string(), sender.map(|s| s.to_string()))
        } else if input.starts_with("confirm:") || input.starts_with("Confirm:") {
            MspcMessage::ConfirmationRequest(input.to_string(), sender.map(|s| s.to_string()))
        } else {
            MspcMessage::UserInput(input.to_string(), sender.map(|s| s.to_string()))
        }
    }
    
    /// Check if a message is an interrupt
    pub fn is_interrupt(&self, message: &MspcMessage) -> bool {
        matches!(message, MspcMessage::InterruptSignal(_, _))
    }
    
    /// Check if a message is a command
    pub fn is_command(&self, message: &MspcMessage) -> bool {
        matches!(message, MspcMessage::Command(_, _))
    }
    
    /// Check if a message is a confirmation request
    pub fn is_confirmation_request(&self, message: &MspcMessage) -> bool {
        matches!(message, MspcMessage::ConfirmationRequest(_, _))
    }
    
    /// Check if a message is a confirmation response
    pub fn is_confirmation_response(&self, message: &MspcMessage) -> bool {
        matches!(message, MspcMessage::ConfirmationResponse(_, _))
    }
}

/// Error type for MSPC operations
#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("Channel send error: {0}")]
    SendError(#[from] tokio_mpsc::error::SendError<MspcMessage>),
    
    #[error("Channel receive error: {0}")]
    RecvError(#[from] tokio_mpsc::error::TryRecvError),
    
    #[error("Channel closed")]
    Closed,
}
