# Issue 111: Add Sender Field to MSPCMessage Enum

## Summary
Add sender field to all MSPCMessage variants to support multi-source tracking.

## Location
- File: `apchat-main/src/mspc/channel.rs`
- Lines: 6-16 (enum definition)

## Current Behavior
The MSPCMessage enum has these variants:
- `UserInput(String)` - no sender field
- `InterruptSignal(String)` - no sender field
- `Command(String)` - no sender field
- `SystemPrompt(String)` - no sender field
- `ConfirmationRequest(String)` - no sender field
- `ConfirmationResponse(bool)` - no sender field
- `ToolResult(String)` - no sender field
- `Error(String)` - no sender field

Messages cannot be tracked by their source.

## Expected Behavior
MSPCMessage enum should include sender in variants with content:
- `UserInput { sender: String, content: String }`
- `InterruptSignal { sender: String, content: String }`
- `Command { sender: String, content: String }`
- SystemPrompt, Confirmation variants can remain as-is (system-generated)

## Impact
**CRITICAL**: This is a breaking change that requires updating all code using MSPCMessage.

Without sender information, we cannot:
- Track which source sent messages
- Broadcast outputs with sender context
- Implement interruption from specific sources
- Support multiple concurrent users

## Suggested Implementation

### Step 1: Update the MSPCMessage enum

```rust
#[derive(Debug, Clone)]
pub enum MspcMessage {
    UserInput {
        sender: String,        // "terminal", "webex-alice", "websocket-abc123"
        content: String,
    },
    InterruptSignal {
        sender: String,
        content: String,       // Content after "!" prefix
    },
    Command {
        sender: String,
        content: String,       // "/model", "/help", etc.
    },
    SystemPrompt(String),
    ConfirmationRequest(String),
    ConfirmationResponse(bool),
    ToolResult(String),
    Error(String),
}
```

### Step 2: Add helper methods

```rust
impl MspcMessage {
    /// Check if this message is an interrupt signal
    pub fn is_interrupt(&self) -> bool {
        matches!(self, MspcMessage::InterruptSignal { .. })
    }

    /// Get the sender of the message
    pub fn sender(&self) -> Option<&str> {
        match self {
            MspcMessage::UserInput { sender, .. } => Some(sender),
            MspcMessage::InterruptSignal { sender, .. } => Some(sender),
            MspcMessage::Command { sender, .. } => Some(sender),
            _ => None,
        }
    }
}
```

## Resolution

This issue will be resolved by adding sender fields to the MSPCMessage enum variants. After this change, all downstream code must be updated.

**Files Modified:**
- `apchat-main/src/mspc/channel.rs`

**Testing:**
- [ ] Verify sender field is preserved in all message types
- [ ] Test is_interrupt() and sender() helper methods work
- [ ] Ensure backward compatibility with existing code (will break)

**Next Steps:**
After this issue is resolved:
1. Update TerminalInputRouter to use new enum
2. Update WebexInputRouter to use new enum
3. Update all tests

---
*Created: 2026-01-18*
