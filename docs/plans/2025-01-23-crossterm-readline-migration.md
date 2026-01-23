# Crossterm Readline Migration Design

**Date:** 2025-01-23
**Author:** Design specification for migrating from rustyline to crossterm

## Overview

Migrate APChat's readline functionality from rustyline to a custom crossterm-based implementation with full feature parity including:
- Full readline-style editing (Emacs mode)
- History navigation with reverse search (Ctrl-R)
- Signal handling (Ctrl-C, Ctrl-D)
- MPSC signal checking with 100ms timeout
- Proper terminal mode handling (raw input, normal output)
- JSONL-based history persistence (keep existing)

## Architecture

### Module Structure

**New module:** `apchat-vty/src/readline.rs`

Contains the core `Readline` struct with full editing capabilities.

### Components

1. **Core Readline Engine** (`apchat-vty/src/readline.rs`)
   - Handles raw terminal mode using crossterm
   - Manages input buffer and cursor position
   - Processes keyboard events with full editing capabilities
   - Implements history navigation (up/down arrows, Ctrl-R)
   - Supports timeout-based polling (100ms) for MPSC signal checking

2. **History Management** (`apchat-main/src/chat/readline_history.rs`)
   - Keep existing JSONL-based history system
   - No changes needed

3. **Signal Integration** (modify `apchat-main/src/app/repl.rs`)
   - Replace `spawn_blocking` rustyline call with direct crossterm call
   - Periodically check MPSC channel for interrupt signals
   - Maintain the same `InterruptSignal` message flow

## Key Design Decisions

### Terminal Mode Handling

**Critical Insight from Rustyline:**
- Use **semi-raw mode**: raw input, normal output
- Disable `ICANON`, `ECHO` for character-by-character input
- **Keep `OPOST` enabled** so `\n` → `\r\n` conversion still works
- Do NOT enable mouse capture (allows text selection)

```rust
// Disable canonical mode and echo (input side)
raw.local_flags &= !(LocalFlags::ECHO | LocalFlags::ICANON | LocalFlags::IEXTEN | LocalFlags::ISIG);

// Keep output processing enabled! (don't touch c_oflag)
// This ensures \n still becomes \r\n
```

### Timeout Strategy

- **100ms timeout** on `crossterm::event::poll()`
- Allows checking MPSC channel for interrupt signals
- Most interrupts (Ctrl-C) come through keyboard immediately
- Future-proof for non-keyboard interrupt sources

### Key Bindings

**Basic Navigation:**
- `Left/Right Arrow` - Move cursor one character
- `Home/Ctrl-A` - Move to beginning of line
- `End/Ctrl-E` - Move to end of line
- `Ctrl-Left/Right` - Move by word

**Editing:**
- `Backspace` - Delete character before cursor
- `Delete/Ctrl-D` - Delete character at cursor
- `Ctrl-W` - Delete word before cursor
- `Ctrl-K` - Delete from cursor to end of line (kill to end)
- `Ctrl-U` - Delete from cursor to beginning of line
- `Ctrl-L` - Clear screen (redraw prompt and current line)

**History Navigation:**
- `Up/Down Arrow` - Previous/next history entry
- `Ctrl-R` - Reverse history search (immediate implementation)

**Reverse Search Mode (Ctrl-R):**
- Enter search state showing `(reverse-i-search)` prompt
- Typing filters history in real-time
- `Ctrl-R` again - cycle through matching entries
- `Enter` - accept selected match
- `Esc/Ctrl-G` - cancel search, return to original line
- `Backspace` - refine search pattern

**Special:**
- `Enter` - Submit line
- `Ctrl-C` - Send interrupt signal
- `Ctrl-D` - EOF (exit if empty line)

**Kill Ring:**
- Store killed text in a kill ring (max 16 entries)
- `Ctrl-Y` - Yank (paste) last killed text

## Public API

```rust
pub struct Readline {
    // Input buffer and cursor
    buffer: String,
    cursor_position: usize,

    // Prompt and mode
    prompt: String,
    mode: ReadlineMode,

    // History management
    history: Vec<String>,
    history_index: usize,  // Index when navigating (0 = newest)

    // Reverse search state
    search_pattern: String,
    search_matches: Vec<usize>,  // Indices of matching history entries
    search_match_index: usize,   // Current match index
    original_line: String,       // Saved line when entering search

    // Kill ring for yank/paste
    kill_ring: Vec<String>,
    kill_ring_index: usize,

    // Terminal state
    terminal: Terminal,
    original_termios: Option<Termios>,
}

pub enum ReadlineMode {
    Normal,
    ReverseSearch,
}

pub enum ReadlineResult {
    Input(String),
    Eof,           // Ctrl-D on empty line
    Interrupt,     // Ctrl-C
    Signal(MspcMessage),  // MPSC interrupt received
}

impl Readline {
    pub fn new() -> Result<Self>;
    pub fn set_prompt(&mut self, prompt: String);
    pub fn load_history(&mut self, path: &Path) -> Result<()>;
    pub fn save_history(&self) -> Result<()>;
    pub fn add_history_entry(&mut self, entry: &str);

    /// Main readline loop with timeout for MPSC checking
    pub fn readline(&mut self, mspc_channel: &MspcChannel) -> Result<ReadlineResult>;
}
```

