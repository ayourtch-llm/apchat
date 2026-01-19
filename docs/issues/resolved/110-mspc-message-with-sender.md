# Issue 110: Update MSPCMessage Enum with Sender Field

## Summary
Update the existing MSPCMessage enum to include sender information for message tracing and multi-source support.

## Location
- File: `apchat-main/src/mspc/channel.rs`
- Lines: 6-16 (enum definition)

## Current Behavior
The MSPCMessage enum has variants but lacks sender field:
- UserInput(String)
- InterruptSignal(String)
- Command(String)
- etc.

Messages cannot be tracked by their source.

## Expected Behavior
MSPCMessage enum should include sender field in all relevant variants:
- UserInput { sender: String, content: String }
- InterruptSignal { sender: String, content: String }
- Command { sender: String, content: String }

## Impact
Without sender information, we cannot:
- Track which source sent messages
- Broadcast outputs with sender context
- Implement interruption from specific sources
- Support multiple concurrent users

## Suggested Implementation

### Step 1: Update the MSPCMessage enum
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    ConfirmationRequest {
        callback_id: String,
        prompt: String,
    },
    ConfirmationResponse {
        callback_id: String,
        approved: bool,
    },
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
            MspcMessage::ConfirmationRequest { .. } => None,
            MspcMessage::ConfirmationResponse { .. } => None,
        }
    }
}
```

### Step 3: Update TerminalInputRouter to use sender
```rust
// In src/input_router/terminal.rs
pub async fn send_to_channel(&self, message: MspcMessage) {
    // Ensure all messages from terminal have "terminal" sender
    let message_with_sender = match message {
        MspcMessage::UserInput(content) => MspcMessage::UserInput {
            sender: "terminal".to_string(),
            content,
        },
        MspcMessage::InterruptSignal(content) => MspcMessage::InterruptSignal {
            sender: "terminal".to_string(),
            content,
        },
        MspcMessage::Command(content) => MspcMessage::Command {
            sender: "terminal".to_string(),
            content,
        },
        other => other,
    };

    let _ = self.channel.send(message_with_sender).await;
}
```

### Step 4: Update parse_input to use new enum
```rust
pub fn parse_input(input: &str) -> MspcMessage {
    let trimmed = input.trim();

    if trimmed.starts_with('!') {
        MspcMessage::InterruptSignal {
            sender: "terminal".to_string(),
            content: trimmed[1..].to_string(),
        }
    } else if trimmed.starts_with('/') {
        MspcMessage::Command {
            sender: "terminal".to_string(),
            content: trimmed.to_string(),
        }
    } else if trimmed.starts_with("confirm:") || trimmed.starts_with("Confirm:") {
        // Parse callback_id from prompt
        MspcMessage::ConfirmationRequest {
            callback_id: "terminal".to_string(),
            prompt: trimmed.to_string(),
        }
    } else {
        MspcMessage::UserInput {
            sender: "terminal".to_string(),
            content: trimmed.to_string(),
        }
    }
}
```

## Testing
- [ ] Verify sender field is preserved in all message types
- [ ] Test parse_input returns messages with correct sender
- [ ] Verify is_interrupt() and sender() helper methods work
- [ ] Ensure backward compatibility with existing code

## Resolution

**Files Modified:**
- `apchat-main/src/mspc/channel.rs`

**Breaking Changes:**
This is a breaking change that affects all code using MSPCMessage. All downstream code will need to be updated to match the new enum structure.

**Next Steps:**
After this issue is resolved, update TerminalInputRouter to use the new enum structure (see Step 3 above).

---
*Created: 2026-01-18*
