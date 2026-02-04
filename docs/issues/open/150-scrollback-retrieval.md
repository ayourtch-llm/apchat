# Issue 150: Implement Separate Scrollback Retrieval

## Summary

The PTY backend has a TODO for implementing separate scrollback retrieval, which is needed to access terminal history beyond the current screen.

## Location
- File: `crates/apchat-terminal/src/pty_backend.rs`
- Line: 152

## Current Behavior

The scrollback retrieval functionality is not implemented:
```rust
// TODO: Implement separate scrollback retrieval if needed
```

## Expected Behavior

The backend should provide:
1. Method to retrieve scrollback lines
2. Configurable number of lines to retrieve
3. Support for both screen buffer and PTY-level scrollback
4. Integration with the screen buffer's history management

## Impact

- **History Navigation**: Users can't access terminal history outside the current screen
- **Log Recovery**: Lost history when scrolling past the buffer
- **User Experience**: Limited terminal functionality

## Suggested Implementation

1. **Add scrollback buffer** to PTY handler:
   ```rust
   struct PtyHandler {
       // ... existing fields
       scrollback_buffer: Vec<String>,
       max_scrollback_lines: usize,
   }
   ```

2. **Accumulate scrollback** when processing PTY output:
   ```rust
   fn process_output(&mut self, output: &[u8]) {
       // ... existing processing
       for line in new_lines {
           self.scrollback_buffer.push(line);
           if self.scrollback_buffer.len() > self.max_scrollback_lines {
               self.scrollback_buffer.remove(0);
           }
       }
   }
   ```

3. **Provide retrieval method**:
   ```rust
   pub fn get_scrollback(&self, count: usize) -> Vec<&str> {
       let start = self.scrollback_buffer.len().saturating_sub(count);
       self.scrollback_buffer[start..].iter().map(|s| s.as_str()).collect()
   }
   ```

4. **Integrate with screen buffer** for unified API

## Resolution

(TO BE ADDED WHEN FIXED)

---
*Created: 2026-02-04*
*Resolved: (TO BE ADDED)*
