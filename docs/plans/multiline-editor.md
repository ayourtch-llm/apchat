# Multiline Editor Implementation Plan

## Executive Summary

Transform the single-line readline into a full multiline editor supporting:
- **Shift-Enter** to insert newlines
- **Enter** to submit (when at end) or insert newline  
- **Arrow keys** to navigate between and within lines
- **Paste** that preserves newlines
- **Up to 10 lines** of editable input with scrolling
- **Full history support** for multiline input

**Estimated effort:** 4-6 hours, ~300-400 lines of code changes
**Complexity:** Medium (mostly straightforward refactoring)

---

## Current State

**File:** `crates/apchat-vty/src/readline.rs`

**Working:**
- ✅ Character insertion/deletion
- ✅ Cursor movement (left/right/home/end)
- ✅ History navigation
- ✅ Kill ring (copy/paste)
- ✅ Bracketed paste mode
- ✅ Ctrl-A, Ctrl-E, Ctrl-C, Ctrl-D
- ✅ Search mode (Ctrl-R)

**Limitations:**
- ❌ Single line buffer only
- ❌ Paste replaces newlines with spaces
- ❌ No way to insert newlines manually
- ❌ Enter always submits immediately

---

## Quick Reference: Data Structure Changes

### Before
```rust
pub struct Readline {
    line: String,           // Single line
    cursor: usize,          // Cursor position
    saved_line: String,     // For history
}
```

### After
```rust
pub struct Readline {
    lines: Vec<String>,     // Multiple lines
    cursor_line: usize,     // Which line we're on
    cursor_col: usize,      // Position in line
    max_lines: usize,       // Max display (10)
    scroll_offset: usize,   // For scrolling
    saved_lines: Vec<String>, // For history
}
```

---

## Implementation Steps

### Step 1: Update Struct (Phase 1)
**File:** `crates/apchat-vty/src/readline.rs`, line ~176

Change:
- `line: String` → `lines: Vec<String>`
- `cursor: usize` → `cursor_line: usize` + `cursor_col: usize`
- Add: `max_lines: usize` (default 10)
- Add: `scroll_offset: usize`
- `saved_line: String` → `saved_lines: Vec<String>`

### Step 2: Update Constructor
**Location:** `new()` method, line ~209

Initialize:
```rust
lines: vec![String::new()],
cursor_line: 0,
cursor_col: 0,
max_lines: 10,
scroll_offset: 0,
```

### Step 3: Add Core Methods

**Add after line 850:**

```rust
// Insert newline at cursor
pub fn handle_newline(&mut self) -> bool {
    let current = self.lines[self.cursor_line].clone();
    let before = &current[..self.cursor_col];
    let after = &current[self.cursor_col..];
    
    self.lines[self.cursor_line] = before.to_string();
    self.lines.insert(self.cursor_line + 1, after.to_string());
    
    self.cursor_line += 1;
    self.cursor_col = 0;
    self.update_scroll_offset();
    true
}

// Update scroll to keep cursor visible
fn update_scroll_offset(&mut self) {
    if self.cursor_line < self.scroll_offset {
        self.scroll_offset = self.cursor_line;
    } else if self.cursor_line >= self.scroll_offset + self.max_lines {
        self.scroll_offset = self.cursor_line - self.max_lines + 1;
    }
}

// Check if cursor at end of all text
fn is_at_end(&self) -> bool {
    self.cursor_line == self.lines.len() - 1
        && self.cursor_col == self.lines[self.cursor_line].len()
}
```

### Step 4: Update Backspace/Delete

**Modify `handle_backspace()` (~line 650):**

```rust
pub fn handle_backspace(&mut self) -> bool {
    if self.history_index.is_some() {
        self.exit_history_navigation();
    }
    
    // At start of line (not first): join with previous
    if self.cursor_col == 0 && self.cursor_line > 0 {
        let prev_len = self.lines[self.cursor_line - 1].len();
        let current = self.lines.remove(self.cursor_line);
        self.lines[self.cursor_line - 1].push_str(&current);
        self.cursor_line -= 1;
        self.cursor_col = prev_len;
        self.update_scroll_offset();
        return true;
    }
    
    // Within line: delete char before cursor
    if self.cursor_col > 0 {
        let line = &mut self.lines[self.cursor_line];
        let byte_pos = line.chars().take(self.cursor_col - 1)
            .map(|c| c.len_utf8()).sum();
        line.remove(byte_pos);
        self.cursor_col -= 1;
        return true;
    }
    
    false
}
```

