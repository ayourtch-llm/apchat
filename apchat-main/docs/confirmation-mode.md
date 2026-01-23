# Tool Confirmation Mode for Readline Interface

## Overview

This document describes the implementation of a confirmation mode system for the readline interface that allows tools to send confirmation requests directly to the user's terminal prompt. The key architectural change is the use of a **confirmation registry** to manage tool confirmations separately from the main MSPC channel.

## Motivation

Previously, there was a race condition where both tools and `mspc_session.rs` were trying to read from the same channel. This caused issues when tools needed user confirmation - the tool would wait for a response, but the response would be consumed by `mspc_session.rs` instead.

## Architecture

### Components

1. **ConfirmationRegistry** (`crates/apchat-toolcore/src/confirmation.rs`)
   - Manages pending tool confirmation requests
   - Uses `HashMap` to map unique IDs to oneshot senders
   - Provides `register()` to create a new pending confirmation
   - Provides `complete()` to send a response to a waiting tool

2. **ToolContext** (`crates/apchat-toolcore/src/tool_context.rs`)
   - Added `signal_receiver` and `confirmation_registry` fields
   - Rewrote `check_permission_via_signal()` to use the confirmation registry
   - Now sends `ToolConfirmationRequest` with a unique confirmation ID
   - Waits on the oneshot channel for the response

3. **MSPC Channel** (`crates/apchat-mspc/src/channel.rs`)
   - Added `ToolConfirmationRequest` message type
   - Added `ToolConfirmationResponse` message type
   - Updated message type detection methods

4. **Readline** (`crates/apchat-vty/src/readline.rs`)
   - Added `confirmation_id` field to track tool confirmation IDs
   - Updated `enter_confirmation_mode()` to accept an optional `confirmation_id`
   - Updated `exit_confirmation_mode()` to clear the `confirmation_id`
   - Updated `handle_confirmation_mode()` to return `ToolConfirmationResponse` when there's a confirmation ID
   - Added handling for `ToolConfirmationRequest` messages in the main readline loop

5. **APChat** (`apchat-main/src/apchat.rs`)
   - Added `confirmation_registry` field to `APChat` struct
   - Added `with_confirmation_registry()` builder method
   - Updated tool execution to pass the confirmation registry to the tool context

6. **REPL** (`apchat-main/src/app/repl.rs`)
   - Created `ConfirmationRegistry` instance and passed it to `chat`
   - Cloned `confirmation_registry` for use in the router task
   - Added handling for `__TOOL_CONFIRMATION_RESPONSE__:` errors
   - Routes confirmation responses to the registry instead of the main MSPC channel

### Message Flow

```
Tool                          Readline                  REPL
  |                              |                        |
  | check_permission_async()     |                        |
  |------------------------------|------------------------>|
  |                              |   register()           |
  |                              |   get ID + receiver    |
  |<-----------------------------|------------------------|
  | ToolConfirmationRequest      |                        |
  | (with confirmation_id)       |                        |
  |------------------------------|------------------------>|
  |                              | enter_confirmation_mode|
  |                              | [yellow prompt]        |
  |                              |                        |
  |                              | user presses y/n       |
  |                              | exit_confirmation_mode |
  | ToolConfirmationResponse     |                        |
  |<------------------------------|------------------------|
  |                              |   complete(ID, resp)   |
  |<-----------------------------|------------------------|
  |                              |                        |
  | oneshot receiver fired       |                        |
  | tool proceeds                |                        |
```

## Changes to ToolContext

### Manual Clone Implementation

Since `mpsc::Receiver` is not cloneable, `ToolContext` can no longer derive `Clone`. A manual implementation was added:

```rust
impl Clone for ToolContext {
    fn clone(&self) -> Self {
        // Note: signal_receiver is NOT cloned - each clone gets None
        Self {
            signal_sender: self.signal_sender.clone(),
            signal_receiver: None,
            confirmation_registry: self.confirmation_registry.clone(),
            .. // other fields
        }
    }
}
```

This means only the original `ToolContext` (created with `with_signal_receiver()`) can receive signals. Clones can only send signals.

### check_permission_via_signal Rewrite

The method now:
1. Registers with the confirmation registry to get a unique ID and oneshot receiver
2. Sends `ToolConfirmationRequest` via `signal_sender`
3. Waits on the oneshot channel for the response
4. Returns the approval status

```rust
pub async fn check_permission_via_signal(&self, tool_name: &str, prompt: &str) -> bool {
    let Some(ref registry) = self.confirmation_registry else {
        return false;
    };

    let (confirmation_id, receiver) = registry.register();

    let msg = ToolConfirmationRequest {
        tool_name: tool_name.to_string(),
        confirmation_id: confirmation_id.clone(),
        prompt: prompt.to_string(),
    };

    // ... send message, wait for response ...
}
```

## Usage Example

For a tool to request confirmation:

```rust
use apchat_toolcore::tool_context::ToolContext;

struct MyTool {
    context: ToolContext,
}

impl MyTool {
    async fn do_something(&self) -> Result<()> {
        let approved = self.context
            .check_permission_async("my_tool", "Do you want to proceed?")
            .await;

        if !approved {
            return Err(Error::PermissionDenied);
        }

        // Proceed with the operation
        Ok(())
    }
}
```

## Files Modified

1. `crates/apchat-toolcore/src/confirmation.rs` (new file)
2. `crates/apchat-toolcore/src/lib.rs`
3. `crates/apchat-toolcore/src/tool_context.rs`
4. `crates/apchat-mspc/src/channel.rs`
5. `crates/apchat-vty/src/readline.rs`
6. `crates/apchat-vty/src/instance.rs`
7. `apchat-main/src/apchat.rs`
8. `apchat-main/src/app/repl.rs`

## Testing

To test the confirmation system:

1. Start the apchat REPL
2. Invoke any tool that requires confirmation (e.g., `file_ops` tools)
3. A yellow confirmation prompt should appear
4. Press `y` to approve, `n` to deny, or `Esc`/`Ctrl+C` to cancel
5. The tool should proceed based on your choice

## Future Improvements

- Add timeout support for confirmation requests
- Add support for more complex confirmation dialogs (e.g., multi-select)
- Add support for default values for confirmations
