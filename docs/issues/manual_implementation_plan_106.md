# Manual Implementation Plan for Issue 106

## Current State
The code has:
1. MSPC channel created (line 284)
2. Terminal input router spawned (lines 287-303)
3. But main loop still reads directly from readline (line 318)

## Required Changes

### 1. Add TerminalOutputDestination to imports
- Add `use crate::app::TerminalOutputDestination;`
- Add `use crate::mspc::OutputDestination;`

### 2. Create output destinations before main loop
```rust
let mut output_destinations: Vec<Box<dyn OutputDestination>> = vec![];
output_destinations.push(Box::new(TerminalOutputDestination::new()));
```

### 3. Refactor main loop
Replace the readline loop with MSPC receiver loop:
```rust
loop {
    // Try to receive message from MSPC channel (non-blocking)
    match mspc_channel.try_recv().await {
        Ok(Some(msg)) => {
            // Process message based on type
            match msg {
                MspcMessage::UserInput(content, _) => {
                    // Handle user input
                }
                MspcMessage::Command(content, _) => {
                    // Handle commands
                }
                MspcMessage::InterruptSignal(content, _) => {
                    // Handle interruptions
                }
                _ => {}
            }
        }
        Ok(None) => {
            // No messages, sleep briefly
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err(_) => {
            // Channel closed or error
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}
```

### 4. Update output to use OutputDestination
Replace all `println!` and `eprintln!` calls with:
```rust
broadcast_to_all(&output_destinations, OutputMessage::...).await;
```

## Files to Modify
- `apchat-main/src/app/repl.rs` - Main implementation

## Testing Strategy
1. `cargo build` - Verify compilation
2. `cargo test` - Run existing tests
3. Manual testing of REPL functionality
4. Verify output formatting
