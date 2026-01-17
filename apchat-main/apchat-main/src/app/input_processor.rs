// SPDX-License-Identifier: MIT
// Copyright (c) 2024-2026 APChat Contributors

// Input Processor
// 
// This module implements the input processing layer for APChat, responsible for:
// - Receiving messages from the input channel
// - Validating message structure
// - Handling interruption logic
// - Managing error states
// - Processing commands and user input
//
// The processor follows the MSPC (Multi-Stage Processing Chain) pattern:
// 1. Reception: Receive from channel
// 2. Validation: Check message structure
// 3. Transformation: Parse and normalize
// 4. Routing: Direct to appropriate handler
// 5. Feedback: Provide processing status

use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use rustyline::error::ReadlineError;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, instrument, trace, warn};

use crate::chat::input_channel::{InputChannel, InputChannelConfig};
use apchat_models::{Message, ModelConfig};

/// Input processor configuration
#[derive(Debug, Clone)]
pub struct InputProcessorConfig {
    /// Maximum message length in characters
    pub max_message_length: usize,
    /// Enable interruption handling
    pub enable_interruption: bool,
    /// Enable message validation
    pub enable_validation: bool,
}

impl Default for InputProcessorConfig {
    fn default() -> Self {
        Self {
            max_message_length: 10_000,
            enable_interruption: true,
            enable_validation: true,
        }
    }
}

/// Input processing result
#[derive(Debug, Clone, PartialEq)]
pub enum InputResult {
    /// Valid input ready for processing
    Processed(String),
    /// Interruption command detected
    Interruption(String),
    /// Input validation failed
    ValidationError(String),
    /// Input channel closed
    ChannelClosed,
    /// No input available
    NoInput,
}

/// Input processor error types
#[derive(Debug, thiserror::Error)]
pub enum InputProcessorError {
    #[error("Channel closed unexpectedly")]
    ChannelClosed,
    
    #[error("Message validation failed: {0}")]
    ValidationError(String),
    
    #[error("Message too long: {0} characters (max {1})")]
    MessageTooLong(usize, usize),
    
    #[error("Empty message not allowed")]
    EmptyMessage,
    
    #[error("Readline error: {0}")]
    ReadlineError(#[from] ReadlineError),
}

/// Main input processor struct
pub struct InputProcessor {
    /// Input channel for receiving messages
    input_channel: InputChannel<Result<String, ReadlineError>>,
    /// Configuration options
    config: InputProcessorConfig,
}

impl InputProcessor {
    /// Create a new input processor
    pub fn new(
        config: InputChannelConfig,
        processor_config: InputProcessorConfig,
    ) -> Self {
        let input_channel = InputChannel::new(config);
        
        Self {
            input_channel,
            config: processor_config,
        }
    }
    
    /// Get a clone of the sender for the terminal listener
    pub fn sender(&self) -> mpsc::Sender<Result<String, ReadlineError>> {
        self.input_channel.sender().clone()
    }
    
    /// Check if there are pending messages in the channel
    pub async fn has_pending_messages(&mut self) -> bool {
        self.input_channel.has_pending_messages().await
    }
    
    /// Try to receive a message without blocking
    pub async fn try_recv(&mut self) -> Option<InputResult> {
        self.input_channel.try_recv().await
            .map(|result| self.process_message(result))
            .unwrap_or(Some(InputResult::NoInput))
    }
    
    /// Receive a message with blocking
    pub async fn recv(&mut self) -> Option<InputResult> {
        match self.input_channel.recv().await {
            Some(result) => Some(self.process_message(result)),
            None => Some(InputResult::ChannelClosed),
        }
    }
    
