# Issue 136: Implement OutputDestination trait and destination types

## Summary
Implement the `OutputDestination` trait and create concrete destination types: `ReadlineDestination`, `TerminalDestination`, and `FileDestination`.

## Location
- File: `apchat-main/src/mspc/destinations.rs` (new file)
- Dependencies: `apchat-main/src/mspc/output.rs`, `apchat-main/src/mspc/mod.rs`

## Current Behavior
No destination types exist for routing emoji-prefixed text to various outputs.

## Expected Behavior
Create `OutputDestination` trait and implement three destination types that can receive and handle `OutputMessage::EmojiText`.

## Impact
Provides the output destinations that receive emoji-prefixed text from the OutputRouter.

## Suggested Implementation

Create `apchat-main/src/mspc/destinations.rs`:

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::PathBuf;
use std::time::SystemTime;
use tokio::sync::mpsc as tokio_mpsc;
use crate::mspc::output::{OutputMessage, OutputDestination, MspcMessage};
use std::error::Error as StdError;

/// Routes to the existing mpsc channel used by the readline instance
pub struct ReadlineDestination {
    sender: tokio_mpsc::Sender<MspcMessage>,
    active: Arc<AtomicBool>,
}

impl ReadlineDestination {
    pub fn new(sender: tokio_mpsc::Sender<MspcMessage>) -> Self {
        Self {
            sender,
            active: Arc::new(AtomicBool::new(true)),
        }
    }
}

#[async_trait::async_trait]
impl OutputDestination for ReadlineDestination {
    async fn send_output(&self, msg: &OutputMessage) -> Result<(), Box<dyn StdError + Send>> {
        match msg {
            OutputMessage::TextOutput { emoji, content, newline } => {
                let _ = self.sender.send(MspcMessage::EmojiText {
                    emoji: emoji.clone(),
                    content: content.clone(),
                    newline: *newline,
                }).await;
                Ok(())
            }
            _ => Ok(()), // Ignore other message types
        }
    }

    fn dest_id(&self) -> String {
        "readline".to_string()
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
}

/// Direct write to stdout/stderr using existing print functions
pub struct TerminalDestination {
    active: Arc<AtomicBool>,
}

impl TerminalDestination {
    pub fn new() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(true)),
        }
    }
}

#[async_trait::async_trait]
impl OutputDestination for TerminalDestination {
    async fn send_output(&self, msg: &OutputMessage) -> Result<(), Box<dyn StdError + Send>> {
        match msg {
            OutputMessage::TextOutput { emoji, content, newline } => {
                // Use existing print functions from apchat-vty
                let mut writer = std::io::stdout();
                crate::vty::print_with_emoji(emoji, content, *newline, &mut writer);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn dest_id(&self) -> String {
        "terminal".to_string()
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
}

/// Appends formatted output to a log file with timestamps
pub struct FileDestination {
    file_path: PathBuf,
    active: Arc<AtomicBool>,
}

impl FileDestination {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            active: Arc::new(AtomicBool::new(true)),
        }
    }
}

#[async_trait::async_trait]
impl OutputDestination for FileDestination {
    async fn send_output(&self, msg: &OutputMessage) -> Result<(), Box<dyn StdError + Send>> {
        match msg {
            OutputMessage::TextOutput { emoji, content, newline } => {
                let timestamp = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)?
                    .as_secs();
                let line = format!("[{}] {} {}{}",
                    timestamp, emoji, content, if *newline { "\n" } else { "" });

                tokio::fs::write(&self.file_path, line).await?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn dest_id(&self) -> String {
        format!("file:{}", self.file_path.display())
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
}
```

Add to `apchat-main/src/mspc/mod.rs`:
```rust
pub mod destinations;
pub use destinations::{ReadlineDestination, TerminalDestination, FileDestination, OutputDestination};
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

---
*Created: 2026-02-03*
