# Issue 140: Test OutputRouter implementation

## Summary
Write comprehensive tests for the OutputRouter system including unit tests for each destination, integration tests for message routing, and backward compatibility verification.

## Location
- New test files: `apchat-main/tests/test_output_router.rs`
- Unit tests: `apchat-main/src/mspc/router.rs`, `apchat-main/src/mspc/destinations.rs`
- Integration tests: `crates/apchat-vty/tests/test_readline_emoji.rs`

## Current Behavior
No tests exist for the OutputRouter system.

## Expected Behavior
Comprehensive test coverage including:
1. Unit tests for each destination implementation
2. Integration test verifying messages reach all destinations
3. Readline integration test verifying cursor save/restore
4. Backward compatibility test verifying direct writes still work
5. Error handling test for closed channels/inactive destinations

## Impact
Ensures the OutputRouter system works correctly and maintains backward compatibility.

## Suggested Implementation

Create `apchat-main/tests/test_output_router.rs`:

```rust
#[tokio::test]
async fn test_router_broadcasts_to_all_destinations() {
    // Register mock destinations
    // Send TextOutput via TEXT_OUTPUT_TX
    // Verify all destinations received the message
}

#[tokio::test]
async fn test_readline_destination_emoji_format() {
    // Register ReadlineDestination with mock MSPC sender
    // Send message via router
    // Verify EmojiText variant sent with correct fields
}

#[tokio::test]
async fn test_inactive_destination_ignored() {
    // Register destination, mark inactive
    // Send message
    // Verify destination not called
}

#[tokio::test]
async fn test_register_unregister_destinations() {
    // Test register, unregister, and count functions
}
```

Create `crates/apchat-vty/tests/test_readline_emoji.rs`:

```rust
#[tokio::test]
async fn test_emojitext_cursor_preservation() {
    // Setup readline with mock MSPC channel
    // Send EmojiText message
    // Verify cursor position preserved and input intact
}

#[test]
fn test_print_with_emoji_backward_compat() {
    // Call print_with_emoji with stdout
    // Verify output still works even if router fails
}
```

## Resolution

---
*Created: 2026-02-03*
