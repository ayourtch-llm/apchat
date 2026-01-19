use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::mspc::MspcChannel;

/// InputSourceManager manages multiple input sources (terminal, webex, websockets)
/// and their associated reader tasks.
///
/// This allows coordinated startup, monitoring, and cleanup of all input readers.
pub struct InputSourceManager {
    /// Handle to the terminal input reader task
    pub terminal_reader: Option<JoinHandle<()>>,
    
    /// Handle to the Webex input reader task
    pub webex_reader: Option<JoinHandle<()>>,
    
    /// Handles to websocket input reader tasks, keyed by session ID
    pub websocket_handlers: HashMap<String, JoinHandle<()>>,
}

impl InputSourceManager {
    /// Create a new InputSourceManager with no active readers
    pub fn new() -> Self {
        Self {
            terminal_reader: None,
            webex_reader: None,
            websocket_handlers: HashMap::new(),
        }
    }
    
    /// Clean up all active reader tasks by aborting them.
    /// This should be called when shutting down the application.
    ///
    /// # Behavior
    /// - Aborts all running tasks
    /// - Waits for tasks to complete (or timeout)
    /// - Clears all task handles
    pub async fn cleanup(&mut self) {
        // Clean up terminal reader
        if let Some(handle) = self.terminal_reader.take() {
            handle.abort();
            let _ = handle.await;
        }
        
        // Clean up Webex reader
        if let Some(handle) = self.webex_reader.take() {
            handle.abort();
            let _ = handle.await;
        }
        
        // Clean up all websocket handlers
        for (_, handle) in self.websocket_handlers.drain() {
            handle.abort();
            let _ = handle.await;
        }
    }
    
    /// Check if any readers are currently active
    pub fn has_active_readers(&self) -> bool {
        self.terminal_reader.is_some() 
            || self.webex_reader.is_some() 
            || !self.websocket_handlers.is_empty()
    }
    
    /// Get the number of active readers
    pub fn active_reader_count(&self) -> usize {
        let mut count = 0;
        if self.terminal_reader.is_some() {
            count += 1;
        }
        if self.webex_reader.is_some() {
            count += 1;
        }
        count += self.websocket_handlers.len();
        count
    }
}
