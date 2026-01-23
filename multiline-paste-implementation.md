# Implementation Plan: Multiline Paste Support in Readline

## Current Status

✅ **Issue 1 Fixed:** Ctrl-A and Ctrl-E keybindings added
🔄 **Issue 2 In Progress:** Multiline paste support

## Technical Analysis

### crossterm Version
We're using `crossterm = "0.28"`, which supports `Event::Paste` (added in v0.25).

### How Paste Events Work

1. **Bracketed Paste Mode:**
   - Must be enabled by sending `\x1b[?2004h` to the terminal
   - Terminal wraps pasted content: `\x1b[200~`content`\x1b[201~`
   - crossterm automatically handles these escape sequences when the mode is enabled
   - Results in `Event::Paste(content)` being emitted

2. **Without Bracketed Paste Mode:**
   - Paste is sent as raw keystrokes including newlines
   - Each newline triggers `KeyCode::Enter`
   - Our current code treats each Enter as "submit the line"
   - This causes the problem we're seeing

### crossterm's Event::Paste Behavior

According to crossterm documentation:
- `Event::Paste` is ONLY emitted when bracketed paste mode is enabled
- crossterm does NOT automatically enable bracketed paste mode
- We need to manually enable/disable it in our raw mode functions

## Implementation Plan

### Step 1: Add Event::Paste to Imports

**File:** `crates/apchat-vty/src/readline.rs`

```rust
use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers};
```

No change needed - `Event` is already imported and includes all variants.

### Step 2: Enable Bracketed Paste Mode

**File:** `crates/apchat-vty/src/readline.rs`
**Function:** `enable_raw_mode_on_stdin()`

After setting up termios, enable bracketed paste mode:

```rust
fn enable_raw_mode_on_stdin() -> io::Result<termios> {
    // ... existing code ...

    unsafe {
        // ... existing tcsetattr call ...
    }

    // Enable bracketed paste mode
    print!("\x1b[?2004h");
    io::stdout().flush()?;

    Ok(original)
}
```

### Step 3: Disable Bracketed Paste Mode

**File:** `crates/apchat-vty/src/readline.rs`
**Function:** `restore_terminal_settings()`

```rust
fn restore_terminal_settings(original: &termios) -> io::Result<()> {
    // ... existing code ...

    unsafe {
        // ... existing tcsetattr call ...
    }

    // Disable bracketed paste mode
    print!("\x1b[?2004l");
    io::stdout().flush()?;

    Ok(())
}
```

### Step 4: Add Paste Handler Method

**File:** `crates/apchat-vty/src/readline.rs`
**Location:** After other handler methods (around line 900)

```rust
/// Handles paste events from bracketed paste mode.
///
/// When text is pasted, we need to decide how to handle newlines.
/// The default behavior is to replace newlines with spaces to keep
/// the input on a single line.
///
/// # Arguments
///
/// * `content` - The pasted content (may contain newlines)
///
/// # Returns
///
/// * `true` - The line was modified, a redraw is needed
/// * `false` - Nothing to insert
pub fn handle_paste(&mut self, content: String) -> bool {
    if content.is_empty() {
        return false;
    }

    // Exit history navigation if we were in it
    if self.history_index.is_some() {
        self.exit_history_navigation();
    }

    // Replace newlines with spaces for single-line input
    // This converts multiline paste into a single line
    let content = content.replace('\n', " ").replace('\r', " ");

    // Collapse multiple spaces into single space
    let content = content.split_whitespace().collect::<Vec<_>>().join(" ");

    // Insert the cleaned content at cursor position
    self.insert_str(&content);

    true
}

/// Inserts a string at the cursor position.
///
/// Helper method to insert multiple characters at once.
///
/// # Arguments
///
/// * `s` - The string to insert
fn insert_str(&mut self, s: &str) {
    for c in s.chars() {
        self.insert_char(c);
    }
}
```

### Step 5: Handle Paste Events in Event Loop

**File:** `crates/apchat-vty/src/readline.rs`
**Function:** `readline()` (around line 1450-1530)

Update the event handling section:

```rust
// Main event loop
loop {
    // Poll for events with 100ms timeout
    if poll(Duration::from_millis(100))? {
        // Read the event
        let event = read()?;

        // Handle keyboard events
        match event {
            Event::Key(key) => {
                match self.handle_key_event(key) {
                    KeyResult::Continue => {}
                    KeyResult::Redraw => {
                        self.redraw(prompt);
                    }
                    KeyResult::Return(result) => {
                        // ... existing return code ...
                    }
                }
            }
            Event::Paste(content) => {
                if self.handle_paste(content) {
                    self.redraw(prompt);
                }
            }
            _ => {
                // Ignore other events (mouse, resize, etc.)
            }
        }
    }

    // ... rest of loop ...
}
```

### Step 6: Add Unit Tests

**File:** `crates/apchat-vty/src/readline.rs`
**Location:** In the `tests` module (around line 1549)

