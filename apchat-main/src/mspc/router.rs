// OutputRouter - Routes output messages to multiple destinations
// Takes TextOutput from the global broadcast channel and forwards to registered destinations

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};

use apchat_vty::print_heart_yellow;
use super::TEXT_OUTPUT_TX;
use crate::mspc::{OutputDestination, OutputMessage, TextOutput};

/// Router for broadcasting output messages to multiple destinations
pub struct OutputRouter {
    destinations: Arc<RwLock<Vec<Arc<dyn OutputDestination>>>>,
    readline_active: Arc<AtomicBool>,
}

impl OutputRouter {
    /// Create a new OutputRouter with an empty list of destinations
    pub fn new() -> Self {
        Self {
            destinations: Arc::new(RwLock::new(Vec::new())),
            readline_active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start monitoring the TEXT_OUTPUT_TX broadcast channel
    /// Spawns a tokio task that receives TextOutput messages and broadcasts them
    /// to all registered destinations.
    /// If readline is active, TerminalDestination is temporarily deactivated to avoid duplicates.
    pub fn start_monitoring(&self) {
        let destinations = self.destinations.clone();
        let readline_active = self.readline_active.clone();
        let mut rx = TEXT_OUTPUT_TX.subscribe();

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(text_output) => {
                        let destinations_lock = destinations.read().await;
                        for dest in destinations_lock.iter() {
                            let dest_id = dest.dest_id();
                            let is_terminal = dest_id == "terminal";
                            let is_readline = dest_id == "readline";
                            
                            // If readline is active, skip terminal destination to avoid duplicates
                            if is_terminal && readline_active.load(Ordering::Relaxed) {
                                continue;
                            }
                            
                            // If terminal is active and we're sending to readline, skip
                            // (this handles the reverse case - though readline shouldn't get terminal output)
                            if is_readline && !readline_active.load(Ordering::Relaxed) {
                                continue;
                            }
                            
                            if dest.is_active() {
                                let message = OutputMessage::TextOutput(text_output.clone());
                                if let Err(e) = dest.send_output(&message).await {
                                    print_heart_yellow(&format!("OutputRouter: Failed to send to {}: {}", dest_id, e), true);
                                }
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        eprintln!("OutputRouter: receiver lagged behind by {} messages, resuming", n);
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
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

    /// Mark readline as active (for duplicate suppression)
    pub fn set_readline_active(&self, active: bool) {
        self.readline_active.store(active, Ordering::Relaxed);
    }
}

impl Default for OutputRouter {
    fn default() -> Self {
        Self::new()
    }
}