# Issue 137: Handle EmojiText in readline poll loop

## Summary
Extend the readline poll loop to handle `MspcMessage::EmojiText` by saving cursor position, clearing the current line, printing emoji text below the input, and restoring cursor position.

## Location
- File: `crates/apchat-vty/src/readline.rs`
- Function: `Readline::poll` or event handling loop

## Current Behavior
Readline poll loop does not handle `EmojiText` messages, so emoji-prefixed output cannot be displayed without disrupting the user's input line.

## Expected Behavior
When `EmojiText` message is received, save cursor position, clear current line, print emoji text below input, restore cursor to input position, and continue event loop without returning.

## Impact
Enables emoji-prefixed text to appear in readline display with a "scroll up" effect without disrupting user input.

## Suggested Implementation

In `crates/apchat-vty/src/readline.rs`, extend the event handling:

```rust
use crate::mspc::MspcMessage;

// In the poll loop's message handling:
match msg {
    // ... existing message types ...

    MspcMessage::EmojiText { emoji, content, newline } => {
        // 1. Save cursor position
        crossterm::cursor::SavePosition;

        // 2. Clear current line
        crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine);

        // 3. Print emoji text below the input
        let output = format!("{} {}", emoji, content);
        crossterm::execute!(
            std::io::stdout(),
            crossterm::cursor::MoveToColumn(0),
            crossterm::style::Print(output),
            if newline {
                crossterm::style::Print("\n")
            } else {
                crossterm::style::Print("")
            }
        )?;

        // 4. Restore cursor to input position
        crossterm::cursor::RestorePosition;

        // 5. Continue event loop (don't return)
        continue;
    }

    // ... rest of handling ...
}
```

Note: Ensure proper imports are added for crossterm cursor and terminal modules.

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
