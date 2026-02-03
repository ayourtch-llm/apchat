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

---
*Created: 2026-02-03*
