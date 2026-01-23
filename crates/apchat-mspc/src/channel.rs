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
    /// Tool confirmation request with a unique ID for routing the response
    ToolConfirmationRequest {
        content: String,
        confirmation_id: String,
    },
    /// Tool confirmation response
    ToolConfirmationResponse {
        approved: bool,
        reason: Option<String>,
        confirmation_id: String,
    },
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

/// Error type for message history validation
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum HistoryError {
    #[error("Empty message pair at index {0}")]
    EmptyMessagePair(usize),
    
    #[error("Invalid message pattern at index {0}: both user and agent messages are empty")]
    InvalidMessagePattern(usize),
    
    #[error("Message too long at index {0}: {1} characters (max {2})")]
    MessageTooLong(usize, usize, usize),
    
    #[error("History starts with agent message at index {0}")]
    StartsWithAgentMessage(usize),
    
    #[error("Duplicate consecutive messages from same speaker at index {0}")]
    DuplicateConsecutiveMessages(usize),
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

    /// Get a clone of the sender for this channel
    pub fn sender(&self) -> tokio_mpsc::Sender<MspcMessage> {
        self.sender.clone()
    }

    /// Get a clone of the receiver for this channel
    pub fn receiver(&self) -> Arc<TokioMutex<tokio_mpsc::Receiver<MspcMessage>>> {
        Arc::clone(&self.receiver)
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
        matches!(message, MspcMessage::ConfirmationRequest(_, _) | MspcMessage::ToolConfirmationRequest { .. })
    }

    /// Check if a message is a confirmation response
    pub fn is_confirmation_response(&self, message: &MspcMessage) -> bool {
        matches!(message, MspcMessage::ConfirmationResponse(_, _) | MspcMessage::ToolConfirmationResponse { .. })
    }
    
    /// Validate message history for consistency and correctness
    /// Returns a list of validation errors if any
    pub async fn validate_history(&self) -> Result<Vec<HistoryError>, HistoryError> {
        let history = self.message_history.lock().await;
        let mut errors = Vec::new();
        
        if history.is_empty() {
            return Ok(errors);
        }
        
        // Maximum message length (10,000 characters)
        const MAX_MESSAGE_LENGTH: usize = 10_000;
        
        // Check each message pair
        for (i, pair) in history.iter().enumerate() {
            // Check for empty message pairs
            if pair.user.is_empty() && pair.agent.is_empty() {
                errors.push(HistoryError::EmptyMessagePair(i));
                continue;
            }
            
            // Check message lengths
            if !pair.user.is_empty() && pair.user.len() > MAX_MESSAGE_LENGTH {
                errors.push(HistoryError::MessageTooLong(i, pair.user.len(), MAX_MESSAGE_LENGTH));
            }
            
            if !pair.agent.is_empty() && pair.agent.len() > MAX_MESSAGE_LENGTH {
                errors.push(HistoryError::MessageTooLong(i, pair.agent.len(), MAX_MESSAGE_LENGTH));
            }
            
            // Check for invalid patterns (both empty)
            if pair.user.is_empty() && pair.agent.is_empty() {
                errors.push(HistoryError::InvalidMessagePattern(i));
            }
            
            // Check if history starts with agent message
            if i == 0 && !pair.user.is_empty() && pair.agent.is_empty() {
                // This is valid: starts with user message
            } else if i == 0 && pair.user.is_empty() && !pair.agent.is_empty() {
                errors.push(HistoryError::StartsWithAgentMessage(i));
            }
            
            // Check for duplicate consecutive messages
            if i > 0 {
                let prev = &history[i - 1];
                let current = pair;
                
                // Both previous and current have user messages
                if !prev.user.is_empty() && !current.user.is_empty() {
                    errors.push(HistoryError::DuplicateConsecutiveMessages(i));
                }
                
                // Both previous and current have agent messages
                if !prev.agent.is_empty() && !current.agent.is_empty() {
                    errors.push(HistoryError::DuplicateConsecutiveMessages(i));
                }
            }
        }
        
        if errors.is_empty() {
            Ok(errors)
        } else {
            Err(errors.into_iter().next().unwrap())
        }
    }
    
    /// Validate and repair message history
    /// Attempts to fix common issues automatically
    pub async fn validate_and_repair_history(&self) -> Result<(), Vec<HistoryError>> {
        let mut history = self.message_history.lock().await;
        let mut errors = Vec::new();
        
        if history.is_empty() {
            return Ok(());
        }
        
        // Maximum message length (10,000 characters)
        const MAX_MESSAGE_LENGTH: usize = 10_000;
        
        // First pass: check for issues and truncate messages
        for (i, pair) in history.iter().enumerate() {
            // Check for empty message pairs
            if pair.user.is_empty() && pair.agent.is_empty() {
                errors.push(HistoryError::EmptyMessagePair(i));
                continue;
            }
            
            // Check message lengths
            if !pair.user.is_empty() && pair.user.len() > MAX_MESSAGE_LENGTH {
                errors.push(HistoryError::MessageTooLong(i, pair.user.len(), MAX_MESSAGE_LENGTH));
            }
            
            if !pair.agent.is_empty() && pair.agent.len() > MAX_MESSAGE_LENGTH {
                errors.push(HistoryError::MessageTooLong(i, pair.agent.len(), MAX_MESSAGE_LENGTH));
            }
            
            // Check if history starts with agent message
            if i == 0 && pair.user.is_empty() && !pair.agent.is_empty() {
                errors.push(HistoryError::StartsWithAgentMessage(i));
            }
            
            // Check for duplicate consecutive messages
            if i > 0 {
                let prev = &history[i - 1];
                
                // Both previous and current have user messages
                if !prev.user.is_empty() && !pair.user.is_empty() {
                    errors.push(HistoryError::DuplicateConsecutiveMessages(i));
                }
                
                // Both previous and current have agent messages
                if !prev.agent.is_empty() && !pair.agent.is_empty() {
                    errors.push(HistoryError::DuplicateConsecutiveMessages(i));
                }
            }
        }
        
        // Second pass: repair issues
        let mut prev_user_empty = true;
        let mut prev_agent_empty = true;
        
        for (i, pair) in history.iter_mut().enumerate() {
            // Truncate messages that are too long
            if pair.user.len() > MAX_MESSAGE_LENGTH {
                pair.user.truncate(MAX_MESSAGE_LENGTH);
            }
            
            if pair.agent.len() > MAX_MESSAGE_LENGTH {
                pair.agent.truncate(MAX_MESSAGE_LENGTH);
            }
            
            // Fix orphaned agent messages at the start
            if i == 0 && pair.user.is_empty() && !pair.agent.is_empty() {
                // Move agent message to user field (treat as user message)
                pair.user = std::mem::take(&mut pair.agent);
            }
            
            // Fix duplicate consecutive messages
            if i > 0 {
                // If previous has user message and current also has user message,
                // merge them or move current user to agent
                if !prev_user_empty && !pair.user.is_empty() {
                    // Move current user message to agent field
                    pair.agent = std::mem::take(&mut pair.user);
                }
                
                // If previous has agent message and current also has agent message,
                // merge them or clear current agent
                if !prev_agent_empty && !pair.agent.is_empty() {
                    // Clear current agent message
                    pair.agent.clear();
                }
            }
            
            // Update previous state for next iteration
            prev_user_empty = pair.user.is_empty();
            prev_agent_empty = pair.agent.is_empty();
        }
        
        // Remove any remaining empty pairs
        history.retain(|pair| !(pair.user.is_empty() && pair.agent.is_empty()));
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Handle interruption with validation and repair
    /// Returns the interrupted message content if any
    pub async fn handle_interruption_with_validation(&self) -> Result<String, Vec<HistoryError>> {
        let interrupted = self.handle_interruption().await;
        
        // Validate and repair history after interruption
        let result = self.validate_and_repair_history().await;
        
        if result.is_ok() {
            Ok(interrupted)
        } else {
            Err(result.unwrap_err())
        }
    }
}
