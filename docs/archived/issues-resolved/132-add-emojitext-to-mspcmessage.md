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
Added the `EmojiText` variant to the `MspcMessage` enum in `crates/apchat-mspc/src/channel.rs`. The variant includes three fields:
- `emoji: String` - The emoji to display
- `content: String` - The content to display
- `newline: bool` - Whether to add a newline after the content

This enables routing of emoji-prefixed text through the MSPC channel system for the OutputRouter to communicate with the readline display.

Commit: `91f5d13`

---
*Created: 2026-02-03*
*Resolved: 2026-02-03*
