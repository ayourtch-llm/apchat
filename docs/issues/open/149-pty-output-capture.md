# Issue 149: Implement PTY Output Capture

## Summary

The PTY session capture functionality has TODOs that prevent writing captured output to files. This feature is important for debugging and audit trails.

## Location
- File: `crates/apchat-terminal/src/session.rs`
- Lines: 
  - Line 150: In reader thread
  - Line 202: In read_and_process_output method

## Current Behavior

When `capture_enabled` is true, the code has placeholders but no actual file writing:
```rust
if session.capture_enabled {
    // TODO: Write to capture file
}
```

## Expected Behavior

The capture functionality should:
1. Write PTY output to a capture file
2. Include timestamps for each write operation
3. Handle file creation/rotation if needed
4. Support both streaming capture and buffered capture

## Impact

- **Debugging**: Without capture, debugging PTY issues is harder
- **Audit Trail**: No record of PTY activity for compliance/forensics
- **Testing**: Can't replay PTY sessions for testing

## Suggested Implementation

1. **Add capture file tracking**:
   ```rust
   struct Session {
       // ... existing fields
       capture_file: Option<std::fs::File>,
       capture_buffer: Vec<u8>,
   }
   ```

2. **Initialize capture file**:
   ```rust
   pub fn enable_capture(&mut self, path: &Path) -> Result<()> {
       let file = std::fs::File::create(path)?;
       self.capture_file = Some(file);
       self.capture_enabled = true;
       Ok(())
   }
   ```

3. **Write with timestamps**:
   ```rust
   let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
   if let Some(ref mut file) = self.capture_file {
       writeln!(file, "[{}] {}", timestamp, data)?;
   }
   ```

4. **Handle both capture modes**:
   - Streaming: Write immediately
   - Buffered: Accumulate and flush periodically

## Resolution

(TO BE ADDED WHEN FIXED)

---
*Created: 2026-02-04*
*Resolved: (TO BE ADDED)*
