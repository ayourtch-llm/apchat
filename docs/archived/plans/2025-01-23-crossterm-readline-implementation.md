# Crossterm Readline Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace rustyline with custom crossterm-based readline implementation maintaining full feature parity including Emacs-style editing, history navigation, reverse search, and MPSC signal integration.

**Architecture:** Create new `apchat-vty/src/readline.rs` module with semi-raw terminal mode (raw input, normal output), 100ms timeout polling for MPSC signals, and full readline editing capabilities. Integrate with existing singleton pattern and history system.

**Tech Stack:** crossterm 0.28, nix (for termios), existing apchat-mspc and JSONL history

---

## Task 1: Add crossterm dependency to apchat-vty

**Files:**
- Modify: `crates/apchat-vty/Cargo.toml`

**Step 1: Add crossterm dependency**

Edit `crates/apchat-vty/Cargo.toml` and add to dependencies section:

```toml
[dependencies]
crossterm = "0.28"
```

**Step 2: Verify dependency resolution**

Run: `cargo check -p apchat-vty`
Expected: Successfully resolves crossterm dependency

**Step 3: Commit**

```bash
git add crates/apchat-vty/Cargo.toml
git commit -m "feat: add crossterm dependency for readline implementation"
```

---

## Task 2: Create basic Readline struct with terminal mode management

**Files:**
- Create: `crates/apchat-vty/src/readline.rs`
- Modify: `crates/apchat-vty/src/lib.rs`

**Step 1: Write the failing test**

Create `crates/apchat-vty/src/readline.rs` with basic struct and tests.

**Step 2: Export module from lib.rs**

Edit `crates/apchat-vty/src/lib.rs`:
```rust
pub mod readline;
pub use readline::Readline;
```

**Step 3: Run tests to verify they pass**

Run: `cargo test -p apchat-vty readline --lib`

**Step 4: Commit**

```bash
git add crates/apchat-vty/src/readline.rs crates/apchat-vty/src/lib.rs
git commit -m "feat: add basic Readline struct with terminal mode management"
```

---

## Task 3: Add history management to Readline

**Files:**
- Modify: `crates/apchat-vty/src/readline.rs`

**Step 1: Write failing test for history operations**

**Step 2: Run test to verify it fails**

**Step 3: Implement history management**

Add fields and methods for history navigation.

**Step 4: Run tests to verify they pass**

**Step 5: Commit**

```bash
git add crates/apchat-vty/src/readline.rs
git commit -m "feat: add history navigation to Readline"
```

---

## Task 4: Implement screen rendering (redraw function)

**Files:**
- Modify: `crates/apchat-vty/src/readline.rs`

**Step 1: Implement redraw function**

**Step 2: Manual test**

**Step 3: Commit**

```bash
git add crates/apchat-vty/src/readline.rs
git commit -m "feat: implement screen rendering for Readline"
```

---

## Task 5: Implement basic key event handlers

**Files:**
- Modify: `crates/apchat-vty/src/readline.rs`

**Step 1: Write tests for key handlers**

**Step 2: Run tests to verify they fail**

**Step 3: Implement key handlers**

Add methods: `handle_char`, `handle_backspace`, `handle_delete`, `handle_left`, `handle_right`, `handle_home`, `handle_end`

**Step 4: Run tests to verify they pass**

**Step 5: Commit**

```bash
git add crates/apchat-vty/src/readline.rs
git commit -m "feat: implement basic key event handlers"
```

---

## Task 6: Implement main readline loop with event polling

**Files:**
- Modify: `crates/apchat-vty/src/readline.rs`

**Step 1: Add result types**

```rust
use std::time::Duration;

pub enum ReadlineResult {
    Input(String),
    Eof,
    Interrupt,
}

enum KeyResult {
    Continue,
    Redraw,
    Return(ReadlineResult),
}
```

**Step 2: Implement main readline loop**

Add `readline()` and `handle_key_event()` methods with 100ms timeout polling.

**Step 3: Manual test**

**Step 4: Commit**

```bash
git add crates/apchat-vty/src/readline.rs
git commit -m "feat: implement main readline loop with event polling"
```

---

## Task 7: Integrate with existing ReadlineInstance singleton

**Files:**
- Modify: `apchat-main/src/chat/readline_instance.rs`

**Step 1: Update imports**

Replace rustyline with apchat_vty::Readline

**Step 2: Update singleton type**

Change to use crossterm Readline

**Step 3: Update method signatures**

Update `get()`, `readline()`, and `add_history()` methods

**Step 4: Run tests**

Run: `cargo test -p apchat readline_instance --lib`

**Step 5: Build and check**

Run: `cargo build -p apchat`

**Step 6: Commit**

```bash
git add apchat-main/src/chat/readline_instance.rs
git commit -m "refactor: integrate crossterm Readline with ReadlineInstance singleton"
```

---

## Task 8: Update history loading for new API

**Files:**
- Modify: `apchat-main/src/chat/readline_history.rs`

**Step 1: Update load_and_add_to_editor function**

```rust
pub fn load_and_add_to_editor(rl: &mut apchat_vty::Readline) -> Result<()> {
    let history = load_history(None)?;
    for entry in history.get_entries() {
        rl.add_history_entry(&entry.command);
    }
    Ok(())
}
```

