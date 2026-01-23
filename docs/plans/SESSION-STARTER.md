# Session Starter Guide: Multiline Editor Implementation

## Quick Start

This guide contains everything needed to implement the multiline editor in a fresh session.

---

## Context & Background

### What We're Building

A multiline REPL editor that allows users to:
- Press **Shift-Enter** to insert newlines
- Press **Enter** to submit (when at end) or insert newline
- Navigate between lines with arrow keys
- Paste multiline content (preserving newlines!)
- Edit up to 10 lines with scrolling

**Why:** Current implementation only supports single-line input. Pasting multiline text joins lines with spaces. Users need proper multiline editing for complex queries.

### Recent History (Important Context)

**Last 3 commits:**
```
dba90c6 docs: Update readline documentation with new features
b0a80e3 feat: Add multiline paste support with bracketed paste mode
0ba4e37 feat: Add Ctrl-A and Ctrl-E keybindings
9f534bd fix: Handle Ctrl-C and Ctrl-D correctly in readline
```

**What's Working Now:**
- ✅ Bracketed paste mode enabled (terminal sends paste events)
- ✅ Ctrl-A, Ctrl-E move to start/end of line
- ✅ Ctrl-C sends interrupt, Ctrl-D sends EOF
- ✅ Paste events detected (but currently join lines with spaces)

**The Problem:**
- Paste replaces newlines with spaces (workaround for single-line buffer)
- No way to manually insert newlines
- Want to preserve newlines and allow true multiline editing

---

## File Locations

### Main Implementation File
```
crates/apchat-vty/src/readline.rs
```

**Key sections:**
- Line ~176: `struct Readline` definition
- Line ~209: `new()` constructor
- Line ~288: Accessor methods (`line()`, `cursor()`)
- Line ~500: History methods (`add_history_entry`, `history_up`, `history_down`)
- Line ~620: `handle_char()` - insert character
- Line ~650: `handle_backspace()` - delete before cursor
- Line ~720: `handle_delete()` - delete at cursor
- Line ~850: `handle_end()` - end of line
- Line ~950: `redraw()` - display the line
- Line ~1070: `handle_paste()` - process paste events
- Line ~1190: `handle_key_event()` - main key handler (Enter key here)
- Line ~1250: Arrow key handlers

### Documentation Files
```
docs/plans/multiline-editor.md           - Main implementation plan
docs/plans/multiline-paste-implementation.md - Previous paste implementation
KEYBINDINGS.md                           - User keybinding reference
CHANGES-SUMMARY.md                       - Summary of recent changes
```

---

## Data Structure Migration

### The Core Change

**Before:**
```rust
pub struct Readline {
    line: String,           // Single line buffer
    cursor: usize,          // Character offset in line
    saved_line: String,     // Saved state for history
    // ... other fields ...
}
```

**After:**
```rust
pub struct Readline {
    lines: Vec<String>,         // Multiple line buffers
    cursor_line: usize,         // Which line we're editing (0-indexed)
    cursor_col: usize,          // Position within current line
    max_lines: usize,           // Max lines to display (default: 10)
    scroll_offset: usize,       // For scrolling when > max_lines
    saved_lines: Vec<String>,   // Saved state for history
    // ... other fields ...
}
```

### Why This Works

- **lines: Vec<String>** - Each string is one line of input
- **cursor_line** - Which element of the Vec we're editing
- **cursor_col** - Where we are within that string
- **max_lines** - Prevent display from taking over whole screen
- **scroll_offset** - Which line is at the top of the display

---

## Implementation Order (Critical!)

### Phase 1: Foundation (DO THIS FIRST)

1. **Update struct definition** (line ~176)
2. **Update constructor** (line ~209)
3. **Update accessor methods** (line ~288)

**Why:** Everything else depends on this. Do not skip ahead.

### Phase 2: Core Multiline Operations

4. **Add `handle_newline()`** - Split line at cursor, insert new line
5. **Add `update_scroll_offset()`** - Keep cursor visible
6. **Add `is_at_end()`** - Check if at end of all text
7. **Update `handle_backspace()`** - Support joining lines
8. **Update `handle_delete()`** - Support joining lines

### Phase 3: Navigation

9. **Update arrow key handlers** - Navigate between/within lines
10. **Update Home/End handlers** - Work on current line

### Phase 4: Enter Key Behavior

11. **Update Enter key handler** - Shift-Enter vs regular Enter
12. **Add `text()` helper** - Join lines with \n
13. **Add `reset_input()` helper** - Clear and reset

### Phase 5: Paste Handling

14. **Update `handle_paste()`** - Preserve newlines instead of replacing with spaces

### Phase 6: History

15. **Update `history_up()`** - Load multiline history
16. **Update `history_down()`** - Navigate down
17. **Update `exit_history_navigation()`** - Restore multiline state
18. **Update `add_history_entry()`** - Already works (stores strings)