**Similar for `handle_delete()` (~line 720):**
- Join with next line if at end

### Step 5: Update Arrow Keys

**In `handle_key_event()` (~line 1250):**

```rust
// Up arrow
KeyCode::Up => {
    if self.lines.len() > 1 && self.cursor_line > 0 {
        self.cursor_line -= 1;
        self.cursor_col = self.cursor_col
            .min(self.lines[self.cursor_line].len());
        self.update_scroll_offset();
        KeyResult::Redraw
    } else if self.history_up() {
        KeyResult::Redraw
    } else {
        KeyResult::Continue
    }
}

// Down arrow
KeyCode::Down => {
    if self.lines.len() > 1 && self.cursor_line < self.lines.len() - 1 {
        self.cursor_line += 1;
        self.cursor_col = self.cursor_col
            .min(self.lines[self.cursor_line].len());
        self.update_scroll_offset();
        KeyResult::Redraw
    } else if self.history_down() {
        KeyResult::Redraw
    } else {
        KeyResult::Continue
    }
}

// Left arrow
KeyCode::Left => {
    if self.cursor_col > 0 {
        self.cursor_col -= 1;
        KeyResult::Redraw
    } else if self.cursor_line > 0 {
        self.cursor_line -= 1;
        self.cursor_col = self.lines[self.cursor_line].len();
        self.update_scroll_offset();
        KeyResult::Redraw
    } else {
        KeyResult::Continue
    }
}

// Right arrow
KeyCode::Right => {
    if self.cursor_col < self.lines[self.cursor_line].len() {
        self.cursor_col += 1;
        KeyResult::Redraw
    } else if self.cursor_line < self.lines.len() - 1 {
        self.cursor_line += 1;
        self.cursor_col = 0;
        self.update_scroll_offset();
        KeyResult::Redraw
    } else {
        KeyResult::Continue
    }
}
```

### Step 6: Update Enter Key

**In `handle_key_event()` (~line 1190):**

```rust
KeyCode::Enter => {
    // Shift-Enter: Always insert newline
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        self.handle_newline();
        KeyResult::Redraw
    } 
    // Regular Enter
    else if self.is_at_end() {
        // At end: submit
        let text = self.text();
        if !text.trim().is_empty() {
            self.add_history_entry(&text);
        }
        self.reset_input();
        KeyResult::Return(ReadlineResult::Input(text))
    } else {
        // Not at end: insert newline
        self.handle_newline();
        KeyResult::Redraw
    }
}
```

**Helper methods to add:**
```rust
fn text(&self) -> String {
    self.lines.join("\n")
}

fn reset_input(&mut self) {
    self.lines = vec![String::new()];
    self.cursor_line = 0;
    self.cursor_col = 0;
    self.scroll_offset = 0;
    self.history_index = None;
}
```

### Step 7: Update Paste Handling

**Modify `handle_paste()` (~line 1070):**

```rust
pub fn handle_paste(&mut self, content: String) -> bool {
    if content.is_empty() {
        return false;
    }
    
    if self.history_index.is_some() {
        self.exit_history_navigation();
    }
    
    let lines: Vec<&str> = content.lines().collect();
    
    if lines.len() == 1 {
        // Single line: insert at cursor
        for c in lines[0].chars() {
            self.handle_char(c);
        }
    } else {
        // Multiple lines: split and insert
        let current = &self.lines[self.cursor_line];
        let before = &current[..self.cursor_col];
        let after = &current[self.cursor_col..];
        
        self.lines[self.cursor_line] = format!("{}{}", before, lines[0]);
        
        for (i, line) in lines.iter().skip(1).take(lines.len()-2).enumerate() {
            self.lines.insert(self.cursor_line + 1 + i, line.to_string());
        }
        
        let last = format!("{}{}", lines.last().unwrap(), after);
        self.lines.insert(self.cursor_line + lines.len(), last);
        
        self.cursor_line += lines.len() - 1;
        self.cursor_col = lines.last().unwrap().len();
    }
    
    true
}
```