    /// Process a single message from the channel
    fn process_message(&self, result: Result<String, ReadlineError>) -> InputResult {
        match result {
            Ok(line) => {
                let trimmed = line.trim();
                
                // Check for empty message
                if trimmed.is_empty() {
                    return InputResult::ValidationError("Empty message".to_string());
                }
                
                // Check for interruption
                if self.config.enable_interruption && trimmed.starts_with('!') {
                    debug!("Interruption command detected: {}", trimmed);
                    return InputResult::Interruption(trimmed[1..].to_string());
                }
                
                // Validate message length
                if trimmed.len() > self.config.max_message_length {
                    return InputResult::ValidationError(
                        format!("Message exceeds maximum length ({} > {})", 
                            trimmed.len(), 
                            self.config.max_message_length)
                    );
                }
                
                // Process the message
                InputResult::Processed(trimmed.to_string())
            }
            Err(e) => {
                error!("Readline error: {}", e);
                InputResult::ValidationError(format!("Readline error: {}", e))
            }
        }
    }
    
    /// Process input for a specific chat state
    #[instrument(skip(self), fields(model = ?self.current_model))]
    pub async fn process_for_chat(&mut self, model: &ModelConfig) -> Result<InputResult> {
        trace!("Processing input for chat state");
        
        let input_result = self.recv().await;
        
        if let Some(InputResult::Processed(line)) = &input_result {
            debug!("Processing message: '{}'", line);
            
            // Additional validation based on model
            if model.is_some() {
                let model = model.as_ref().unwrap();
                
                // Model-specific validation could go here
                if line.contains('\x00') {
                    return Ok(InputResult::ValidationError("Null bytes not allowed".to_string()));
                }
                
                // Check for model-specific restrictions
                if model.context_window < line.len() / 10 {
                    warn!("Message is very large relative to model context window");
                }
            }
        }
        
        Ok(input_result.unwrap_or(InputResult::NoInput))
    }
    
    /// Create a terminal listener task
    pub async fn spawn_terminal_listener<F>(
        &self,
        rl: rustyline::DefaultEditor,
        prompt: String,
        prompt_updater: F,
    ) -> Result<()>
    where
        F: Fn() -> String + Send + 'static,
    {
        let sender = self.sender();
        
        tokio::spawn(async move {
            Self::terminal_listener_task(rl, sender, prompt, prompt_updater).await
        });
        
        Ok(())
    }
    
    /// Terminal listener task implementation
    async fn terminal_listener_task<F>(
        mut rl: rustyline::DefaultEditor,
        sender: mpsc::Sender<Result<String, ReadlineError>>,
        mut prompt: String,
        mut prompt_updater: F,
    ) -> Result<()>
    where
        F: Fn() -> String,
    {
        debug!("Terminal listener task started");
        
        loop {
            // Update prompt before each readline
            let current_prompt = prompt_updater();
            
            // Read line from terminal
            let line = rl.readline(&current_prompt);
            
            match &line {
                Ok(_) => {
                    trace!("Received input from terminal");
                }
                Err(ReadlineError::Interrupted) => {
                    debug!("Readline interrupted (Ctrl+C)");
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    debug!("Readline EOF (Ctrl+D) - exiting");
                    break;
                }
                Err(e) => {
                    error!("Readline error: {}", e);
                }
            }
            
            // Send result to channel
            if let Err(e) = sender.send(line).await {
                error!("Failed to send message to channel: {}", e);
                break;
            }
            
            // Small delay to prevent busy loop
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        
        debug!("Terminal listener task exited");
        Ok(())
    }
    
    /// Validate a message against model constraints
    pub fn validate_message(&self, message: &str, model: &ModelConfig) -> Result<(), InputProcessorError> {
        if message.is_empty() {
            return Err(InputProcessorError::EmptyMessage);
        }
        
        if message.len() > self.config.max_message_length {
            return Err(InputProcessorError::MessageTooLong(
                message.len(),
                self.config.max_message_length,
            ));
        }
        
        // Model-specific validation
        if message.contains('\x00') {
            return Err(InputProcessorError::ValidationError(
                "Null bytes not allowed in messages".to_string(),
            ));
        }
        
        // Check against context window
        if let Some(model) = model {
            if message.len() > model.context_window * 2 {
                warn!("Message is very large relative to model context window ({} > {}*2)", 
                    message.len(), 
                    model.context_window);
            }
        }
        
        Ok(())
    }
    
    /// Create a message from processed input
    pub fn create_message(&self, input: &str, user_id: &str) -> Message {
        Message::new(user_id, input)
    }
}

#[async_trait]
pub trait InputProcessorTrait: Send + Sync {
    /// Check for pending input
    async fn has_pending_messages(&mut self) -> bool;
    
