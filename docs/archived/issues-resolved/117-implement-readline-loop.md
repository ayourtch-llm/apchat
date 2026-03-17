# Task 6: Implement main readline loop with event polling

**Status:** Resolved
**Created:** 2025-01-23
**Resolved:** 2025-01-23
**Task:** 6 from crossterm-readline implementation plan

## Description

Implement the main readline loop with event polling using a 100ms timeout to allow for MPSC signal checking in future tasks.

## Implementation Steps

- [x] Add result types
- [x] Implement main readline loop
- [x] Manual test
- [x] Commit

## Verification Criteria

- [x] Result types are defined (ReadlineResult, KeyResult)
- [x] `readline()` method implemented with 100ms timeout
- [x] `handle_key_event()` dispatches to appropriate handlers
- [x] Enter key returns Input(line)
- [x] Ctrl-C returns Interrupt
- [x] Ctrl-D returns Eof (if line empty)
- [x] Arrow keys work for navigation
- [x] Other keys trigger appropriate handlers
- [x] Manual testing shows responsive input

## Files Modified

- `crates/apchat-vty/src/readline.rs`

## Implementation Details

### Result Types

**ReadlineResult** - Public API result type
```rust
pub enum ReadlineResult {
    Input(String),    // User entered a line
    Eof,              // End of file (Ctrl-D)
    Interrupt,        // Interrupted (Ctrl-C)
}
```

**KeyResult** - Internal key handler result
```rust
enum KeyResult {
    Continue,                 // Continue reading input
    Redraw,                   // Redraw screen and continue
    Return(ReadlineResult),   // Return the specified result
}
```

### Methods Implemented

**`readline(&mut self, prompt: &str) -> io::Result<ReadlineResult>`**
- Displays initial prompt with `redraw(prompt)`
- Enters event loop with 100ms timeout
- On timeout: continue loop (future MPSC check point)
- On event: dispatch to `handle_key_event()`
- On KeyResult::Redraw: call `redraw(prompt)`
- On KeyResult::Return: clear line and return result

**`handle_key_event(&mut self, key: KeyEvent) -> KeyResult`**
- Main dispatch function for keyboard input
- Maps key codes to handler methods:
  - `Enter`: Submit line, add to history, return Input
  - `Ctrl-C`: Return Interrupt
  - `Ctrl-D`: Return Eof if line empty, else delete char
  - `Backspace`: Call `handle_backspace()`, return Redraw
  - `Delete`: Call `handle_delete()`, return Redraw
  - `Left/Right`: Call arrow handlers, return Redraw
  - `Home/End`: Call home/end handlers, return Redraw
  - `Up/Down`: Call history navigation, return Redraw
  - `Char`: Call `handle_char()`, return Redraw

### Event Polling

Uses `crossterm::event::poll(Duration::from_millis(100))` to check for events with timeout. This allows:
- Responsive input handling
- Future integration of MPSC signal checking on timeout
- No busy-waiting CPU usage

### Cleanup on Return

Before returning, clears the current line by:
1. Moving cursor to column 0
2. Clearing the current line
3. Flushing output

This ensures clean terminal state for caller.

## Commit

```
commit 5bb0b52
Author: [Author]
Date: [Date]

feat: implement main readline loop with event polling
```

## Notes

Task 6 was already completed in a previous implementation. The readline loop is fully functional with 100ms timeout polling, proper key event handling, and clean terminal state management.
