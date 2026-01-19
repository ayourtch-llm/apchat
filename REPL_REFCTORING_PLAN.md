# REPL Refactoring Plan

## Current State Analysis

1. The `run_repl_mode` function exists in `apchat-main/src/app/repl.rs`
2. MSPC infrastructure is already partially implemented but commented out
3. `chat_with_mspc` function exists in `apchat-main/src/chat/mspc_session.rs` but is not exported
4. TerminalInputRouter and TerminalOutputDestination are available
5. The current REPL loop uses direct readline calls (line 327)

## Implementation Plan

### Step 1: Export chat_with_mspc from chat module
- Update `apchat-main/src/chat/mod.rs` to export `chat_with_mspc`

### Step 2: Update run_repl_mode to use MSPC
- Create MSPC channel
- Set up output destinations (TerminalOutputDestination)
- Spawn terminal input router
- Call chat_with_mspc instead of direct readline loop
- Ensure proper cleanup

### Step 3: Keep backward compatibility
- Keep the function name as `run_repl_mode` for now
- The existing `start_repl` can remain or be added if needed

### Step 4: Test the implementation
- Run `cargo build` to check for compilation errors
- Run `cargo test` to ensure tests pass
