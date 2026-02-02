# Quick Reference: Multi-Agent Mode Removal

## Critical Warning

**DO NOT** relocate `ToolDefinition`, `ChatMessage`, `ToolCall`, or `FunctionCall` types.
They are **defined in `apchat-llm-api`** and merely re-exported by `apchat-agents`.
Change import paths, not type locations.

---

## Import Path Changes (Copy-Paste Ready)

### `apchat-main/src/api/client.rs` (line 10)
```rust
// FROM:
use apchat_agents::{ToolDefinition, ChatMessage};
// TO:
use apchat_llm_api::{ToolDefinition, ChatMessage};
```

### `apchat-main/src/api/client.rs` (lines 226-228)
```rust
// FROM:
calls.into_iter().map(|call| apchat_agents::ToolCall {
    id: call.id,
    function: apchat_agents::FunctionCall {
// TO:
calls.into_iter().map(|call| apchat_llm_api::ToolCall {
    id: call.id,
    function: apchat_llm_api::FunctionCall {
```

### `apchat-main/src/api/streaming.rs` (line 7)
```rust
// FROM:
use apchat_agents::{ToolDefinition, ChatMessage};
// TO:
use apchat_llm_api::{ToolDefinition, ChatMessage};
```

### `apchat-main/src/api/streaming.rs` (lines 447-449)
```rust
// FROM:
calls.into_iter().map(|call| apchat_agents::ToolCall {
    id: call.id,
    function: apchat_agents::FunctionCall {
// TO:
calls.into_iter().map(|call| apchat_llm_api::ToolCall {
    id: call.id,
    function: apchat_llm_api::FunctionCall {
```

### `apchat-main/src/chat/session.rs` (lines 39-40)
```rust
// FROM:
let mut progress_evaluator = Some(apchat_agents::progress_evaluator::ProgressEvaluator::new(
    std::sync::Arc::new(apchat_agents::GroqLlmClient::new(
// TO:
let mut progress_evaluator = Some(apchat_progress::ProgressEvaluator::new(
    std::sync::Arc::new(apchat_llm_api::client::groq::GroqLlmClient::new(
```

### `apchat-main/src/chat/session.rs` (line 51)
```rust
// FROM:
let mut tool_call_history: Vec<apchat_agents::progress_evaluator::ToolCallInfo> = Vec::new();
// TO:
let mut tool_call_history: Vec<apchat_progress::ToolCallInfo> = Vec::new();
```

### `apchat-main/src/chat/session.rs` (line 285)
```rust
// FROM:
let summary = apchat_agents::progress_evaluator::ToolCallSummary {
// TO:
let summary = apchat_progress::ToolCallSummary {
```

### `apchat-main/src/chat/session.rs` (line 510)
```rust
// FROM:
let call_info = apchat_agents::progress_evaluator::ToolCallInfo {
// TO:
let call_info = apchat_progress::ToolCallInfo {
```

---

## Type Definitions Location

| Type | Defined In | NOT In |
|------|------------|--------|
| `ToolDefinition` | `apchat-llm-api/src/client/mod.rs:100` | ~~apchat-agents~~ |
| `ChatMessage` | `apchat-llm-api/src/client/mod.rs:12` | ~~apchat-agents~~ |
| `ToolCall` | `apchat-llm-api/src/client/mod.rs:26` | ~~apchat-agents~~ |
| `FunctionCall` | `apchat-llm-api/src/client/mod.rs:33` | ~~apchat-agents~~ |
| `GroqLlmClient` | `apchat-llm-api/src/client/groq.rs` | ~~apchat-agents~~ |
| `ProgressEvaluator` | Move to `apchat-progress` | ~~apchat-agents~~ |
| `ToolCallInfo` | Move to `apchat-progress` | ~~apchat-agents~~ |
| `ToolCallSummary` | Move to `apchat-progress` | ~~apchat-agents~~ |

---

## Files Requiring Changes

### Critical for Tool Use (Fix These First!)
1. `apchat-main/src/api/client.rs` - Import + inline types
2. `apchat-main/src/api/streaming.rs` - Import + inline types

### Critical for Chat Loop
3. `apchat-main/src/chat/session.rs` - ProgressEvaluator references

### Agent-Specific Code Removal
4. `apchat-main/src/cli.rs` - Remove `agents` field
5. `apchat-main/src/apchat.rs` - Remove fields, imports, methods
6. `apchat-main/src/config/mod.rs` - Remove imports, function
7. `apchat-main/src/app/task.rs` - Remove agent check
8. `apchat-main/src/app/subagent.rs` - Remove agent check
9. `apchat-main/src/app/repl/inference.rs` - Remove agent check
10. `apchat-main/src/web/routes.rs` - Remove agent handling
11. `apchat-main/src/main.rs` - Simplify task mode

### Dependencies
12. `Cargo.toml` (workspace) - Remove `apchat-agents`, add `apchat-progress`
13. `apchat-main/Cargo.toml` - Remove `apchat-agents`, add `apchat-progress`

---

## New Crate: `apchat-progress`

Create before removing `apchat-agents`:

```bash
mkdir -p crates/apchat-progress/src
```

Copy `crates/apchat-agents/src/progress_evaluator.rs` to `crates/apchat-progress/src/lib.rs`

Update `crate::GroqLlmClient` to `dyn apchat_llm_api::LlmClient`

---

## Verification Commands

```bash
# 1. Build
cargo build --release

# 2. Check --agents is gone
cargo run -- --agents -i 2>&1 | grep -q "unexpected argument" && echo "OK"

# 3. Test tool use
cargo run -- -i
# Then test: /help, file operations, search operations
```

---

## Why Tool Use Breaks

The conversion in `client.rs:221-247` and `streaming.rs:441-468`:

```
Message (old) → ChatMessage (new)
Tool (old)    → ToolDefinition (new)
```

If `apchat_agents::ToolCall` references are broken, tools don't convert correctly → API calls fail → no tool use.

---

## Full Guide

See `docs/MULTI_AGENT_REMOVAL_GUIDE.md` for complete step-by-step instructions.
