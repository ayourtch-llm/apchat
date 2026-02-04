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

This issue has been implemented as part of the OutputRouter integration:

- OutputDestination trait implemented in `apchat-main/src/mspc/output.rs`
- Destination types (ReadlineDestination, TerminalDestination, FileDestination) implemented in `apchat-main/src/mspc/destinations.rs`
- EmojiText handling added to readline poll loop in `crates/apchat-vty/src/readline.rs`
- print_with_emoji updated to send to TEXT_OUTPUT_TX in `crates/apchat-vty/src/lib.rs`
- OutputRouter initialized in `apchat-main/src/mspc/mod.rs` with `initialize_output_router()` function
- All println/eprintln replaced with print_heart_red/print_heart_yellow in terminal manager, repl, input router, and router
- All println in apchat-todo replaced with print_heart_red
- Unit tests added and passing for all destination types

Changes committed in commit fea2393.
Implemented the `OutputRouter` component in `apchat-main/src/mspc/router.rs`. The implementation:
- Creates an OutputRouter struct with `destinations: Arc<RwLock<Vec<Arc<dyn OutputDestination>>>>`
- Provides `new()` and `Default` implementation
- `start_monitoring()` subscribes to `TEXT_OUTPUT_TX` and spawns a tokio task to broadcast messages to all registered destinations
- `register()` and `unregister()` for managing destinations
- `active_count()` for filtering and counting only active destinations
- Router is exported in `apchat-main/src/mspc/mod.rs`

Commit: `4a28fe2`

---
*Created: 2026-02-03*
*Resolved: 2026-02-03*