### Step 8: Update History

**Modify `history_up()` (~line 550):**

```rust
pub fn history_up(&mut self) -> bool {
    if self.history.is_empty() {
        return false;
    }
    
    if self.history_index.is_none() {
        self.saved_lines = self.lines.clone();
        self.saved_cursor_line = self.cursor_line;
        self.saved_cursor_col = self.cursor_col;
    }
    
    let new_index = match self.history_index {
        Some(i) if i > 0 => i - 1,
        None => self.history.len() - 1,
        _ => return false,
    };
    
    self.history_index = Some(new_index);
    
    let entry = &self.history[new_index];
    self.lines = entry.lines().map(|s| s.to_string()).collect();
    self.cursor_line = self.lines.len().saturating_sub(1);
    self.cursor_col = self.lines.last().map(|l| l.len()).unwrap_or(0);
    
    true
}
```

**Similar for `history_down()` and `exit_history_navigation()`**

### Step 9: Update Redraw

**Modify `redraw()` (~line 950):**

```rust
pub fn redraw(&mut self, prompt: &str) {
    let stdout = &mut std::io::stdout();
    
    // Calculate visible range
    let start = self.scroll_offset;
    let end = (start + self.max_lines).min(self.lines.len());
    
    // Clear and redraw each visible line
    for i in start..end {
        stdout.queue(MoveToColumn(0)).ok();
        stdout.queue(Clear(ClearType::CurrentLine)).ok();
        
        if i == 0 {
            print!("{}", prompt);
        }
        
        println!("{}", self.lines[i]);
    }
    
    // Position cursor
    let visual_line = self.cursor_line - self.scroll_offset;
    let mut visual_col = self.cursor_col;
    if visual_line == 0 {
        visual_col += prompt.chars().count();
    }
    
    stdout.queue(MoveTo(visual_col as u16, visual_line as u16)).ok();
    stdout.flush().ok();
}
```

### Step 10: Update handle_char

**Modify to use cursor_col:**

```rust
pub fn handle_char(&mut self, c: char) -> bool {
    if self.history_index.is_some() {
        self.lines = vec![String::new()];
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.history_index = None;
    }
    
    let line = &mut self.lines[self.cursor_line];
    let byte_pos = line.chars().take(self.cursor_col)
        .map(|c| c.len_utf8()).sum();
    line.insert(byte_pos, c);
    self.cursor_col += 1;
    true
}
```

### Step 11: Update Accessor Methods

**Around line 288:**

```rust
pub fn text(&self) -> String {
    self.lines.join("\n")
}

pub fn line_count(&self) -> usize {
    self.lines.len()
}

// Remove or update old line() and cursor() methods
```

---

## Testing Checklist

- [ ] Shift-Enter creates newline
- [ ] Enter submits when at end
- [ ] Enter inserts newline when not at end
- [ ] Up/Down navigate between lines
- [ ] Left/Right navigate within/across lines
- [ ] Home/End work on current line
- [ ] Backspace joins lines
- [ ] Delete joins lines
- [ ] Paste preserves newlines
- [ ] History saves/restores multiline
- [ ] Scroll works when > 10 lines
- [ ] Ctrl-C/D work with multiline

---

## Files to Reference

- **Main implementation:** `crates/apchat-vty/src/readline.rs`
- **Previous paste fix:** `docs/plans/multiline-paste-implementation.md`
- **Keybindings:** `KEYBINDINGS.md`

---

## Notes for Fresh Session

1. **Start with data structure changes** - everything depends on this
2. **Work in phases** - complete each phase before moving to next
3. **Test frequently** - build and run after each major change
4. **Watch for byte vs character positions** - UTF-8 handling
5. **Previous commits** - see recent fixes for Ctrl-C, Ctrl-D, Ctrl-A, Ctrl-E
