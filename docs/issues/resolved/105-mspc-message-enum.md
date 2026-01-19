# Issue 105: Implement MSPCMessage Enum Variants

## Summary
Implement the complete MSPCMessage enum with all required variants according to the architecture plan.

## Location
- File: `src/mspc/message.rs`

## Current Behavior
The MSPCMessage enum may be incomplete or not match the architecture plan requirements.

## Expected Behavior
The MSPCMessage enum should include:
- UserInput with sender field
- InterruptSignal with sender field
- Command with sender field
- ConfirmationRequest
- ConfirmationResponse

## Impact
Without complete message types, the MSPC system cannot handle all required communication patterns.

## Suggested Implementation

### Step 1: Create complete MSPCMessage enum

```rust
// Add to src/mspc/message.rs

pub enum MSPCMessage {
    UserInput {
        sender: String,
        content: String,
    },
    InterruptSignal {
        sender: String,
        content: String,
    },
    Command {
        sender: String,
        content: String,
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

impl MSPCMessage {
    /// Check if this message is an interrupt signal
    pub fn is_interrupt(&self) -> bool {
        matches!(self, MSPCMessage::InterruptSignal { .. })
    }

    /// Get the sender of the message
    pub fn sender(&self) -> Option<&str> {
        match self {
            MSPCMessage::UserInput { sender, .. } => Some(sender),
            MSPCMessage::InterruptSignal { sender, .. } => Some(sender),
            MSPCMessage::Command { sender, .. } => Some(sender),
            MSPCMessage::ConfirmationRequest { .. } => None,
            MSPCMessage::ConfirmationResponse { .. } => None,
        }
    }
}
```

### Step 2: Add serialization support

```rust
// Add Serialize and Deserialize derives
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MSPCMessage {
    // ... enum variants as above ...
}
```

## Resolution
This will provide the complete message type system needed for MSPC communication.

**Files Modified:**
- `src/mspc/message.rs`

**Testing:**
- Test serialization/deserialization
- Test sender extraction
- Test interrupt detection
- Test all message variants

---
*Created: 2026-01-18*
