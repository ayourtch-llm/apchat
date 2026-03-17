# Multiline Editor Implementation Plan

## Goal
Transform the single-line readline into a multiline editor that supports:
- Shift-Enter to insert newlines
- Auto-wrap when line exceeds terminal width
- Display multiple lines (up to 10)
- Scroll within the multiline buffer
- Submit with Enter (only when not in middle of multiline input)

## Data Structure Changes

### Current Structure
```rust
pub struct Readline {
    line: String,           // Single line buffer
    cursor: usize,          // Cursor position in line
    // ... other fields ...
}
```

### New Structure
```rust
pub struct Readline {
    lines: Vec<String>,     // Multiple line buffers
    cursor_line: usize,     // Which line we're on
    cursor_col: usize,      // Cursor position within current line
    max_lines: usize,       // Maximum lines (default: 10)
    scroll_offset: usize,   // For scrolling if lines > max_lines
    // ... other fields ...
}
```

## Implementation Approach

### Phase 1: Data Structure Migration
1. Change `line` to `lines: Vec<String>`
2. Add `cursor_line` and `cursor_col` to replace single `cursor`
3. Update all cursor position calculations
4. Update history to store multiline strings (join with `\n`)

### Phase 2: Display Updates
1. Update `redraw()` to display multiple lines
2. Handle scrolling if `lines.len() > max_lines`
3. Update prompt display (show on first line only)
4. Clear/redraw multiple lines on each update

### Phase 3: Input Handling
1. Handle Enter key:
   - If Shift-Enter: Insert newline (split current line at cursor)
   - If regular Enter:
     - If only 1 line: Submit
     - If multiple lines: Check if at end → Submit, else insert newline
2. Handle Backspace/Delete across line boundaries
3. Handle Up/Down arrows to navigate between lines
4. Handle Left/Right arrows across line boundaries
5. Handle Home/End for current line
6. Handle Ctrl-A/Ctrl-E for first/last line (or keep as line start/end)

### Phase 4: Paste Handling (Already Done!)
1. Our existing `handle_paste()` already replaces newlines with spaces
2. Update it to preserve newlines instead:
   ```rust
   pub fn handle_paste(&mut self, content: String) -> bool {
       // Split into lines and insert each line
       let pasted_lines: Vec<&str> = content.lines().collect();
       // Insert at current cursor position
       // ... implementation ...
   }
   ```

### Phase 5: Auto-Wrap (Optional Enhancement)
1. Detect when line exceeds terminal width
2. Automatically wrap to next line
3. Similar to how text editors handle word wrap

## Detailed Changes

### 1. Shift-Enter Detection

Crossterm can detect Shift-Enter:
```rust
KeyCode::Enter => {
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        // Shift-Enter: Insert newline
        self.handle_newline();
        KeyResult::Redraw
    } else {
        // Regular Enter: Submit if conditions met
        if self.lines.len() == 1 || self.is_at_end() {
            // Submit
            KeyResult::Return(ReadlineResult::Input(self.get_text()))
        } else {
            // Insert newline instead
            self.handle_newline();
            KeyResult::Redraw
        }
    }
}
```

### 2. Handle Newline Method

```rust
/// Inserts a newline at the current cursor position.
///
/// Splits the current line into two lines at the cursor position.
fn handle_newline(&mut self) -> bool {
    let current_line = &self.lines[self.cursor_line];
    let current_col = self.cursor_col;

    // Split the line at cursor position
    let before = &current_line[..current_col];
    let after = &current_line[current_col..];

    // Update current line and insert new line
    self.lines[self.cursor_line] = before.to_string();
    self.lines.insert(self.cursor_line + 1, after.to_string());

    // Move cursor to start of next line
    self.cursor_line += 1;
    self.cursor_col = 0;

    true
}
```

### 3. Multi-Line Redraw

