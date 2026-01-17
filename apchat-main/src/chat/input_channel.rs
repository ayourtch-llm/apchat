use tokio::sync::mpsc;
use std::time::Duration;

/// Priority level for input messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    /// Normal priority (default)
    Normal,
    /// High priority - should be processed before normal messages
    High,
}

impl Default for MessagePriority {
    fn default() -> Self {
        MessagePriority::Normal
    }
}

/// Source of an input message
#[derive(Debug, Clone, PartialEq)]
pub enum MessageSource {
    /// Message from standard input (terminal)
    StdIn,
    /// Message from a file input
    File(String),
    /// Message from a pipe
    Pipe,
    /// Message from an API/webhook
    Api,
    /// Message from an internal system component
    Internal(String),
    /// Message from a custom source
    Custom(String),
}

impl Default for MessageSource {
    fn default() -> Self {
        MessageSource::StdIn
    }
}

/// Message received from an input channel
#[derive(Debug, Clone)]
pub struct InputMessage {
    pub content: String,
    pub timestamp: std::time::SystemTime,
    
    /// Whether this message should interrupt the current operation
    /// Defaults to false for backward compatibility
    pub interrupt: bool,
    
    /// Priority level of the message
    /// Defaults to Normal for backward compatibility
    pub priority: MessagePriority,
    
    /// Source of the message
    /// Defaults to StdIn for backward compatibility
    pub source: MessageSource,
}

impl InputMessage {
    /// Validate that content is non-empty and within length limits
    pub fn validate_content(&self) -> Result<(), String> {
        if self.content.is_empty() {
            return Err("Content cannot be empty".to_string());
        }

        const MAX_CONTENT_LENGTH: usize = 10_000; // 10KB limit for content
        if self.content.len() > MAX_CONTENT_LENGTH {
            return Err(format!(
                "Content exceeds maximum length of {} characters",
                MAX_CONTENT_LENGTH
            ));
        }

        Ok(())
    }

    /// Validate that timestamp is not in the future
    pub fn validate_timestamp(&self) -> Result<(), String> {
        let now = std::time::SystemTime::now();
        // If timestamp is in the future, return error
        // Note: SystemTime comparison doesn't directly tell us if it's in the future,
        // but we can check if now < timestamp
        match now.duration_since(self.timestamp) {
            Ok(_) => Ok(()), // timestamp is in the past or now
            Err(_) => Err("Timestamp cannot be in the future".to_string()),
        }
    }

    /// Validate interrupt flag based on priority
    pub fn validate_interrupt_flag(&self) -> Result<(), String> {
        // Only high priority messages can be interrupts
        if self.interrupt && self.priority != MessagePriority::High {
            return Err(
                "Only messages with High priority can be interrupts".to_string(),
            );
        }

        Ok(())
    }

    /// Comprehensive validation of all fields
    pub fn validate(&self) -> Result<(), String> {
        self.validate_content()?;
        self.validate_timestamp()?;
        self.validate_interrupt_flag()?;
        Ok(())
    }
    
    /// Create a new InputMessage with default values (backward compatible)
    pub fn new(content: String) -> Self {
        Self {
            content,
            timestamp: std::time::SystemTime::now(),
            interrupt: false,
            priority: MessagePriority::Normal,
            source: MessageSource::StdIn,
        }
    }
    
    /// Create a new InputMessage with custom fields
    pub fn with_interrupt(mut self, interrupt: bool) -> Self {
        self.interrupt = interrupt;
        self
    }
    
    /// Set priority level
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }
    
    /// Set message source
    pub fn with_source(mut self, source: MessageSource) -> Self {
        self.source = source;
        self
    }
    
    /// Create a high-priority interrupt message
    pub fn high_priority_interrupt(content: String) -> Self {
        Self {
            content,
            timestamp: std::time::SystemTime::now(),
            interrupt: true,
            priority: MessagePriority::High,
            source: MessageSource::StdIn,
        }
    }
}
impl Default for InputMessage {
    fn default() -> Self {
        Self::new(String::new())
    }
}

/// Configuration for an input channel
#[derive(Debug, Clone)]
pub struct InputChannelConfig {
    pub buffer_size: usize,
}

/// Asynchronous input channel for receiving messages
pub struct InputChannel<T> {
    receiver: mpsc::Receiver<T>,
    sender: mpsc::Sender<T>,
}

impl<T> InputChannel<T> {
    /// Create a new input channel with the given configuration
    pub fn new(config: InputChannelConfig) -> Self {
        let (sender, receiver) = mpsc::channel(config.buffer_size);
        Self {
            receiver,
            sender,
        }
    }

    /// Get a clone of the sender for this channel
    pub fn sender(&self) -> mpsc::Sender<T> {
        self.sender.clone()
    }

    /// Check if there are pending messages in the channel
    pub fn has_pending_messages(&self) -> bool {
        // Note: mpsc channel doesn't have a direct way to check for pending messages
        // This checks if the channel is still open
        !self.receiver.is_closed()
    }

    /// Try to receive a message without blocking
    pub async fn try_recv(&mut self) -> Option<T> {
        match self.receiver.try_recv() {
            Ok(msg) => Some(msg),
            Err(_) => None,
        }
    }

    /// Receive a message with a timeout
    pub async fn recv_with_timeout(&mut self, timeout: Duration) -> Option<T> {
        match tokio::time::timeout(timeout, self.receiver.recv()).await {
            Ok(recv_result) => recv_result,
            Err(_) => None,
        }
    }

    /// Get the raw receiver (for advanced use cases)
    pub fn into_receiver(self) -> mpsc::Receiver<T> {
        self.receiver
    }
}

impl Default for InputChannelConfig {
    fn default() -> Self {
        Self {
            buffer_size: 100,
        }
    }
}
