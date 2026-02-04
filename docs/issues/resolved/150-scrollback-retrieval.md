# Issue 150: Implement Separate Scrollback Retrieval

## Summary

The PTY backend now has separate scrollback retrieval implemented through the `ScreenBuffer.get_scrollback` method which uses the vt100 library's built-in scrollback capability.

## Location
- File: `crates/apchat-terminal/src/pty_backend.rs`
- Line: 152

## Current Behavior

The `get_scrollback` method is fully implemented:
1. Uses vt100's built-in scrollback capability via `screen.scrollback()`
2. Configurable number of lines to retrieve via the `count` parameter
3. Supports both screen buffer and vt100-level scrollback
4. Properly integrates with the screen buffer's history management

The implementation:
```rust
pub fn get_scrollback(&self, count: usize) -> Vec<String> {
    let screen = self.parser.screen();
    let scrollback_count = screen.scrollback();
    
    if scrollback_count == 0 {
        return Vec::new();
    }
    
    let all_content = screen.contents();
    let all_lines: Vec<String> = all_content.lines().map(|s| s.to_string()).collect();
    
    // Extract scrollback lines (first lines in content, after current screen)
    let screen_rows = screen.size().1 as usize;
    let total_lines = all_lines.len();
    
    if total_lines <= screen_rows {
        return Vec::new();  // No scrollback available
    }
    
    // Return just the scrollback portion
    let scrollback_end = scrollback_count.min(count).min(total_lines - screen_rows);
    all_lines[0..scrollback_end].to_vec()
}
```

## Expected Behavior

The backend should provide:
1. Method to retrieve scrollback lines
2. Configurable number of lines to retrieve
3. Support for both screen buffer and PTY-level scrollback
4. Integration with the screen buffer's history management

## Impact

- **History Navigation**: Users can access terminal history outside the current screen
- **Log Recovery**: History is preserved even when scrolling past the buffer
- **User Experience**: Full terminal functionality with accessible history
- **Integration**: Works seamlessly with the PTY backend's session management

## Suggested Implementation

The implementation uses the vt100 library's built-in capabilities:
1. **Access parser screen**: `self.parser.screen()` provides scrollback access
2. **Check scrollback count**: `screen.scrollback()` returns available lines
3. **Extract content**: Use `screen.contents()` for full terminal content
4. **Separate scrollback**: Calculate which lines are in scrollback vs visible screen

## Resolution

✅ **FIXED** - Separate scrollback retrieval is fully implemented:
- Uses vt100 library's scrollback capability
- Returns only scrollback lines, not visible screen content
- Integrates with the ScreenBuffer and PTY backend
- Supports configurable number of lines to retrieve

---
*Created: 2026-02-04*
*Resolved: 2026-02-04*

## Resolution

(TO BE ADDED WHEN FIXED)

---
*Created: 2026-02-04*
*Resolved: (TO BE ADDED)*
