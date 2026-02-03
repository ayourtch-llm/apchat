# Issue 138: Update print_with_emoji to send to router

## Summary
Modify `print_with_emoji` function to create and send `TextOutput` messages to `TEXT_OUTPUT_TX` for routing, while maintaining backward compatibility with direct writes.

## Location
- File: `crates/apchat-vty/src/lib.rs` or `src/print.rs`
- Function: `print_with_emoji`, `print_heart_red`, `print_heart_yellow`

## Current Behavior
`print_with_emoji` only writes directly to the provided writer, making it impossible to send output to multiple destinations.

## Expected Behavior
`print_with_emoji` should create a `TextOutput` message, send it to `TEXT_OUTPUT_TX` (non-blocking), and continue with existing direct write logic for backward compatibility.

## Impact
Enables emoji-prefixed text to be broadcast to all registered destinations while maintaining compatibility with existing code.

## Suggested Implementation

In `crates/apchat-vty/src/lib.rs`:

```rust
use apchat_main::mspc::TEXT_OUTPUT_TX;
use apchat_main::mspc::TextOutput;

pub fn print_with_emoji(
    emoji: &str,
    text: &str,
    newline: bool,
    mut writer: impl io::Write
) -> io::Result<()> {
    // Create TextOutput message
    let output = TextOutput {
        emoji: emoji.to_string(),
        content: text.to_string(),
        newline,
    };

    // Send to router (non-blocking, ignore errors)
    let _ = TEXT_OUTPUT_TX.send(output);

    // Existing direct write logic (backward compatibility)
    // ... rest of the existing implementation ...

    Ok(())
}
```

Note:
- Add `apchat_main` to dependencies if not already present
- Or create a wrapper module that can access the static without circular dependencies
- Consider using feature flags or optional dependencies to avoid coupling

## Resolution

---
*Created: 2026-02-03*
