# Issue 133: Add TextOutput struct and extend OutputMessage

## Summary
Add the `TextOutput` message struct and extend `OutputMessage` enum with an `EmojiText` variant for the OutputRouter system.

## Location
- File: `apchat-main/src/mspc/output.rs` (new file)
- Related: `apchat-main/src/mspc/mod.rs`

## Current Behavior
No `TextOutput` or `OutputMessage` types exist in the codebase for representing emoji-prefixed text in the routing system.

## Expected Behavior
Create `TextOutput` struct with emoji, content, and newline fields. Extend `OutputMessage` enum with `TextOutput` variant.

## Impact
Provides the data structures needed for the OutputRouter to pass emoji-prefixed messages to destinations.

## Suggested Implementation

Create `apchat-main/src/mspc/output.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Simple message capturing what print_with_emoji prints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextOutput {
    pub emoji: String,
    pub content: String,
    pub newline: bool,
}

/// Extended output message type for router destinations
#[derive(Debug, Clone)]
pub enum OutputMessage {
    UserMessage { sender: String, text: String },
    AssistantResponse(String),
    ToolCall { name: String, args: serde_json::Value },
    ToolResult(String),
    SystemMessage(String),
    Error(String),
    TextOutput { emoji: String, content: String, newline: bool },
}
```

Add to `apchat-main/src/mspc/mod.rs`:
```rust
pub mod output;
pub use output::{TextOutput, OutputMessage};
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
Issue was already implemented before processing. The `apchat-main/src/mspc/output.rs` file contains:
1. `TextOutput` struct with fields: `emoji: String`, `content: String`, `newline: bool`
2. `OutputMessage` enum with a `TextOutput` variant

Also fixed a dependency issue in `crates/apchat-mspc/Cargo.toml` where the tokio dev-dependencies incorrectly specified a `test` feature which doesn't exist.

Commit: `32d1548`

---
*Created: 2026-02-03*
*Resolved: 2026-02-03*
