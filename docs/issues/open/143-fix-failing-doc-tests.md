# Issue 143: Fix Failing Doc Tests in apchat-vty

## Summary

Several doc tests in `crates/apchat-vty/src/readline.rs` are failing during `cargo test --doc`. These tests are failing because they:
1. Try to access private fields (`line`, `cursor`, `history_index`) that are not available in the public API
2. Require a TTY environment (terminal) which is not available in CI/CD pipelines
3. Some have compilation errors due to incorrect API usage

## Location
- File: `crates/apchat-vty/src/readline.rs`
- Lines: Various doc test examples throughout the file

## Current Behavior

Running `cargo test -p apchat-vty --doc` produces the following failures:

```
test crates/apchat-vty/src/readline.rs - readline::Readline::handle_char (line 787) ... FAILED
test crates/apchat-vty/src/readline.rs - readline::Readline::handle_backspace (line 833) ... FAILED
test crates/apchat-vty/src/readline.rs - readline::Readline::exit_history_navigation (line 615) ... FAILED
test crates/apchat-vty/src/instance.rs - instance::ReadlineInstance::get (line 107) ... FAILED
test crates/apchat-vty/src/readline.rs - readline::Readline::handle_delete (line 897) ... FAILED
test crates/apchat-vty/src/readline.rs - readline::Readline::handle_home (line 1057) ... FAILED
test crates/apchat-vty/src/readline.rs - readline::Readline::handle_end (line 1092) ... FAILED
test crates/apchat-vty/src/readline.rs - readline::Readline::handle_left (line 957) ... FAILED
test crates/apchat-vty/src/readline.rs - readline::Readline::handle_right (line 1006) ... FAILED
test crates/apchat-vty/src/readline.rs - readline::Readline::cursor (line 344) ... FAILED
test crates/apchat-vty/src/readline.rs - readline::Readline::add_history_entry (line 417) ... FAILED
test crates/apchat-vty/src/readline.rs - readline::Readline::history_up (line 456) ... FAILED
test crates/apchat-vty/src/readline.rs - readline::Readline::get_history_entries (line 592) ... FAILED
test crates/apchat-vty/src/readline.rs - readline::Readline::redraw (line 1542) - compile ... FAILED
test crates/apchat-vty/src/readline.rs - readline::Readline::readline (line 2149) - compile ... FAILED
test crates/apchat-vty/src/readline.rs - readline::Readline::history_down (line 535) ... FAILED
test crates/apchat-vty/src/readline.rs - readline::Readline::line (line 330) ... FAILED
```

## Expected Behavior

All doc tests should either:
1. Pass successfully (if they can be fixed to use public API)
2. Be marked with `#[ignore]` attribute to skip in non-TTY environments

## Impact

- Failing doc tests cause CI/CD pipelines to fail
- Users running `cargo test` see confusing failures
- Doc tests don't provide value if they can't run in standard environments

## Suggested Implementation

### Step 1: Add `#[ignore]` attribute to failing doc tests

Since these tests require a TTY to run (they call `Readline::new()` which initializes terminal mode), they should be marked with `#[ignore]`:

```rust
/// # Example
///
/// ```ignore
/// use apchat_vty::Readline;
///
/// let mut readline = Readline::new().unwrap();
/// // ... rest of example
/// ```
```

### Step 2: Fix doc tests that can be fixed without TTY

For doc tests that don't require terminal initialization, update them to:
1. Use public API methods instead of private fields
2. Replace `readline.cursor = X` with `readline.set_cursor(X)` if available
3. Replace `readline.line = "..."` with `readline.set_line("...")` if available
4. Replace `readline.history_index` with `readline.get_history_index()` if available

### Step 3: Add doc tests that don't require terminal

Create simple doc tests that demonstrate usage without requiring a TTY:
```rust
/// # Example
///
/// ```no_run
/// // Show example without running it
/// let readline = Readline::new(); // This would require TTY
/// ```
```

### Step 4: Verify all doc tests pass

```bash
cargo test -p apchat-vty --doc
```

All tests should either pass or be ignored.

## Resolution

**Date:** 2026-02-04

**Changes Made:**
- Added `#[ignore]` attribute to all doc tests that require terminal initialization
- Fixed doc tests that access private fields to use public API
- Updated doc test examples to be more accurate

**Files Modified:**
- `crates/apchat-vty/src/readline.rs` - Updated doc test examples

**Commit:** (to be added)

---
*Created: 2026-02-04*
*Resolved: (to be filled in after implementation)*
