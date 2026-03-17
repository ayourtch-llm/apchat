# Task 11: Add MPSC signal checking to readline loop

**Status:** Resolved
**Created:** 2025-01-23
**Resolved:** 2025-01-23
**Task:** 11 from crossterm-readline implementation plan

## Description

Add MPSC signal checking capability to the readline loop so that external signals can interrupt input handling.

## Implementation Steps

- [x] Add apchat-mspc dependency
- [x] Update ReadlineResult to include signals
- [x] Update readline signature
- [x] Implement MPSC checking in timeout
- [x] Update call sites
- [x] Build and test
- [x] Commit

## Verification Criteria

- [x] apchat-mspc dependency added to Cargo.toml
- [x] ReadlineResult includes Signal(MspcMessage) variant
- [x] readline() accepts Option<&mut tokio::sync::mpsc::Receiver<MspcMessage>>
- [x] Timeout handler checks MPSC channel using try_recv()
- [x] Returns Signal(msg) when message received
- [x] Build succeeds (release mode)
- [x] Call sites updated with None parameter
- [x] Pattern matching updated to handle Signal variant

## Files Modified

- `crates/apchat-vty/Cargo.toml`
- `crates/apchat-vty/src/readline.rs`
- `apchat-main/src/chat/readline_instance.rs`

## Implementation Details

### Dependency Added

**crates/apchat-vty/Cargo.toml:**
```toml
[dependencies]
apchat-mspc = { path = "../apchat-mspc" }
```

### ReadlineResult Updated

Added new variant:
```rust
pub enum ReadlineResult {
    Input(String),
    Eof,
    Interrupt,
    Signal(MspcMessage),  // New variant
}
```

### Imports Added

```rust
use apchat_mspc::MspcMessage;
```

### readline() Signature Updated

**Before:**
```rust
pub fn readline(&mut self, prompt: &str) -> io::Result<ReadlineResult>
```

**After:**
```rust
pub fn readline(
    &mut self,
    prompt: &str,
    mut mspc_receiver: Option<&mut tokio::sync::mpsc::Receiver<MspcMessage>>,
) -> io::Result<ReadlineResult>
```

### MPSC Checking Implementation

In the timeout handler (after 100ms poll timeout):
```rust
if let Some(ref mut receiver) = mspc_receiver {
    // Try to receive a message without blocking
    if let Ok(msg) = receiver.try_recv() {
        // Clear the line before returning
        let mut stdout = std::io::stdout();
        stdout.queue(MoveToColumn(0)).ok();
        stdout.queue(Clear(crossterm::terminal::ClearType::CurrentLine)).ok();
        stdout.flush().ok();
        return Ok(ReadlineResult::Signal(msg));
    }
}
```

### Call Sites Updated

**apchat-main/src/chat/readline_instance.rs:**
```rust
match rl.readline(prompt, None)? {
    ReadlineResult::Input(line) => { /* ... */ }
    ReadlineResult::Eof => Ok(None),
    ReadlineResult::Interrupt => Ok(None),
    ReadlineResult::Signal(_msg) => {
        // For now, ignore signals in the basic readline interface
        // In Task 12, the REPL will handle signals properly
        Ok(None)
    }
}
```

## Technical Notes

### Mutable Reference Handling

The receiver is passed as `Option<&mut tokio::sync::mpsc::Receiver<MspcMessage>>` with a `mut` binding keyword to allow multiple calls to `try_recv()` in the loop without moving the value.

### Non-Blocking Check

Uses `try_recv()` which is non-blocking, returning immediately with:
- `Ok(Some(msg))` - Message available
- `Ok(None)` - Channel closed
- `Err(TryRecvError::Empty)` - No message available
- `Err(TryRecvError::Disconnected)` - Channel disconnected

### Integration Pattern

The current implementation passes `None` from `readline_instance.rs`, which will be updated in Task 12 to pass the actual MPSC receiver from the REPL context.

## Build Results

```
cargo build --release
    Finished `release` profile [optimized] target(s) in 13.49s
```

## Commit

```
commit 1681087
Author: [Author]
Date: 2025-01-23

feat: add MPSC signal checking to readline loop

 3 files changed, 40 insertions(+), 7 deletions(-)
```

## Code Statistics

- Files modified: 3
- Lines added: 40
- Lines removed: 7
- Net change: +33 lines

## Notes

Task 11 has been successfully implemented. The readline loop now supports checking for MPSC signals during the 100ms timeout interval. The Signal variant is added to ReadlineResult but currently ignored in the basic readline interface (returning None). Task 12 will integrate this with the REPL to properly handle signals from the MPSC channel.
