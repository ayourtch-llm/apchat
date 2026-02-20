# Auto-Wrapping User Prompt Editor - Design

**Date:** 2026-02-20
**Status:** Approved Design

## Overview

The user prompt editor in APChat currently does not handle line wrapping. When a line exceeds terminal width, the terminal wraps it visually, but the UI treats it as a single line, causing cursor positioning and navigation issues.

This design implements auto-splitting of long lines at word boundaries, ensuring the editor's internal state (`self.lines`) always represents visually correct lines. The solution reuses existing multiline infrastructure.

## High-Level Approach

Lines are automatically split when they exceed terminal width. The split happens in `handle_char()` after character insertion, using word boundary detection to avoid mid-word breaks. This ensures all existing rendering, scrolling, and navigation logic continues to work without modification.

**Key principle:** The split happens at visual overflow point, not cursor position. We calculate where the line actually exceeds terminal width, then search backward for a word boundary.

## Core Splitting Algorithm

### Split Function: `split_line_if_needed()`

1. **Check line width:**
   - Use existing `display_width()` function to calculate visible width
   - Get terminal width from `crossterm::terminal::size()`
   - For first line, subtract prompt width from available space

2. **Compare with limits:**
   - If `line_width <= available_width`, return early - no split needed

3. **Find split point:**
   - If overflow exists, iterate backward from end of line to find last space
   - This gives a word boundary for clean split
   - If no space found, force split at overflow point

4. **Perform the split:**
   - Extract portion after split: `after = &line[split_idx..]`
   - Truncate current line to split point
   - Insert `after` as new line at `cursor_line + 1`
   - Update `cursor_line += 1` and `cursor_col = 0`

5. **Cursor positioning:**
   - Cursor moves to beginning of new line
   - User continues typing where they left off

## Integration with Existing Code

### Modified Function: `handle_char()`

```rust
pub fn handle_char(&mut self, c: char) -> bool {
    // Exit history navigation (existing code)
    if self.history_index.is_some() {
        self.exit_history_navigation();
    }

    // Insert character at cursor position (existing code)
    let line = &mut self.lines[self.cursor_line];
    let byte_pos = line.chars().take(self.cursor_col).map(|c| c.len_utf8()).sum();
    line.insert(byte_pos, c);
    self.cursor_col += 1;

    // NEW: Check if we need to split this line
    self.split_line_if_needed();

    true
}
```

### New Function: `split_line_if_needed()`

Private method on `Readline` that:
- Checks if current line exceeds available terminal width
- Finds appropriate split point (word boundary preferred)
- Performs split if needed
- Updates cursor position to maintain typing continuity

## Edge Cases and Special Handling

### Prompt Width on First Line
When `self.cursor_line == 0`, subtract prompt's visible width from available space. Use `display_width()` to strip ANSI codes from prompt.

### Cursor Position During Split
If user is typing in middle of line and it overflows, split still happens at visual overflow point. Cursor stays at same character, which is now on new line.

### No Word Boundary Available
If line has no spaces (single word/URL), force split at exact overflow point. This is the correct fallback.

### Empty/Short Lines
These pass through unchanged - width check handles naturally.

### ANSI Codes and Unicode
Existing `display_width()` function correctly handles ANSI escape codes and wide Unicode characters (emojis). Split logic inherits this capability.

### Performance
Calling `display_width()` and `terminal_size()` on every character is acceptable for typical typing speeds. Optimize with width caching later if profiling shows issues.

## Testing Strategy

### Unit Tests
- Basic splitting: single line exceeds width, splits at word boundary
- No splitting: line fits within width, no modification
- Forced break: line with no spaces exceeds width, splits at overflow
- Cursor positioning: after split, cursor on new line at correct position
- Prompt width: first line respects prompt length
- Emoji/Unicode: correctly accounts for wide characters

### Integration Tests
- Typing paragraph: multiple splits as lines fill up
- Backspace after split: user can delete and lines merge
- History navigation: loading long lines triggers splitting

### Manual Testing
- Type long sentence, observe smooth wrapping
- Type in middle of line, verify split works
- Paste long paragraph, verify wrapping
- Resize terminal (initial behavior may not handle resize)

## Implementation Notes

- Changes localized to `crates/apchat-vty/src/readline.rs`
- No changes to rendering, scrolling, or navigation logic
- Existing multiline infrastructure fully reused
- Backwards compatible - history entries unchanged until loaded

## Success Criteria

1. Long lines wrap automatically at word boundaries
2. Cursor positioning and navigation work correctly with wrapped lines
3. Existing functionality (history, search, confirmation) unaffected
4. ANSI codes and Unicode handled correctly
5. No performance degradation during normal typing