### Phase 7: Display

19. **Update `redraw()`** - Display multiple lines with scrolling
20. **Update `handle_char()`** - Use cursor_col instead of cursor

### Phase 8: Testing & Polish

21. **Test all functionality**
22. **Fix edge cases**
23. **Update documentation**

---

## Critical Implementation Details

### Byte vs Character Positions

**Rust strings are UTF-8:**
```rust
let s = "hello";
let byte_pos = s.chars().take(3).map(|c| c.len_utf8()).sum();
// byte_pos = 3 (all ASCII)

let s = "héllo";
let byte_pos = s.chars().take(3).map(|c| c.len_utf8()).sum();
// byte_pos = 4 (é is 2 bytes in UTF-8)
```

**Rule:** Always convert character position to byte position before indexing:
```rust
let byte_pos = line.chars().take(cursor_col).map(|c| c.len_utf8()).sum();
line.insert(byte_pos, new_char);
```

### Scrolling Logic

**Keep cursor visible:**
```rust
fn update_scroll_offset(&mut self) {
    // Cursor scrolled above visible area
    if self.cursor_line < self.scroll_offset {
        self.scroll_offset = self.cursor_line;
    }
    // Cursor scrolled below visible area
    else if self.cursor_line >= self.scroll_offset + self.max_lines {
        self.scroll_offset = self.cursor_line - self.max_lines + 1;
    }
}
```

### Backspace Across Lines

**At start of line (not first line):**
```rust
if cursor_col == 0 && cursor_line > 0 {
    // Join with previous line
    let prev_len = lines[cursor_line - 1].len();
    let current = lines.remove(cursor_line);
    lines[cursor_line - 1].push_str(&current);
    cursor_line -= 1;
    cursor_col = prev_len;
}
```

### Enter Key Logic

**Shift-Enter = Always newline:**
```rust
if key.modifiers.contains(KeyModifiers::SHIFT) {
    handle_newline();
}
```

**Regular Enter:**
- If at end of text → Submit
- If not at end → Insert newline

```rust
if is_at_end() {
    // Submit
    return KeyResult::Return(ReadlineResult::Input(text()));
} else {
    // Insert newline
    handle_newline();
}
```

### Paste with Newlines

**Single line:** Insert at cursor
**Multiple lines:**
1. Split current line at cursor
2. Insert pasted lines in middle
3. Cursor goes to end of last pasted line

```rust
let lines: Vec<&str> = content.lines().collect();
if lines.len() == 1 {
    // Insert at cursor
} else {
    // Split and insert
    let before = &current_line[..cursor_col];
    let after = &current_line[cursor_col..];
    // ... insert lines between before and after
}
```

---

## Common Pitfalls

### ❌ Don't Do This

```rust
// WRONG: Direct byte indexing
self.line[self.cursor] = 'x';

// WRONG: Assuming ASCII
let byte_pos = self.cursor_col;

// WRONG: Forgetting to clear display
redraw(prompt); // Leaves old lines visible
```

### ✅ Do This Instead

```rust
// RIGHT: Convert char pos to byte pos
let byte_pos = line.chars().take(cursor_col).map(|c| c.len_utf8()).sum();
line.insert(byte_pos, 'x');

// RIGHT: Always use character positions
let byte_pos = line.chars().take(cursor_col).map(|c| c.len_utf8()).sum();

// RIGHT: Clear before redraw
stdout.queue(Clear(ClearType::CurrentLine)).ok();
```

---

## Testing Strategy

### After Each Phase

1. **Build:** `cargo build`
2. **Run:** `cargo run --bin apchat`
3. **Test:**
   - Type some text
   - Try the new functionality
   - Check for panics

### Final Testing Checklist

- [ ] **Shift-Enter** creates newline
- [ ] **Enter** submits when at end of last line
- [ ] **Enter** inserts newline when not at end
- [ ] **Up/Down arrows** navigate between lines
- [ ] **Left/Right arrows** navigate within/across lines
- [ ] **Home/End** go to start/end of current line
- [ ] **Backspace** joins lines when at line start
- [ ] **Delete** joins lines when at line end
- [ ] **Paste** single line inserts at cursor
- [ ] **Paste** multiline creates multiple lines
- [ ] **History** saves multiline state
- [ ] **History** restores multiline state
- [ ] **Scroll** works when > 10 lines
- [ ] **Ctrl-C** works with multiline input
- [ ] **Ctrl-D** works with multiline input

---

## Git Workflow

### Before Starting
```bash
# Create a feature branch
git checkout -b feature/multiline-editor

# Check current state
git log --oneline -5
git status
```

### During Implementation
```bash
# Commit frequently after each phase
git add -A
git commit -m "wip: Phase 1 - data structure migration"

# Check what's changed
git diff HEAD~1
```

