# Issue 139: Initialize OutputRouter in main

## Summary
Initialize the `OutputRouter` early in the application startup, register all destination types, and start the background monitoring task.

## Location
- File: `apchat-main/src/main.rs`
- Function: `main` or initialization section

## Current Behavior
No OutputRouter initialization exists, so the routing system is not functional.

## Expected Behavior
Create OutputRouter instance, register ReadlineDestination and TerminalDestination (optionally FileDestination), and start the background monitoring task during application initialization.

## Impact
Activates the entire OutputRouter system for routing emoji-prefixed text to multiple destinations.

## Suggested Implementation

Add to `apchat-main/src/main.rs`:

```rust
use apchat_main::mspc::{initialize_output_router, OutputRouter};

async fn run_repl_mode(mspc_sender: tokio_mpsc::Sender<MspcMessage>) -> Result<()> {
    // Initialize output router
    let router = initialize_output_router(mspc_sender.clone()).await;
    
    // ... rest of REPL setup ...
}

// Or add new function in apchat-main/src/mspc/mod.rs:

pub async fn initialize_output_router(
    mspc_sender: tokio_mpsc::Sender<MspcMessage>,
) -> Arc<OutputRouter> {
    let router = Arc::new(OutputRouter::new());

    // Register readline destination
    let readline_dest = Arc::new(ReadlineDestination::new(mspc_sender));
    router.register(readline_dest).await;

    // Register terminal destination
    let terminal_dest: Arc<dyn OutputDestination> = Arc::new(TerminalDestination::new());
    router.register(terminal_dest).await;

    // Optional: Register file destination
    // let file_dest = Arc::new(FileDestination::new(
    //     PathBuf::from("emoji_output.log")
    // ));
    // router.register(file_dest).await;

    // Start background monitoring
    router.start_monitoring();

    router
}
```

Ensure proper imports and module visibility are set up.

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

---
*Created: 2026-02-03*
