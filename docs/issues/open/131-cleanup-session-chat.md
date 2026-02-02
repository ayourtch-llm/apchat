# Issue 131: Clean up session.rs after REPL refactor

## Summary
Once `run_tool_loop()` is working with the LLM task, simplify or remove the old `session::chat()` function.

## Location
- File: `apchat-main/src/chat/session.rs`

## Current Behavior
`session::chat()` contains the full tool loop logic, duplicating what's now in `run_tool_loop()`.

## Expected Behavior
`session::chat()` is either:
- Removed entirely (if no other callers)
- Simplified to just delegate to `run_tool_loop()` for backwards compatibility
- Kept only for non-REPL use cases (web server, task mode)

## Impact
Removes code duplication, single source of truth for tool loop logic.

## Suggested Implementation

1. Search for all callers of `session::chat()`:
```bash
grep -r "session::chat" apchat-main/src/
```

2. If only called from REPL:
   - Remove `session::chat()`
   - Keep only `prepare_for_llm_call()` for the initial message setup

3. If called from other places (web server, task mode):
   - Keep `session::chat()` but have it create its own LLM task internally
   - Or refactor those callers to use the new architecture

4. Update `apchat-main/src/chat/mod.rs` exports accordingly.

## Verification
```bash
cargo build -p apchat
cargo test -p apchat
# Test all modes: -i, --web, --task
```

---
*Created: 2025-02-02*