### When Done
```bash
# Final commit
git commit -m "feat: Implement multiline editor

- Shift-Enter to insert newlines
- Enter to submit when at end
- Arrow keys navigate between lines
- Paste preserves newlines
- History supports multiline input"

# Merge to main
git checkout main
git merge feature/multiline-editor
```

---

## Useful Commands

### Building
```bash
# Build everything
cargo build

# Build specific package
cargo build -p apchat-vty

# Build with output
cargo build 2>&1 | tail -20
```

### Running
```bash
# Run the REPL
cargo run --bin apchat

# Run with backtrace
RUST_BACKTRACE=1 cargo run --bin apchat
```

### Testing
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_handle_backspace

# Run tests in readline module
cargo test --package apchat-vty
```

### Debugging
```bash
# Check syntax
cargo check

# Format code
cargo fmt

# Lint
cargo clippy

# Show file with line numbers
less -N crates/apchat-vty/src/readline.rs
```

---

## Getting Unstuck

### Compiler Errors

**"field `line` does not exist"**
- → You changed the struct but didn't update all references
- → Search: `grep -n "self\.line" crates/apchat-vty/src/readline.rs`

**"field `cursor` does not exist"**
- → You changed the struct but didn't update all references
- → Search: `grep -n "self\.cursor" crates/apchat-vty/src/readline.rs`

**"cannot borrow as mutable"**
- → You might be trying to modify lines while iterating
- → Use indices instead of iterators

**"expected usize, found tuple"**
- → You're using old cursor (single value) instead of (line, col)
- → Split into cursor_line and cursor_col

### Runtime Panics

**"index out of bounds"**
- → Check cursor_line is within lines.len()
- → Add bounds checking: `self.lines.get(self.cursor_line)`

**"byte index not a char boundary"**
- → You're using byte position directly instead of converting
- → Always use char-to-byte conversion

### Logic Bugs

**Backspace not joining lines**
- → Check the condition: `cursor_col == 0 && cursor_line > 0`
- → Add debug prints to verify

**Scroll not working**
- → Check update_scroll_offset() is called after cursor movement
- → Verify max_lines is set correctly

**History not restoring**
- → Check saved_lines is being saved correctly
- → Verify history_index is being reset

---

## Quick Reference

### Key Method Locations

| Method | Line | Purpose |
|--------|------|---------|
| `new()` | ~209 | Constructor |
| `handle_char()` | ~620 | Insert character |
| `handle_backspace()` | ~650 | Delete before cursor |
| `handle_delete()` | ~720 | Delete at cursor |
| `redraw()` | ~950 | Display input |
| `handle_paste()` | ~1070 | Process paste |
| `handle_key_event()` | ~1150 | Main key handler |
| Arrow keys | ~1250 | Navigation |

### Crossterm Key Codes

```rust
KeyCode::Enter           // Enter key
KeyCode::Up/Down/Left/Right  // Arrow keys
KeyCode::Home/End        // Home/End keys
KeyCode::Char(c)         // Character key
KeyModifiers::SHIFT      // Shift modifier
KeyModifiers::CONTROL    // Ctrl modifier
KeyModifiers::ALT        // Alt modifier
```

---

## Final Notes

1. **Take it step by step** - Don't try to do everything at once
2. **Build frequently** - Catch errors early
3. **Test as you go** - Verify each phase works
4. **Read the error messages** - They're usually helpful
5. **Use git wisely** - Commit after each working phase

**Good luck! This is a solid plan and very achievable.** 🚀

---

## Appendix: Full Example

Here's a complete example of what the updated `handle_backspace` should look like:

```rust
pub fn handle_backspace(&mut self) -> bool {
    // Exit history navigation
    if self.history_index.is_some() {
        self.exit_history_navigation();
    }
    
    // Case 1: At start of a line (not first line) - join with previous
    if self.cursor_col == 0 && self.cursor_line > 0 {
        let prev_len = self.lines[self.cursor_line - 1].len();
        let current_line = self.lines.remove(self.cursor_line);
        self.lines[self.cursor_line - 1].push_str(&current_line);
        
        self.cursor_line -= 1;
        self.cursor_col = prev_len;
        
        self.update_scroll_offset();
        return true;
    }
    
    // Case 2: Within a line - delete character before cursor
    if self.cursor_col > 0 {
        let line = &mut self.lines[self.cursor_line];
        
        // Convert char position to byte position
        let byte_pos = line.chars()
            .take(self.cursor_col - 1)
            .map(|c| c.len_utf8())
            .sum();
        
        // Get character length (for UTF-8)
        let char_len = line[byte_pos..].chars().next().unwrap().len_utf8();
        
        // Remove character
        line.remove(byte_pos);
        
        self.cursor_col -= 1;
        return true;
    }
    
    // Case 3: At start of first line - nothing to delete
    false
}
```

Use this as a template for other methods that need updating.
