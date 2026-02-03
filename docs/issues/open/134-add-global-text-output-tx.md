# Issue 134: Add global TEXT_OUTPUT_TX broadcast channel

## Summary
Add a global lazy static broadcast channel (`TEXT_OUTPUT_TX`) for sending `TextOutput` messages from synchronous code without requiring async context.

## Location
- File: `apchat-main/src/mspc/mod.rs`
- Section: Module-level static

## Current Behavior
No global channel exists for broadcasting emoji-prefixed text output, making it impossible for `print_with_emoji` (sync function) to send messages to async destinations.

## Expected Behavior
Create a `TEXT_OUTPUT_TX` static broadcast channel with buffer size 100 that can be accessed from any part of the codebase.

## Impact
Enables non-blocking sends from sync code (like `print_with_emoji`) to async destinations via the OutputRouter system.

## Suggested Implementation

Add to `apchat-main/src/mspc/mod.rs`:

```rust
use tokio::sync::broadcast;
use once_cell::sync::Lazy;

/// Global broadcast channel for TextOutput messages
/// Allows non-blocking sends from synchronous code
pub static TEXT_OUTPUT_TX: Lazy<broadcast::Sender<TextOutput>> =
    Lazy::new(|| broadcast::channel(100).0);
```

Dependencies:
- `tokio` already in workspace
- May need to add `once_cell` if not present

## Resolution

---
*Created: 2026-02-03*
