# Issue 132: Add EmojiText variant to MspcMessage

## Summary
Extend the `MspcMessage` enum to include an `EmojiText` variant for routing emoji-prefixed text output to the readline instance.

## Location
- File: `crates/apchat-mspc/src/channel.rs`
- Enum: `MspcMessage`

## Current Behavior
`MspcMessage` enum does not have a variant for emoji-prefixed text output, making it impossible to send such messages through the MSPC channel system.

## Expected Behavior
Add `EmojiText` variant to `MspcMessage` with emoji, content, and newline fields.

## Impact
This is a foundational change needed for the OutputRouter system to communicate emoji text to the readline display.

## Suggested Implementation

```rust
pub enum MspcMessage {
    // ... existing variants ...
    UserInput(String, Option<String>),
    SystemPrompt(String, Option<String>),
    ConfirmationRequest(String, Option<String>),
    ConfirmationResponse(bool, Option<String>),
    ToolConfirmationRequest { content: String, confirmation_id: String },
    ToolConfirmationResponse { approved: bool, reason: Option<String>, confirmation_id: String },
    InterruptSignal(String, Option<String>),
    Command(String, Option<String>),
    ToolResult(String, Option<String>),
    Error(String, Option<String>),

    // NEW: Emoji-prefixed text for display in readline
    EmojiText {
        emoji: String,
        content: String,
        newline: bool,
    },
}
```

## Resolution

---
*Created: 2026-02-03*
