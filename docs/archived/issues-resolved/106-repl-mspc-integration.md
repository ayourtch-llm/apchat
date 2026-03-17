# Issue 106: Update REPL Loop to Use MSPC Channel

## Summary
Update the REPL loop in src/app/repl.rs to use the MSPC channel for input/output instead of direct readline calls.

## Location
- File: `src/app/repl.rs`
- Function: `start_repl`

## Current Behavior
The REPL loop directly reads from readline and processes messages synchronously without using the MSPC channel.

## Expected Behavior
The REPL should:
1. Initialize MSPC channel
2. Spawn terminal input reader
3. Run MSPC chat loop
4. Connect output destinations

## Impact
Without this change, the MSPC infrastructure cannot be used in the main application flow.

## Suggested Implementation

### Step 1: Update function signature

```rust
pub async fn start_repl(
    session: Session,
    config: AppConfig,
    logger: Option<Arc<dyn ConversationLogger>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Implementation below
}
```

### Step 2: Create MSPC channel

```rust
let (mspc_tx, mspc_rx) = mspc_channel::create_channel();
```

### Step 3: Spawn terminal input reader

```rust
let terminal_router = input_router::TerminalInputRouter::new(
    mspc_tx.clone(),
    config.clone(),
);

let router_handle = tokio::spawn(async move {
    terminal_router.run().await
});
```

### Step 4: Create output destinations

```rust
let mut output_destinations: Vec<Box<dyn OutputDestination>> = vec![];

// Add terminal output
output_destinations.push(Box::new(TerminalOutputDestination::new()));

// Future: Add webex output
// output_destinations.push(Box::new(WebexOutputDestination::new(client)));
```

### Step 5: Create and run MSPC chat loop

```rust
let mspc_loop = mspc_chat::MSPCChatLoop::new(
    mspc_rx,
    mspc_tx.clone(),
    session,
    config,
    logger,
);

mspc_loop.run(&output_destinations).await?
```

### Step 6: Cleanup on exit

```rust
router_handle.abort();
Ok(())
```

## Resolution
This will integrate the REPL with the MSPC infrastructure, enabling input decoupling.

**Files Modified:**
- `src/app/repl.rs`

**Testing:**
- Run REPL normally
- Test interruption with "!"
- Test command parsing
- Verify output appears correctly

---
*Created: 2026-01-18*

## Resolution

This issue has been successfully implemented. The REPL loop now integrates with the MSPC infrastructure for input/output decoupling.

**Implementation Details:**

1. **MSPC Channel Integration:** The REPL now uses `MspcChannel` for communication between input/output components instead of direct readline calls.

2. **TerminalInputRouter:** Implemented to read terminal input and send messages to the MSPC channel properly parsed (UserInput, Command, InterruptSignal, ConfirmationResponse).

3. **TerminalOutputDestination:** Added to the output destinations vector for system output via MSPC.

4. **Signal Handling:** The REPL correctly receives and processes interrupt signals (`^C`) and command interruptions via the MSPC channel.

5. **Backward Compatibility:** All existing REPL functionality (commands like `/model`, `/history`, `/brainstorm`, etc.) continues to work alongside the new MSPC integration.

**Files Modified:**
- `apchat-main/src/app/repl.rs`: Refactored to use MSPC channels throughout

**Testing:**
- Integration tests verify MSPC message handling in REPL context
- Manual testing confirms UTF-8 encoding and prompt parsing work correctly

**Key Code Locations:**
- TerminalInputRouter in `apchat-main/src/input_router/terminal.rs`
- Output routing in `apchat-main/src/app/repl.rs` lines ~305-310
- Interrupt signal handling in `apchat-main/src/app/repl.rs` lines ~1070-1095

---
*Created: 2026-01-18*
*Resolved: 2026-01-25*