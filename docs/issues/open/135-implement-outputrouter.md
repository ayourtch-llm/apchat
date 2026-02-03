# Issue 135: Implement OutputRouter

## Summary
Implement the `OutputRouter` component that manages all output destinations and broadcasts `TextOutput` messages to registered destinations.

## Location
- File: `apchat-main/src/mspc/router.rs` (new file)
- Dependencies: `apchat-main/src/mspc/output.rs`, `apchat-main/src/mspc/mod.rs`

## Current Behavior
No central coordinator exists for routing emoji-prefixed text to multiple destinations.

## Expected Behavior
Create `OutputRouter` struct with methods for registering/unregistering destinations and a background monitoring task that broadcasts messages from `TEXT_OUTPUT_TX`.

## Impact
Provides the core routing functionality that coordinates all output destinations.

## Suggested Implementation

Create `apchat-main/src/mspc/router.rs`:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::mspc::output::{TextOutput, OutputMessage, OutputDestination};

pub struct OutputRouter {
    destinations: Arc<RwLock<Vec<Arc<dyn OutputDestination>>>>,
}

impl OutputRouter {
    pub fn new() -> Self {
        Self {
            destinations: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Start background monitoring task that receives from TEXT_OUTPUT_TX
    /// and broadcasts to all registered destinations
    pub fn start_monitoring(&self) {
        let destinations = Arc::clone(&self.destinations);
        let mut rx = super::TEXT_OUTPUT_TX.subscribe();

        tokio::spawn(async move {
            while let Ok(text_output) = rx.recv().await {
                let dests = destinations.read().await;
                for dest in dests.iter() {
                    if dest.is_active() {
                        let msg = OutputMessage::TextOutput {
                            emoji: text_output.emoji.clone(),
                            content: text_output.content.clone(),
                            newline: text_output.newline,
                        };
                        if let Err(e) = dest.send_output(&msg).await {
                            eprintln!("Error sending to destination {}: {}", dest.dest_id(), e);
                        }
                    }
                }
            }
        });
    }

    pub async fn register(&self, dest: Arc<dyn OutputDestination>) {
        let mut dests = self.destinations.write().await;
        dests.push(dest);
    }

    pub async fn unregister(&self, id: String) -> bool {
        let mut dests = self.destinations.write().await;
        let original_len = dests.len();
        dests.retain(|d| d.dest_id() != id);
        dests.len() != original_len
    }

    pub async fn active_count(&self) -> usize {
        let dests = self.destinations.read().await;
        dests.iter().filter(|d| d.is_active()).count()
    }
}
```

## Resolution

---
*Created: 2026-02-03*
