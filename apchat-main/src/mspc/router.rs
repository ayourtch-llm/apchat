// OutputRouter - Routes output messages to multiple destinations
// Takes TextOutput from the global broadcast channel and forwards to registered destinations

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use super::TEXT_OUTPUT_TX;
use crate::mspc::{OutputDestination, OutputMessage, TextOutput};

/// Router for broadcasting output messages to multiple destinations
pub struct OutputRouter {
    destinations: Arc<RwLock<Vec<Arc<dyn OutputDestination>>>>,
}

impl OutputRouter {
    /// Create a new OutputRouter with an empty list of destinations
    pub fn new() -> Self {
        Self {
            destinations: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Start monitoring the TEXT_OUTPUT_TX broadcast channel
    /// Spawns a tokio task that receives TextOutput messages and broadcasts them
    /// to all registered destinations.
    pub fn start_monitoring(&self) {
        let destinations = self.destinations.clone();
        let mut rx = TEXT_OUTPUT_TX.subscribe();

        tokio::spawn(async move {
            while let Ok(text_output) = rx.recv().await {
                let destinations_lock = destinations.read().await;
                for dest in destinations_lock.iter() {
                    if dest.is_active() {
                        let message = OutputMessage::TextOutput(text_output.clone());
                        if let Err(e) = dest.send_output(&message).await {
                            eprintln!("OutputRouter: Failed to send to {}: {}", dest.dest_id(), e);
                        }
                    }
                }
            }
        });
    }

    /// Register a new output destination
    pub async fn register(&self, destination: Arc<dyn OutputDestination>) {
        let mut destinations = self.destinations.write().await;
        destinations.push(destination);
    }

    /// Unregister an output destination by its ID
    pub async fn unregister(&self, dest_id: &str) -> bool {
        let mut destinations = self.destinations.write().await;
        let original_len = destinations.len();
        destinations.retain(|dest| dest.dest_id() != dest_id);
        destinations.len() < original_len
    }

    /// Get the count of active destinations
    pub async fn active_count(&self) -> usize {
        let destinations = self.destinations.read().await;
        destinations.iter().filter(|d| d.is_active()).count()
    }
}

impl Default for OutputRouter {
    fn default() -> Self {
        Self::new()
    }
}