    /// Try to receive input without blocking
    async fn try_recv(&mut self) -> Option<InputResult>;
    
    /// Receive input with blocking
    async fn recv(&mut self) -> Option<InputResult>;
    
    /// Process input for chat context
    async fn process_for_chat(&mut self, model: &ModelConfig) -> Result<InputResult>;
    
    /// Get the sender for external use
    fn sender(&self) -> mpsc::Sender<Result<String, ReadlineError>>;
    
    /// Spawn terminal listener
    async fn spawn_terminal_listener<F>(
        &self,
        rl: rustyline::DefaultEditor,
        prompt: String,
        prompt_updater: F,
    ) -> Result<()>
    where
        F: Fn() -> String + Send + 'static;
}

#[async_trait]
impl InputProcessorTrait for InputProcessor {
    async fn has_pending_messages(&mut self) -> bool {
        self.has_pending_messages().await
    }
    
    async fn try_recv(&mut self) -> Option<InputResult> {
        self.try_recv().await
    }
    
    async fn recv(&mut self) -> Option<InputResult> {
        self.recv().await
    }
    
    async fn process_for_chat(&mut self, model: &ModelConfig) -> Result<InputResult> {
        self.process_for_chat(model).await
    }
    
    fn sender(&self) -> mpsc::Sender<Result<String, ReadlineError>> {
        self.sender()
    }
    
    async fn spawn_terminal_listener<F>(
        &self,
        rl: rustyline::DefaultEditor,
        prompt: String,
        prompt_updater: F,
    ) -> Result<()>
    where
        F: Fn() -> String + Send + 'static,
    {
        self.spawn_terminal_listener(rl, prompt, prompt_updater).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_input_processor_creation() {
        let input_config = InputChannelConfig::default();
        let processor_config = InputProcessorConfig::default();
        
        let processor = InputProcessor::new(
            input_config,
            processor_config,
        );
        
        assert!(processor.sender().capacity() > 0);
    }
    
    #[test]
    fn test_interruption_detection() {
        let input_config = InputChannelConfig::default();
        let processor_config = InputProcessorConfig::default();
        
        let processor = InputProcessor::new(
            input_config,
            processor_config,
        );
        
        // Test interruption detection
        let result = processor.process_message(Ok("!cancel".to_string()));
        match result {
            InputResult::Interruption(cmd) => assert_eq!(cmd, "cancel"),
            _ => panic!("Expected Interruption result"),
        }
        
        // Test normal message
        let result = processor.process_message(Ok("Hello world".to_string()));
        match result {
            InputResult::Processed(msg) => assert_eq!(msg, "Hello world"),
            _ => panic!("Expected Processed result"),
        }
    }
    
    #[test]
    fn test_message_validation() {
        let input_config = InputChannelConfig::default();
        let mut processor_config = InputProcessorConfig::default();
        processor_config.max_message_length = 10;
        
        let processor = InputProcessor::new(
            input_config,
            processor_config,
        );
        
        // Test empty message
        let result = processor.process_message(Ok("".to_string()));
        assert!(matches!(result, InputResult::ValidationError(_)));
        
        // Test message too long
        let result = processor.process_message(Ok("This is a very long message".to_string()));
        assert!(matches!(result, InputResult::ValidationError(_)));
    }
    
    #[test]
    fn test_readline_error_handling() {
        let input_config = InputChannelConfig::default();
        let processor_config = InputProcessorConfig::default();
        
        let processor = InputProcessor::new(
            input_config,
            processor_config,
        );
        
        let result = processor.process_message(Err(ReadlineError::Eof));
        assert!(matches!(result, InputResult::ValidationError(_)));
    }
}
