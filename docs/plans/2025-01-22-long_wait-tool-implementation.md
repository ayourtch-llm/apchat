# Long Wait Tool Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a `long_wait` tool that allows AI agents to pause execution during long reasoning chains, with exponential backoff progress updates via MSPC and user cancellation support.

**Architecture:** Single new tool module (`long_wait.rs`) integrated into the existing tool registry, using MSPC framework for broadcasting progress updates and listening for interrupt signals. ToolContext will be extended to provide MSPC channel access.

**Tech Stack:** Rust, async/await (tokio), apchat-toolcore, apchat-mspc, async_trait

---

## Task 1: Extend ToolContext with MSPC Channel Access

**Files:**
- Modify: `crates/apchat-toolcore/src/tool_context.rs`

**Step 1: Add MSPC channel fields to ToolContext struct**

Find the `ToolContext` struct definition and add the following fields:

```rust
use tokio::sync::mpsc::Sender;
use std::sync::Arc;
use tokio::sync::Mutex;

// Add these fields to ToolContext struct
pub struct ToolContext {
    pub work_dir: PathBuf,
    // NEW FIELDS:
    pub mspc_sender: Sender<apchat_mspc::MspcMessage>,
    pub mspc_receiver: Arc<Mutex<tokio::sync::mpsc::Receiver<apchat_mspc::MspcMessage>>>,
    // ... existing fields
}
```

**Step 2: Update ToolContext constructor to accept MSPC parameters**

Modify the `ToolContext::new()` method (or equivalent constructor) to accept the new parameters.

**Step 3: Update all ToolContext instantiation sites**

Search for `ToolContext::new(` or `ToolContext {` across the codebase and update each call site to pass the new MSPC parameters.

**Step 4: Verify compilation**

Run: `cargo build --release 2>&1 | head -50`

Expected: Compilation may fail with "missing fields" errors - we'll fix these in subsequent tasks.

**Step 5: Commit**

```bash
git add crates/apchat-toolcore/src/tool_context.rs
git commit -m "feat(toolcore): add MSPC channel fields to ToolContext"
```

---

## Task 2: Create LongWaitTool Module Skeleton

**Files:**
- Create: `crates/apchat-tools/src/long_wait.rs`
- Modify: `crates/apchat-tools/src/lib.rs`

**Step 1: Create the long_wait.rs file with basic structure**

(See design document for complete code)

**Step 2: Export the new module in lib.rs**

Add to `crates/apchat-tools/src/lib.rs`:

```rust
pub mod long_wait;
pub use long_wait::*;
```

**Step 3: Verify compilation**

Run: `cargo build --release 2>&1 | grep -A5 "long_wait"`

**Step 4: Commit**

```bash
git add crates/apchat-tools/src/long_wait.rs crates/apchat-tools/src/lib.rs
git commit -m "feat(tools): add LongWaitTool skeleton"
```

---

## Task 3: Implement Core Wait Logic

**Files:**
- Modify: `crates/apchat-tools/src/long_wait.rs`

**Step 1: Implement the execute method with timing loop**

(See design document for complete code)

**Step 2: Add helper function to check for interrupts**

**Step 3: Add helper function to send progress updates**

**Step 4: Verify compilation**

Run: `cargo build --release 2>&1 | grep -E "(error|warning.*long_wait)" | head -20`

**Step 5: Commit**

```bash
git add crates/apchat-tools/src/long_wait.rs
git commit -m "feat(tools): implement core wait logic for LongWaitTool"
```

---

## Task 4: Write Unit Tests

**Files:**
- Create: `crates/apchat-tools/tests/long_wait_tests.rs`

**Step 1: Create test file with basic tests**

Test coverage:
- Zero duration completes immediately
- Duration exceeds maximum returns error
- Normal duration completes successfully
- Custom message formatting

**Step 2: Run tests**

Run: `cargo test --package apchat-tools long_wait 2>&1 | tail -30`

**Step 3: Commit**

```bash
git add crates/apchat-tools/tests/long_wait_tests.rs
git commit -m "test(tools): add unit tests for LongWaitTool"
```

---

## Task 5: Wire Up ToolContext Initialization Sites

**Files:**
- Modify: `apchat-main/src/config/mod.rs`
- Modify: `apchat-main/src/app/repl.rs`
- Modify: Test files that create ToolContext

**Step 1: Update initialize_tool_registry to include LongWaitTool**

In `apchat-main/src/config/mod.rs`:

```rust
registry.register_with_categories(LongWaitTool, vec!["system".to_string()]);
```

**Step 2: Find and update ToolContext instantiation in repl.rs**

**Step 3: Update test files**

**Step 4: Verify compilation**

Run: `cargo build --release 2>&1 | tail -30`

**Step 5: Run all tests**

Run: `cargo test --release 2>&1 | tail -50`

**Step 6: Commit**

```bash
git add apchat-main/src/config/mod.rs apchat-main/src/app/repl.rs
git commit -m "feat(main): wire up ToolContext MSPC channels and register LongWaitTool"
```

---

## Task 6: Add Integration Test with MSPC

**Files:**
- Create: `crates/apchat-tools/tests/long_wait_integration_tests.rs`

**Step 1: Create integration test file**

Test coverage:
- Progress updates sent via MSPC
- Interrupt signal cancels wait

**Step 2: Run integration tests**

Run: `cargo test --package apchat-tools test_progress_updates 2>&1 | tail -30`

**Step 3: Commit**

```bash
git add crates/apchat-tools/tests/long_wait_integration_tests.rs
git commit -m "test(tools): add integration tests for LongWaitTool MSPC behavior"
```

---

## Task 7: Verify End-to-End Functionality

**Files:**
- Manual testing

**Step 1: Build the application**

Run: `cargo build --release`

Expected: Clean build.

**Step 2: Start APChat in interactive mode**

Run: `cargo run --release -- --stream --interactive`

**Step 3: Test the tool via AI interaction**

Ask the AI: "Please wait for 5 seconds while telling me you're waiting for a test build"

Expected: See progress updates every 1s, 2s, 4s...

**Step 4: Test cancellation**

Send interrupt signal (Ctrl+C or equivalent) during a wait.

Expected: Graceful cancellation with message.

**Step 5: Final commit**

```bash
git add -A
git commit -m "feat: complete long_wait tool implementation with MSPC integration"
```

---

## Verification Checklist

- [ ] All tests pass: `cargo test --release`
- [ ] Build succeeds: `cargo build --release`
- [ ] Tool appears in registry: `list_files` shows it's available
- [ ] Progress updates visible in terminal
- [ ] Cancellation works correctly
- [ ] Duration validation works (0, normal, >600)
- [ ] Custom message formatting works
