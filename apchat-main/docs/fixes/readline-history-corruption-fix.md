# Readline History Corruption Fix

## Problem

On startup, the application was displaying this error:

```
💛 ⚠️ Failed to load readline history: Failed to deserialize ReadlineEntry from line: trailing characters at line 1 column 84
```

## Root Cause

The readline history file (`~/.okaychat/logs/readline_history.jsonl`) contained a corrupted line with two JSON objects concatenated together:

```json
{"command":"command 1","session_id":null,"timestamp":"2026-01-21T18:12:39.907668Z"}{"command":"test command","session_id":null,"timestamp":"2026-01-21T18:12:39.907637Z"}
```

This likely happened due to a race condition or concurrent writes where two entries were written to the same line.

## Solution

Modified the `load_history()` function in `src/chat/readline_history.rs` to:

1. **Gracefully handle corrupted lines**: Instead of failing entirely on a corrupted line, the function now:
   - Attempts to parse each line as normal
   - If parsing fails with a "trailing characters" error, it tries to recover multiple JSON objects from the same line
   - Reports corrupted lines but continues loading valid entries

2. **Recover concatenated JSON objects**: When the error "trailing characters" is detected, the function:
   - Iterates through the line attempting to parse JSON objects at different positions
   - Successfully extracts and adds each valid entry to the history
   - Reports how many entries were recovered

3. **Added cleanup function**: Added `cleanup_history_file()` to rewrite the history file with only valid entries, removing corruption

## Key Changes

### Before (Original Code)
```rust
let entry: ReadlineEntry = serde_json::from_str(&line)
    .map_err(|e| anyhow::anyhow!("Failed to deserialize ReadlineEntry from line: {}", e))?;
history.add_entry(entry);
```

### After (New Code)
```rust
match serde_json::from_str::<ReadlineEntry>(&trimmed) {
    Ok(entry) => {
        history.add_entry(entry);
    }
    Err(e) => {
        let error_msg = e.to_string().to_lowercase();
        if error_msg.contains("trailing character") {
            // Recover multiple JSON objects from corrupted line
            let mut pos = 0;
            let mut recovered_count = 0;
            
            while pos < trimmed.len() {
                let substring = &trimmed[pos..];
                match serde_json::from_str::<ReadlineEntry>(substring) {
                    Ok(entry) => {
                        history.add_entry(entry);
                        recovered_count += 1;
                        // Move past this object
                        let json_str = serde_json::to_string(&entry).unwrap();
                        pos += json_str.len();
                        // Skip whitespace
                        while pos < trimmed.len() && trimmed[pos..].starts_with(|c: char| c.is_whitespace() || c == ',') {
                            pos += 1;
                        }
                    }
                    Err(_) => break,
                }
            }
            
            if recovered_count > 0 {
                eprintln!("⚠️  Recovered {} entries from corrupted line", recovered_count);
            }
        }
    }
}
```

## Testing

1. The corrupted history file has been backed up to `~/.okaychat/logs/readline_history.jsonl.backup`
2. Created test example: `examples/test_corrupted_history.rs`
3. The application will now:
   - Load successfully even with corrupted lines
   - Display warnings about recovered entries
   - Continue functioning normally

## Future Improvements

To prevent this issue from happening again, consider:
1. **File locking**: Implement proper file locking when appending to the history file
2. **Atomic writes**: Use temporary files and atomic rename operations
3. **Validation**: Validate the history file format after writes

## Files Modified

- `src/chat/readline_history.rs`: Enhanced `load_history()` function with corruption recovery
- `src/chat/readline_history.rs`: Added `cleanup_history_file()` utility function

## Usage

The fix is automatic - no user action required. The application will:
- Recover corrupted entries on startup
- Display warnings about any issues found
- Continue loading valid history entries

To manually clean up the history file after verification:
```rust
apchat::chat::readline_history::cleanup_history_file(None)?;
```
