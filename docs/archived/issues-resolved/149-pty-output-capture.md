# Issue 149: Implement PTY Output Capture

## Summary

The PTY session capture functionality has been implemented through the `SessionLogger` which writes JSON lines to capture files when `capture_enabled` is true. The capture functionality supports both streaming and event-based logging.

## Location
- File: `crates/apchat-terminal/src/session.rs`
- Lines: 
  - Line 150: In reader thread
  - Line 202: In read_and_process_output method

## Current Behavior

Capture is fully implemented via `SessionLogger`:
- Writes JSON lines to capture files when `capture_enabled` is true
- Includes timestamps in RFC3339 format
- Handles both input and output logging
- Supports capture start/stop with path tracking
- Calculates capture duration and byte counts

The `SessionLogger.log_output` method writes to the capture file with timestamps:
```rust
if let Some(ref mut capture_file) = self.capture_file {
    let entry = json!({
        "timestamp": Utc::now().to_rfc3339(),
        "data": data,
    });
    writeln!(capture_file, "{}", entry.to_string())?;
}
```

## Expected Behavior

The capture functionality should:
1. Write PTY output to a capture file
2. Include timestamps for each write operation
3. Handle file creation/rotation if needed
4. Support both streaming capture and buffered capture

## Impact

- **Debugging**: Complete PTY activity is logged for debugging
- **Audit Trail**: JSON lines format provides structured audit records
- **Testing**: Can replay PTY sessions from captured logs
- **Metrics**: Tracks capture duration and byte counts

## Suggested Implementation

The implementation uses `SessionLogger`:
1. **Capture file tracking**: `capture_file: Option<File>` in `SessionLogger`
2. **JSON line format**: Each capture entry is a JSON object with timestamp and data
3. **Event logging**: Separate methods for input (`log_input`), output (`log_output`), and events
4. **Capture management**: `start_capture()` and `stop_capture()` methods with path tracking

## Resolution

✅ **FIXED** - PTY output capture is fully implemented:
- SessionLogger captures all PTY I/O to JSON lines files
- Timestamps are RFC3339 formatted
- Capture files are stored in the logs directory
- Supports start/stop capture with metrics tracking

---
*Created: 2026-02-04*
*Resolved: 2026-02-04*

## Resolution

(TO BE ADDED WHEN FIXED)

---
*Created: 2026-02-04*
*Resolved: (TO BE ADDED)*
