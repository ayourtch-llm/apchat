# Issue 113: Create basic Readline struct with terminal mode management

## Summary
Create the foundational `Readline` struct in `crates/apchat-vty/src/readline.rs` with terminal mode management (semi-raw mode: raw input, normal output). This is the core structure that will handle all readline functionality.

## Location
- Create: `crates/apchat-vty/src/readline.rs`
- Modify: `crates/apchat-vty/src/lib.rs`

## Current Behavior
The apchat-vty crate does not have a readline module or struct.

## Expected Behavior
The apchat-vty crate should have a `Readline` struct that:
- Manages terminal mode (enables raw mode on construction, disables on drop)
- Stores the current input line and cursor position
- Implements `new()` constructor
- Implements `Drop` for cleanup
- Has basic tests for terminal mode management

## Impact
This is the foundational structure for all readline functionality. It establishes the basic terminal handling pattern that all features will build upon.

## Implementation Plan

### Step 1: Write the failing test
Create `crates/apchat-vty/src/readline.rs` with:
- `Readline` struct with terminal field
- Constructor that enables raw mode
- Drop implementation that disables raw mode
- Unit tests for terminal mode management

### Step 2: Export module from lib.rs
Edit `crates/apchat-vty/src/lib.rs`:
```rust
pub mod readline;
pub use readline::Readline;
```

### Step 3: Run tests to verify they pass
Run: `cargo test -p apchat-vty readline --lib`

### Step 4: Commit
```bash
git add crates/apchat-vty/src/readline.rs crates/apchat-vty/src/lib.rs
git commit -m "feat: add basic Readline struct with terminal mode management"
```

## Verification Criteria
- [ ] `crates/apchat-vty/src/readline.rs` exists with Readline struct
- [ ] Readline manages terminal mode (raw mode enabled/disabled correctly)
- [ ] Module is exported from lib.rs
- [ ] `cargo test -p apchat-vty readline --lib` passes all tests
- [ ] Changes committed with appropriate message

## References
- Part of larger plan: `docs/plans/2025-01-23-crossterm-readline-implementation.md`
- Task 2 from implementation plan (14 total tasks)
- Follows TDD: Write failing test first, then implement

---
*Created: 2025-01-23*
*Resolved: 2025-01-23*

## Resolution
Successfully implemented the basic Readline struct with terminal mode management.

**Changes Made:**
1. Created `crates/apchat-vty/src/readline.rs` with:
   - `Readline` struct with terminal mode tracking
   - `new()` constructor that enables raw mode using `crossterm::terminal::enable_raw_mode()`
   - `Drop` implementation that disables raw mode
   - Methods: `buffer()`, `cursor()`, `is_raw_mode_enabled()`
   - Comprehensive unit tests (5 tests total)

2. Modified `crates/apchat-vty/src/lib.rs`:
   - Added `pub mod readline;`
   - Added `pub use readline::Readline;`

**Files Modified:**
- `crates/apchat-vty/src/readline.rs` (new file, 177 lines)
- `crates/apchat-vty/src/lib.rs` (added module exports)

**Verification:**
- ✅ All 5 tests pass: `cargo test -p apchat-vty readline --lib`
  - `test_readline_creation` - Readline instance creation
  - `test_raw_mode_enabled_on_creation` - Raw mode enabled on construction
  - `test_raw_mode_disabled_on_drop` - Raw mode disabled on drop
  - `test_initial_state` - Initial buffer and cursor state
  - `test_multiple_readline_instances` - Multiple instance handling
- ✅ Module properly exported from lib.rs

**Commits:**
1. `248a3da` - "feat: add basic Readline struct with terminal mode management"
2. `b14da5b` - "fix: ensure raw mode is enabled on Readline creation"

**Notes:**
- The implementation uses "semi-raw" mode: raw input, normal output (like rustyline)
- Raw mode tracking is done via `raw_mode_enabled` field since `crossterm::terminal::is_raw_mode_enabled()` may return false in non-TTY environments
- Tests follow TDD approach (written first, then implemented)