```rust
pub fn redraw(&mut self, prompt: &str) {
    let mut stdout = std::io::stdout();

    // Calculate visible lines (with scrolling)
    let offset = self.scroll_offset;
    let end_line = (offset + self.max_lines).min(self.lines.len());

    // Move cursor to the start of our display area
    // We need to move up if we have multiple lines
    let current_line_count = self.lines.len();
    if current_line_count > 1 {
        // Move cursor up to the start of our multiline display
        stdout.queue(crossterm::cursor::MoveUp(current_line_count as u16 - 1)).ok();
    }
    stdout.queue(MoveToColumn(0)).ok();

    // Display each visible line
    for i in offset..end_line {
        // Clear the line
        stdout.queue(Clear(crossterm::terminal::ClearType::CurrentLine)).ok();

        // Display prompt only on first line
        if i == 0 {
            print!("{}", prompt);
        }

        // Display the line content
        println!("{}", self.lines[i]);
    }

    // Position cursor
    let cursor_row = self.cursor_line - self.scroll_offset;
    let cursor_col = self.cursor_col + prompt.chars().count();
    stdout.queue(crossterm::cursor::MoveTo(cursor_col as u16, cursor_row as u16)).ok();

    stdout.flush().ok();
}
```

### 4. Backspace Across Lines

```rust
pub fn handle_backspace(&mut self) -> bool {
    // If at start of a line (not first line)
    if self.cursor_col == 0 && self.cursor_line > 0 {
        // Join with previous line
        let prev_line_len = self.lines[self.cursor_line - 1].len();
        let current_line = self.lines.remove(self.cursor_line);
        self.lines[self.cursor_line - 1].push_str(&current_line);

        // Move cursor to end of previous line
        self.cursor_line -= 1;
        self.cursor_col = prev_line_len;
        true
    } else if self.cursor_col > 0 {
        // Normal backspace within line
        let current_line = &mut self.lines[self.cursor_line];
        current_line.remove(self.cursor_col - 1);
        self.cursor_col -= 1;
        true
    } else {
        false
    }
}
```

### 5. Arrow Key Navigation

```rust
// Up arrow: Move to previous line or end of previous line
KeyCode::Up => {
    if self.cursor_line > 0 {
        self.cursor_line -= 1;
        // Clamp cursor to line length
        self.cursor_col = self.cursor_col.min(self.lines[self.cursor_line].len());
        KeyResult::Redraw
    } else {
        KeyResult::Continue
    }
}

// Down arrow: Move to next line or end of next line
KeyCode::Down => {
    if self.cursor_line < self.lines.len() - 1 {
        self.cursor_line += 1;
        // Clamp cursor to line length
        self.cursor_col = self.cursor_col.min(self.lines[self.cursor_line].len());
        KeyResult::Redraw
    } else {
        KeyResult::Continue
    }
}

// Left arrow: Move left or to end of previous line
KeyCode::Left => {
    if self.cursor_col > 0 {
        self.cursor_col -= 1;
        KeyResult::Redraw
    } else if self.cursor_line > 0 {
        // Move to end of previous line
        self.cursor_line -= 1;
        self.cursor_col = self.lines[self.cursor_line].len();
        KeyResult::Redraw
    } else {
        KeyResult::Continue
    }
}

// Right arrow: Move right or to start of next line
KeyCode::Right => {
    if self.cursor_col < self.lines[self.cursor_line].len() {
        self.cursor_col += 1;
        KeyResult::Redraw
    } else if self.cursor_line < self.lines.len() - 1 {
        // Move to start of next line
        self.cursor_line += 1;
        self.cursor_col = 0;
        KeyResult::Redraw
    } else {
        KeyResult::Continue
    }
}
```

### 6. History with Multiline

