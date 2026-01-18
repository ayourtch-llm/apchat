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
