# Issue 104: Create MSPC Channel with Sender Field

## Summary
Create a new MSPC channel implementation that includes a sender field for message tracing.

## Location
- File: `src/mspc/channel.rs`

## Current Behavior
The existing MSPC channel doesn't include sender information in messages.

## Expected Behavior
- MSPCMessage enum should include sender information
- Channel should support sending messages with sender tags
- Messages should maintain sender information through the channel

## Impact
Without sender information, we cannot track which source sent messages.

## Suggested Implementation

### Step 1: Update MSPCMessage enum
Add sender field to relevant message variants:

```rust
pub enum MSPCMessage {
    UserInput {
        sender: String,        // NEW: "terminal", "webex-alice", "websocket-abc123"
        content: String,
    },
    InterruptSignal {
        sender: String,        // NEW
        content: String,
    },
    Command {
        sender: String,        // NEW
        content: String,
    },
    // Other variants can remain unchanged
}
```

### Step 2: Update channel sending methods
Ensure sender information is preserved when sending messages.

## Resolution
This will enable message tracking by source, which is necessary for Phase 1.

**Files Modified:**
- `src/mspc/channel.rs`

**Testing:**
- Verify sender field is preserved in messages
- Test all message types maintain sender information
- Ensure backward compatibility

---
*Created: 2026-01-18*
