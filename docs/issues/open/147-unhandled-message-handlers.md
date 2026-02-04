# Issue 147: Implement Unhandled Message Handlers in Web Routes

## Summary

The `handle_message` function in `apchat-main/src/web/routes.rs` has a fallback handler that only prints a yellow heart message for unhandled client messages. This needs proper implementation for all message types.

## Location
- File: `apchat-main/src/web/routes.rs`
- Function: `handle_message` (around line 223)

## Current Behavior

When a message type is not explicitly handled, it falls through to:
```rust
_ => {
    // TODO: Implement other message handlers
    print_heart_yellow(&format!("Unhandled client message: {:?}", message), true);
}
```

## Expected Behavior

All message variants from the client protocol should be properly handled:
1. Each message type should have a dedicated handler function
2. Unknown or unimplemented message types should return appropriate error responses
3. The fallback should provide better diagnostics for debugging

## Impact

- **Functionality**: Missing message handlers mean some client features won't work
- **Debugging**: Poor error messages make it hard to identify issues
- **Extensibility**: The codebase needs a pattern for adding new message types

## Suggested Implementation

1. First, identify all message variants in the client protocol
2. Implement handlers for each variant:
   - `handle_tool_call`
   - `handle_cancel_tool`
   - `handle_edit_plan`
   - etc.
3. Create a module structure for handlers:
   ```
   apchat-main/src/web/handlers/
   ├── mod.rs
   ├── session.rs
   ├── tool.rs
   └── message.rs
   ```
4. Add error responses for unknown message types
5. Add logging for debugging

## Resolution

(TO BE ADDED WHEN FIXED)

---
*Created: 2026-02-04*
*Resolved: (TO BE ADDED)*
