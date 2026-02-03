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

---
*Created: 2026-02-03*
