# Multiline Paste Issue - Analysis & Implementation Plan

## Problem Statement

When pasting multiline text into the REPL, instead of being added to the current input line, the newlines cause the text to be split across multiple input submissions.

For example, pasting:
```
line 1
line 2
line 3
```

Results in three separate submissions instead of a single line with the content.

## Root Cause Analysis

### How Paste Works in Terminals

1. **Standard Paste (no bracketed paste mode):**
   - When you paste in a terminal, the terminal emulator sends the pasted content as a stream of characters
   - Newlines in the pasted text are sent as actual newline characters (or Enter key events)
   - The application receives these as individual key events or characters
   - The readline implementation treats newlines as "submit the current line"

2. **Bracketed Paste Mode:**
   - A terminal feature that wraps pasted content in special escape sequences:
     - `\x1b[200~` - Start of paste
     - Pasted content (may include newlines)
     - `\x1b[201~` - End of paste
   - This allows the application to distinguish paste events from manual typing
   - The application can then handle pasted content specially (e.g., replace newlines with spaces)

### Current Implementation

Looking at the code in `crates/apchat-vty/src/readline.rs`:

1. **Event Loop (line ~1450-1530):**
   - Uses `crossterm::event::poll()` and `read()` to get events
   - Only handles `Event::Key` events
   - Does not handle `Event::Paste` (if crossterm supports it) or bracketed paste escape sequences

2. **Enter Key Handler (line ~1200):**
   - `KeyCode::Enter` immediately returns with the current line
   - No way to distinguish between:
     - User pressing Enter manually
     - Paste containing newlines

3. **Character Input (line ~1380):**
   - `KeyCode::Char(c)` inserts a single character
   - Newlines from paste might come as `KeyCode::Enter` or as `\n` characters

## Solution Options

### Option 1: Enable Bracketed Paste Mode (RECOMMENDED)

**Advantages:**
- Clean distinction between paste and typing
- Standard approach used by most readline libraries (GNU readline, libedit, etc.)
- Allows smart handling (e.g., replace newlines with spaces, or preserve them)

**Implementation:**
1. Enable bracketed paste mode when entering raw mode
   - Send: `\x1b[?2004h`
2. Disable bracketed paste mode when exiting raw mode
   - Send: `\x1b[?2004l`
3. Handle the bracketed paste escape sequences in the event loop
4. When paste content is detected:
   - Option A: Replace all newlines with spaces (join into single line)
   - Option B: Preserve newlines as literal `\n` characters (for multiline input)
   - Option C: Ask user how to handle (not recommended for REPL)

**Code Changes:**
- Add bracketed paste enable/disable to `enable_raw_mode_on_stdin()` and `restore_terminal_settings()`
- Modify event loop to detect and handle `\x1b[200~` and `\x1b[201~` escape sequences
- Add state to track if we're in the middle of a paste event
- Process paste content as a batch instead of character-by-character

**Complexity:** Medium
**Crossterm Support:** Crossterm has `Event::Paste` (since version 0.25+) which should handle this automatically if we enable it

### Option 2: Debounce Detection

**Approach:**
- Detect rapid succession of Enter key events
- If multiple Enter events occur within a short time window (< 100ms), treat as paste
- Join the lines into a single input

**Advantages:**
- No terminal mode changes needed
- Works with terminals that don't support bracketed paste

**Disadvantages:**
- Heuristic-based (may not always work correctly)
- Can't distinguish between fast typing and paste
- Adds complexity to the event loop

**Complexity:** Medium-High

### Option 3: Always Join Lines (Simple but Limited)

**Approach:**
- Treat all Enter key events as inserting a newline character
- Require a different key (e.g., Ctrl-D or Esc+Enter) to submit

**Advantages:**
- Very simple to implement

**Disadvantages:**
- Changes the UX significantly
- Confusing for users
- Not standard behavior

**Complexity:** Low

## Recommended Implementation: Option 1 (Bracketed Paste Mode)

### Phase 1: Enable Bracketed Paste Support

1. **Enable bracketed paste mode in raw mode:**
   ```rust
   fn enable_raw_mode_on_stdin() -> io::Result<termios> {
       // ... existing code ...

       // Enable bracketed paste mode
       print("\x1b[?2004h");
       io::stdout().flush()?;

       Ok(original)
   }
   ```

2. **Disable bracketed paste mode when restoring:**
   ```rust
   fn restore_terminal_settings(original: &termios) -> io::Result<()> {
       // ... existing code ...

       // Disable bracketed paste mode
       print("\x1b[?2004l");
       io::stdout().flush()?;

       Ok(())
   }
   ```

3. **Update event loop to handle paste events:**
   - Check if crossterm's `Event::Paste` is available
   - Add handler for paste events in the main event loop

### Phase 2: Handle Paste Content

4. **Add paste handler:**
   ```rust
   // In handle_key_event or as a separate method
   fn handle_paste(&mut self, content: String) -> KeyResult {
       // Option A: Replace newlines with spaces
       let content = content.replace('\n', " ").replace('\r', " ");
       self.insert_str(&content);

       // Option B: Insert literal newlines
       // self.insert_str(&content);

       KeyResult::Redraw
   }
   ```

5. **Add helper to insert string (not just single char):**
   ```rust
   fn insert_str(&mut self, s: &str) {
       for c in s.chars() {
           self.insert_char(c);
       }
   }
   ```

### Phase 3: Handle Edge Cases

6. **Handle empty paste:** Ignore or return Continue
7. **Handle paste with only newlines:** May want to ignore or handle specially
8. **Handle very long paste:** May want to truncate or add confirmation

### Phase 4: Testing

9. **Test cases:**
   - Paste single line (should work same as before)
   - Paste multiline text (should join into single line)
   - Paste with leading/trailing whitespace
   - Paste empty content
   - Paste while cursor is in middle of line
   - Paste while in history navigation mode

### Phase 5: Documentation

10. **Add documentation:**
    - Explain bracketed paste mode behavior
    - Document how paste is handled (newlines → spaces)
    - Add examples to code

## Alternative: Use Crossterm's Built-in Paste Support

Crossterm (version 0.25+) has built-in support for bracketed paste mode through the `Event::Paste` event type. We should check:

1. What version of crossterm we're using
2. If `Event::Paste` is available
3. How to enable bracketed paste mode in crossterm

If available, we can use:
```rust
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

// In the event loop
if let Event::Paste(content) = event {
    match self.handle_paste(content) {
        KeyResult::Continue => {}
        KeyResult::Redraw => self.redraw(prompt),
        KeyResult::Return(result) => return Ok(result),
    }
}
```

## Implementation Checklist

- [ ] Check crossterm version and `Event::Paste` availability
- [ ] Enable bracketed paste mode in raw mode
- [ ] Disable bracketed paste mode when exiting raw mode
- [ ] Add `handle_paste()` method to `Readline`
- [ ] Add `insert_str()` helper method
- [ ] Update event loop to handle `Event::Paste`
- [ ] Test with various paste scenarios
- [ ] Update documentation
- [ ] Add unit tests for paste handling

## References

- [Bracketed Paste Mode - Readline documentation](https://tiswww.case.edu/php/chet/readline/readline.html)
- [Crossterm Event::Paste documentation](https://docs.rs/crossterm/latest/crossterm/event/enum.Event.html)
- [XTerm Control Sequences - Bracketed Paste Mode](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Bracketed-Paste-Mode)
