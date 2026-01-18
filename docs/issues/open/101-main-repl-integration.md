# Issue 101: Main REPL Integration with MSPC Loop

## Summary
The main REPL loop needs to be updated to use the MSPC channel system instead of the old synchronous loop.

## Location
- File: `src/app/repl.rs`
- Function: `start_repl`

## Current Behavior
The current REPL uses a synchronous loop that doesn't integrate with the MSPC channel system. The MSPC chat loop is implemented but not connected to the main application flow.

## Expected Behavior
The REPL should:
1. Initialize the MSPC channel
2. Spawn the terminal input router
3. Spawn the MSPC chat loop
4. Connect all components together
5. Handle graceful shutdown

## Impact
Without this integration, the input decoupling feature is inaccessible to users. The core infrastructure exists but cannot be used.

## Suggested Implementation

### Step 1: Update `start_repl` function

```rust
pub async fn start_repl(
    session: Session,
    config: AppConfig,
    logger: Option<Arc<dyn ConversationLogger>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize MSPC channel
    let (mspc_tx, mspc_rx) = mspc_channel::create_channel();

    // Create input router
    let terminal_router = input_router::terminal::TerminalInputRouter::new(
        mspc_tx.clone(),
        config.clone(),
    );

    // Spawn input router task
    let router_handle = tokio::spawn(async move {
        terminal_router.run().await
    });

    // Create and run MSPC chat loop
    let mspc_loop = mspc_chat::MSPCChatLoop::new(
        mspc_rx,
        mspc_tx.clone(),
        session,
        config,
        logger,
    );

    // Run the chat loop
    mspc_loop.run().await?

    // Cleanup
    router_handle.abort();
    Ok(())
}
```

### Step 2: Update imports

Add to the top of the file:
```rust
use crate::mspc::channel::{self as mspc_channel, MSPCMessage};
use crate::input_router::{self, terminal};
use crate::chat::mspc_chat;
```

### Step 3: Update function signature

Change from:
```rust
pub async fn start_repl(...)
```

To:
```rust
pub async fn start_repl(...)
```

(Already correct, but ensure it's marked as `pub`)

## Resolution

This issue will be resolved by integrating the MSPC channel system into the main REPL loop. The implementation will:

1. Create MSPC channel on REPL start
2. Spawn terminal input router task
3. Initialize and run MSPC chat loop
4. Ensure proper cleanup on exit

**Files Modified:**
- `src/app/repl.rs`

**Testing:**
- All existing tests should pass
- Manual testing of REPL with various inputs
- Test interrupt handling
- Test command parsing
- Test confirmation prompts

---
*Created: 2026-01-18*