**Step 2: Test**

Run: `cargo build -p apchat`

**Step 3: Commit**

```bash
git add apchat-main/src/chat/readline_history.rs
git commit -m "refactor: update history loading for crossterm Readline"
```

---

## Task 9: Remove rustyline dependency

**Files:**
- Modify: `apchat-main/Cargo.toml`

**Step 1: Remove rustyline dependency**

Delete: `rustyline = "14.0"`

**Step 2: Verify build**

Run: `cargo build --release`

**Step 3: Test REPL functionality**

Run: `cargo run --release -- --stream --interactive`

**Step 4: Commit**

```bash
git add apchat-main/Cargo.toml
git commit -m "chore: remove rustyline dependency"
```

---

## Task 10: Implement Ctrl-R reverse search

**Files:**
- Modify: `crates/apchat-vty/src/readline.rs`

**Step 1: Add search state to Readline struct**

Add: `mode`, `search_pattern`, `search_matches`, `search_match_index`, `original_line`

**Step 2: Implement search mode handlers**

Add: `enter_search_mode()`, `exit_search_mode()`, `update_search()`, `cycle_search_match()`

**Step 3: Update redraw for search mode**

Add: `redraw_search()` method

**Step 4: Update handle_key_event for search mode**

Split into: `handle_normal_mode()` and `handle_search_mode()`

**Step 5: Manual test**

**Step 6: Commit**

```bash
git add crates/apchat-vty/src/readline.rs
git commit -m "feat: implement Ctrl-R reverse search"
```

---

## Task 11: Add MPSC signal checking to readline loop

**Files:**
- Modify: `crates/apchat-vty/src/readline.rs`
- Modify: `crates/apchat-vty/Cargo.toml`

**Step 1: Add apchat-mspc dependency**

**Step 2: Update ReadlineResult to include signals**

```rust
pub enum ReadlineResult {
    Input(String),
    Eof,
    Interrupt,
    Signal(MspcMessage),
}
```

**Step 3: Update readline signature**

Accept `Option<&MspcChannel>` parameter

**Step 4: Implement MPSC checking in timeout**

Check `channel.try_recv()` on timeout

**Step 5: Update call sites**

Pass None for now in `readline_instance.rs`

**Step 6: Build and test**

Run: `cargo build -p apchat-vty`

**Step 7: Commit**

```bash
git add crates/apchat-vty/src/readline.rs crates/apchat-vty/Cargo.toml
git commit -m "feat: add MPSC signal checking to readline loop"
```

---

## Task 12: Update REPL to use MPSC-aware readline

**Files:**
- Modify: `apchat-main/src/app/repl.rs`

**Step 1: Update readline call to pass MPSC channel**

Modify the spawn_blocking call to pass the MPSC channel

**Step 2: Update result handling**

Handle `ReadlineResult::Signal(msg)` variant

**Step 3: Test**

Run: `cargo run --release -- --stream --interactive`

**Step 4: Commit**

```bash
git add apchat-main/src/app/repl.rs
git commit -m "refactor: update REPL to use MPSC-aware readline"
```

---

## Task 13: Implement advanced editing features

**Files:**
- Modify: `crates/apchat-vty/src/readline.rs`

**Step 1: Add kill ring to Readline struct**

**Step 2: Implement kill ring operations**

Add: `kill_to_end()`, `kill_to_start()`, `kill_word()`, `yank()`

**Step 3: Add word navigation**

Add: `handle_word_left()`, `handle_word_right()`

**Step 4: Update handle_key_event**

Add Ctrl-K, Ctrl-U, Ctrl-W, Ctrl-Y, Ctrl-Left, Ctrl-Right

**Step 5: Manual test**

**Step 6: Commit**

```bash
git add crates/apchat-vty/src/readline.rs
git commit -m "feat: implement advanced editing (kill ring, word navigation)"
```

---

## Task 14: Final testing and cleanup

**Files:**
- Various test files

**Step 1: Run all tests**

Run: `cargo test --all`

**Step 2: Manual integration testing**

Test all features:
- Basic input and editing
- History navigation (up/down)
- Ctrl-R reverse search
- Ctrl-C interrupt
- Ctrl-D EOF
- Kill ring operations
- Word navigation
- Unicode handling

**Step 3: Update any remaining references**

Search for remaining rustyline references in code/comments

**Step 4: Update documentation**

Update any docs that reference rustyline

**Step 5: Final commit**

```bash
git add -A
git commit -m "feat: complete crossterm readline migration"
```

---

## Verification Steps

After completing all tasks:

1. **Build verification**: `cargo build --release` succeeds
2. **Test verification**: `cargo test --all` passes
3. **Manual verification**: REPL works with all features
4. **Dependency verification**: No rustyline references remain
5. **History verification**: JSONL history loads/saves correctly

---

## Notes

- The implementation uses "semi-raw" mode: raw input, normal output (like rustyline)
- This ensures `\n` → `\r\n` conversion still works
- No mouse capture is enabled (allows text selection)
- 100ms timeout balances responsiveness and CPU usage
- Kill ring follows Emacs pattern (max 16 entries)
- Unicode handling uses `.chars().count()` for cursor positioning
