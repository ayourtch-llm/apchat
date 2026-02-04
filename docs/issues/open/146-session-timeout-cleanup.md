# Issue 146: Implement Session Timeout and Cleanup

## Summary

The `cleanup_inactive` method in `apchat-main/src/web/session_manager.rs` is a stub that always returns 0 without actually cleaning up inactive sessions. This is a security and resource management concern.

## Location
- File: `apchat-main/src/web/session_manager.rs`
- Function: `cleanup_inactive` (line 412)

## Current Behavior

The `cleanup_inactive` method currently does nothing:
```rust
pub async fn cleanup_inactive(&self, _timeout_seconds: i64) -> usize {
    // TODO: Implement session timeout and cleanup
    0
}
```

## Expected Behavior

The method should:
1. Iterate through all sessions
2. Check the last activity timestamp for each session
3. Remove sessions that have been inactive for longer than the timeout
4. Return the number of sessions cleaned up

## Impact

- **Security**: Inactive sessions could be hijacked or used by attackers
- **Resource Management**: Old sessions consume memory and potentially database connections
- **User Experience**: Stale sessions may cause confusion

## Suggested Implementation

1. Add a `last_activity` field to the `Session` struct or track it separately
2. Update `last_activity` timestamp on each session access
3. Implement cleanup logic:
   ```rust
   pub async fn cleanup_inactive(&self, timeout_seconds: i64) -> usize {
       let now = chrono::Utc::now().timestamp();
       let mut sessions = self.sessions.write().await;
       let before_len = sessions.len();
       
       sessions.retain(|_, session| {
           let session = session.read().await;
           now - session.last_activity < timeout_seconds
       });
       
       before_len - sessions.len()
   }
   ```
4. Call this method periodically (e.g., via a background task)

## Resolution

(TO BE ADDED WHEN FIXED)

---
*Created: 2026-02-04*
*Resolved: (TO BE ADDED)*
