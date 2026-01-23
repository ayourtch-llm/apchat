// Confirmation registry for managing pending tool confirmation requests
// This module provides a global registry that maps confirmation IDs to response channels

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};

/// Confirmation response type
pub type ConfirmationResponse = (bool, Option<String>);

/// Global confirmation registry
#[derive(Clone)]
pub struct ConfirmationRegistry {
    inner: Arc<RwLock<HashMap<String, oneshot::Sender<ConfirmationResponse>>>>,
}

impl ConfirmationRegistry {
    /// Create a new confirmation registry
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a pending confirmation and return a unique ID with a receiver
    pub async fn register(&self) -> (String, oneshot::Receiver<ConfirmationResponse>) {
        let id = format!("confirm_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos());

        let (sender, receiver) = oneshot::channel();

        let mut inner = self.inner.write().await;
        inner.insert(id.clone(), sender);

        (id, receiver)
    }

    /// Complete a confirmation by sending the response
    pub async fn complete(&self, id: &str, response: ConfirmationResponse) -> Result<(), String> {
        let mut inner = self.inner.write().await;

        if let Some(sender) = inner.remove(id) {
            sender.send(response)
                .map_err(|_| "Confirmation receiver dropped".to_string())
        } else {
            Err(format!("Unknown confirmation ID: {}", id))
        }
    }

    /// Cancel a confirmation (remove it from the registry without responding)
    pub async fn cancel(&self, id: &str) {
        let mut inner = self.inner.write().await;
        inner.remove(id);
    }

    /// Get the count of pending confirmations
    pub async fn pending_count(&self) -> usize {
        let inner = self.inner.read().await;
        inner.len()
    }
}

impl Default for ConfirmationRegistry {
    fn default() -> Self {
        Self::new()
    }
}