## State Machine

```
Normal Mode:
  - Type → append to buffer
  - Ctrl-R → save buffer, switch to ReverseSearch
  - Up/Down → navigate history, replace buffer
  - Enter → return Input(buffer)

ReverseSearch Mode:
  - Type → update search_pattern, recompute matches
  - Ctrl-R → cycle through matches
  - Enter → accept match, return Input(selected)
  - Ctrl-G/Esc → cancel, restore original_line, return to Normal
```

## Main Readline Loop

```rust
pub fn readline(&mut self, mspc_channel: &MspcChannel) -> Result<ReadlineResult> {
    self.enable_raw_mode()?;
    self.redraw()?;

    loop {
        // Poll for events with 100ms timeout
        let poll_result = crossterm::event::poll(Duration::from_millis(100))?;

        if !poll_result {
            // Timeout - check MPSC channel for signals
            if let Some(signal) = self.check_mspc_signals(mspc_channel)? {
                self.cleanup()?;
                return Ok(ReadlineResult::Signal(signal));
            }
            continue;
        }

        // Event available - process it
        let event = crossterm::event::read()?;

        match event {
            Event::Key(key_event) => {
                match self.handle_key_event(key_event, mspc_channel)? {
                    KeyResult::Continue => continue,
                    KeyResult::Redraw => self.redraw()?,
                    KeyResult::Return(result) => {
                        self.cleanup()?;
                        return Ok(result);
                    }
                }
            }
            Event::Resize(_, _) => {
                self.redraw()?;
            }
            _ => {}
        }
    }
}
```

## Screen Rendering

Use crossterm's cursor positioning commands for clean redraws:

```rust
fn redraw_normal(&mut self) -> Result<()> {
    let stdout = stdout();
    let mut lock = stdout.lock();

    // Clear current line and move to beginning
    queue!(lock, Clear(ClearType::CurrentLine))?;
    queue!(lock, MoveToColumn(0))?;

    // Draw prompt
    queue!(lock, Print(&self.prompt))?;

    // Draw buffer (with cursor at correct position)
    let before_cursor = &self.buffer[..self.cursor_position];
    let after_cursor = &self.buffer[self.cursor_position..];

    queue!(lock, Print(before_cursor))?;
    queue!(lock, SavePosition)?;
    queue!(lock, Print(after_cursor))?;

    // Move cursor back to correct position
    let visible_len = after_cursor.chars().count();
    if visible_len > 0 {
        queue!(lock, MoveLeft(visible_len as u16))?;
    }

    lock.flush()?;
    Ok(())
}
```

**Key considerations:**
- Use `queue!` and flush once for efficiency
- Calculate visible width excluding ANSI codes
- Handle multi-byte Unicode with `.chars().count()`
- Save/Restore cursor for positioning

## Integration Points

### 1. Replace `ReadlineInstance` in `src/chat/readline_instance.rs`

Keep the singleton pattern, wrap the new crossterm `Readline` struct, maintain same public API.

### 2. Modify `load_and_add_to_editor()` in `src/chat/readline_history.rs`

Adapt to work with new API (load history entries into Readline struct).

### 3. Update REPL loop in `src/app/repl.rs`

Replace `spawn_blocking` rustyline call with direct `readline()` call, handle `ReadlineResult` enum.

## Dependencies

Add to `apchat-vty/Cargo.toml`:

```toml
[dependencies]
crossterm = "0.28"
```

Remove from `apchat-main/Cargo.toml`:

```toml
rustyline = "14.0"
```

## Testing Strategy

1. Unit tests for individual key handlers
2. Integration tests for history navigation
3. Manual testing for:
   - Ctrl-C interrupt behavior
   - Ctrl-D EOF behavior
   - Ctrl-R reverse search
   - Unicode handling
   - Terminal mode restoration

## Migration Steps

1. Create new `apchat-vty/src/readline.rs` module
2. Implement core readline functionality
3. Add unit tests
4. Integrate with existing `ReadlineInstance` singleton
5. Update REPL loop
6. Remove rustyline dependency
7. Test thoroughly
8. Update any remaining references