```rust
/// Adds a multiline entry to history.
pub fn add_history_entry(&mut self, text: &str) {
    // Store as joined string with newlines
    let joined = text.lines().collect::<Vec<&str>>().join("\n");
    if !joined.trim().is_empty() {
        self.history.push(joined);
        // Trim history if needed
        if self.history.len() > self.max_history_size {
            self.history.remove(0);
        }
    }
}

/// Navigate to previous history entry.
pub fn history_up(&mut self) -> bool {
    if self.history.is_empty() {
        return false;
    }

    // Save current state if this is first navigation
    if self.history_index.is_none() {
        self.saved_lines = self.lines.clone();
    }

    let new_index = match self.history_index {
        Some(index) => {
            if index > 0 {
                index - 1
            } else {
                return false; // Already at oldest
            }
        }
        None => self.history.len() - 1,
    };

    self.history_index = Some(new_index);

    // Load history entry and split into lines
    let entry = &self.history[new_index];
    self.lines = entry.lines().map(|s| s.to_string()).collect();
    self.cursor_line = self.lines.len() - 1;
    self.cursor_col = self.lines.last().map(|l| l.len()).unwrap_or(0);

    true
}
```

### 7. Paste with Newlines Preserved

```rust
pub fn handle_paste(&mut self, content: String) -> bool {
    if content.is_empty() {
        return false;
    }

    // Exit history navigation if we were in it
    if self.history_index.is_some() {
        self.exit_history_navigation();
    }

    // Split pasted content into lines
    let pasted_lines: Vec<&str> = content.lines().collect();

    if pasted_lines.len() == 1 {
        // Single line: insert at cursor
        self.lines[self.cursor_line].insert_str(self.cursor_col, pasted_lines[0]);
        self.cursor_col += pasted_lines[0].len();
    } else {
        // Multiple lines: split current line and insert
        let current_line = &self.lines[self.cursor_line];
        let before = &current_line[..self.cursor_col];
        let after = &current_line[self.cursor_col..];

        // Update current line (before cursor)
        self.lines[self.cursor_line] = format!("{}{}", before, pasted_lines[0]);

        // Insert middle lines
        for (i, line) in pasted_lines.iter().take(pasted_lines.len() - 1).skip(1).enumerate() {
            self.lines.insert(self.cursor_line + 1 + i, line.to_string());
        }

        // Insert last line with the "after" part
        let last_line = format!("{}{}", pasted_lines.last().unwrap(), after);
        self.lines.insert(self.cursor_line + pasted_lines.len(), last_line);

        // Move cursor to end of last pasted line
        self.cursor_line += pasted_lines.len() - 1;
        self.cursor_col = pasted_lines.last().unwrap().len();
    }

    true
}
```

## Complexity Assessment

### Easy Parts ✅
- Data structure changes (straightforward)
- Shift-Enter detection (crossterm supports it)
- Basic newline insertion
- Redraw logic (just loop over lines)

### Medium Parts 🟡
- Cursor navigation across line boundaries
- Backspace/Delete across line boundaries
- History with multiline entries
- Paste with newlines

### Hard Parts 🔴
- Terminal scrolling when content exceeds screen
- Auto-wrap based on terminal width
- Complex cursor positioning calculations
- Redraw efficiency with many lines

## Implementation Order

1. ✅ Data structure migration
2. ✅ Basic newline handling (Shift-Enter)
3. ✅ Update redraw for multiple lines
4. ✅ Update cursor movement (arrows, home/end)
5. ✅ Update backspace/delete
6. ✅ Update paste handling
7. ✅ Update history
8. ✅ Testing and refinement
9. ⏸️ Auto-wrap (optional, can be added later)

## Estimated Complexity

- **Time:** 4-6 hours of focused work
- **Lines of code:** ~300-400 changes
- **Risk:** Medium (lots of edge cases to test)

## Alternative: Simplified Approach

If this seems too complex, here's a simpler alternative:

### Simplified Multiline
- Keep single line buffer but allow `\n` characters
- Display `\n` as `⏎` or similar in the line
- On submit, join with actual newlines
- Much simpler but less intuitive

**Recommendation:** Go with the full multiline editor - it's worth the effort!

## Testing Checklist

- [ ] Shift-Enter creates newline
- [ ] Enter submits when at end
- [ ] Arrow keys navigate between lines
- [ ] Backspace joins lines
- [ ] Paste with newlines works
- [ ] History navigation works
- [ ] Scroll when > 10 lines
- [ ] Auto-wrap when line too long (optional)
- [ ] Ctrl-C works with multiline
- [ ] Ctrl-D works with multiline