```rust
#[test]
fn test_handle_paste_single_line() {
    let mut readline = Readline::new().unwrap();

    // Paste single line
    assert!(readline.handle_paste("hello world".to_string()));
    assert_eq!(readline.line(), "hello world");
    assert_eq!(readline.cursor(), 11);
}

#[test]
fn test_handle_paste_multiline() {
    let mut readline = Readline::new().unwrap();

    // Paste multiline text - should become single line
    assert!(readline.handle_paste("line1\nline2\nline3".to_string()));
    assert_eq!(readline.line(), "line1 line2 line3");
}

#[test]
fn test_handle_paste_with_carriage_returns() {
    let mut readline = Readline::new().unwrap();

    // Paste with CRLF line endings
    assert!(readline.handle_paste("line1\r\nline2".to_string()));
    assert_eq!(readline.line(), "line1 line2");
}

#[test]
fn test_handle_paste_multiple_spaces() {
    let mut readline = Readline::new().unwrap();

    // Paste with extra whitespace
    assert!(readline.handle_paste("word1    word2".to_string()));
    assert_eq!(readline.line(), "word1 word2");
}

#[test]
fn test_handle_paste_empty() {
    let mut readline = Readline::new().unwrap();

    // Paste empty string
    assert!(!readline.handle_paste("".to_string()));
    assert_eq!(readline.line(), "");
}

#[test]
fn test_handle_paste_with_existing_text() {
    let mut readline = Readline::new().unwrap();
    readline.line = "start".to_string();
    readline.cursor = 5;

    // Paste at end of existing text
    assert!(readline.handle_paste(" end".to_string()));
    assert_eq!(readline.line(), "start end");
}

#[test]
fn test_handle_paste_in_middle_of_line() {
    let mut readline = Readline::new().unwrap();
    readline.line = "start end".to_string();
    readline.cursor = 5; // Position after "start"

    // Paste in middle of line
    assert!(readline.handle_paste(" middle".to_string()));
    assert_eq!(readline.line(), "start middle end");
}
```

### Step 7: Update Documentation

Add documentation to the `Readline` struct explaining paste behavior:

```rust
/// Readline implementation with terminal mode management.
///
/// # Paste Behavior
///
/// This implementation supports bracketed paste mode. When you paste
/// multiline text, the newlines are automatically converted to spaces
/// to keep the input on a single line.
///
/// Example:
/// ```text
/// Pasting:    Becomes:
/// line1       line1 line2 line3
/// line2
/// line3
/// ```
///
/// # Bracketed Paste Mode
///
/// Bracketed paste mode is automatically enabled when the Readline
/// instance is created and disabled when it's dropped. This allows
/// the application to distinguish between pasted content and manually
/// typed input.
///
/// # Keybindings
///
/// Movement:
/// - `Left`, `Right`, `Ctrl-Left`, `Ctrl-Right` - Move by character/word
/// - `Home`, `End`, `Ctrl-A`, `Ctrl-E` - Move to start/end
/// - `Up`, `Down` - Navigate history
///
/// Editing:
/// - `Backspace`, `Delete` - Delete characters
/// - `Ctrl-K`, `Ctrl-U`, `Ctrl-W` - Kill text
/// - `Ctrl-Y` - Yank (paste) last killed text
///
/// Special:
/// - `Enter` - Submit the current line
/// - `Ctrl-C` - Interrupt (sends interrupt signal)
/// - `Ctrl-D` - Exit if line is empty, otherwise delete character
/// - `Ctrl-R` - Reverse search in history
```

## Testing Checklist

- [ ] Paste single line of text
- [ ] Paste multiline text (should join with spaces)
- [ ] Paste with leading/trailing whitespace (should be trimmed/normalized)
- [ ] Paste while cursor is in middle of line
- [ ] Paste while in history navigation mode
- [ ] Paste empty content
- [ ] Paste very long content (1000+ characters)
- [ ] Verify bracketed paste mode is enabled on startup
- [ ] Verify bracketed paste mode is disabled on exit
- [ ] Test in different terminal emulators (iTerm, Terminal.app, etc.)

## Expected Behavior After Implementation

### Before
```
> [paste three lines]
line1
line2
line3
```
Result: Three separate submissions

### After
```
> [paste three lines]
> line1 line2 line3_
```
Result: Single line with spaces between words

## Edge Cases to Consider

1. **Very long paste:** Should work, but may need to limit line length
2. **Binary content:** Paste should only contain text, but we should handle gracefully
3. **Terminal without bracketed paste support:** Will fall back to raw paste (multiple Enter events)
4. **Paste with tabs:** Should convert tabs to spaces or preserve?
5. **Paste with ANSI codes:** Should we strip them?

## Future Enhancements

1. **Configurable paste behavior:**
   - Option to preserve newlines (for multiline input support)
   - Option to join with specific separator

2. **Paste from kill ring:**
   - Already implemented with Ctrl-Y

3. **Smart paste:**
   - Detect if paste looks like code and preserve formatting
   - Ask user how to handle ambiguous cases

## Implementation Order

1. Add imports (already done)
2. Enable/disable bracketed paste mode
3. Add `insert_str()` helper
4. Add `handle_paste()` method
5. Update event loop to handle `Event::Paste`
6. Add unit tests
7. Test manually with various paste scenarios
8. Update documentation
9. Commit changes

## Files to Modify

- `crates/apchat-vty/src/readline.rs`:
  - Add bracketed paste mode enable/disable
  - Add `handle_paste()` method
  - Add `insert_str()` helper
  - Update event loop in `readline()`
  - Add unit tests
  - Update documentation
