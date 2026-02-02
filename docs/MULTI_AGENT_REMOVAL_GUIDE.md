# Multi-Agent Mode Removal Guide

This document provides accurate, step-by-step instructions to remove multi-agent mode from APChat while preserving all single-agent functionality including tool use.

## Summary

Multi-agent mode is controlled solely through the `--agents` CLI flag. The agent system is implemented in `crates/apchat-agents`, which is a direct dependency.

**CRITICAL:** Many types used for LLM API calls (`ToolDefinition`, `ChatMessage`, `ToolCall`, `FunctionCall`) are **defined in `apchat-llm-api`** and merely re-exported by `apchat-agents`. The fix is to change import paths, NOT relocate types.

## Pre-Removal: Create `apchat-progress` Crate

The `ProgressEvaluator` feature is used in the main chat loop (single-agent mode) and should be preserved by moving it to a new crate.

### Step 0.1: Create the new crate

```bash
mkdir -p crates/apchat-progress/src
```

### Step 0.2: Create `crates/apchat-progress/Cargo.toml`

```toml
[package]
name = "apchat-progress"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1.0"
async-trait = "0.1"
colored = "2.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
apchat-llm-api = { path = "../apchat-llm-api" }
```

### Step 0.3: Move `progress_evaluator.rs`

Copy `crates/apchat-agents/src/progress_evaluator.rs` to `crates/apchat-progress/src/lib.rs` and update the import:

```rust
// Change this line:
llm_client: Option<std::sync::Arc<crate::GroqLlmClient>>,

// To:
llm_client: Option<std::sync::Arc<dyn apchat_llm_api::LlmClient>>,
```

Also update the `new()` function signature:

```rust
pub fn new(
    llm_client: std::sync::Arc<dyn apchat_llm_api::LlmClient>,
    min_confidence: f32,
    eval_interval: u32,
) -> Self {
```

### Step 0.4: Add to workspace

In root `Cargo.toml`, add to members:
```toml
members = [
    # ... existing members
    "crates/apchat-progress",
]
```

### Step 0.5: Add dependency to `apchat-main/Cargo.toml`

```toml
apchat-progress = { path = "../crates/apchat-progress" }
```

---

## Complete Removal Checklist

### 1. CLI Argument

**File:** `apchat-main/src/cli.rs`

**Remove:**
```rust
/// Enable multi-agent system for specialized task handling
#[arg(long, action = clap::ArgAction::SetTrue)]
pub agents: bool,
```

### 2. Dependency Removal

**File:** `apchat-main/Cargo.toml`

**Remove:**
```toml
apchat-agents = { path = "../crates/apchat-agents" }
```

**File:** `Cargo.toml` (workspace root)

**Remove from members:**
```toml
"crates/apchat-agents",
```

### 3. Main Entry Point

**File:** `apchat-main/src/main.rs`

**Change** the task mode handling (around lines 45-64) from:
```rust
if let Some(task_text) = cli.task.clone() {
    if !cli.agents {
        return run_subagent_mode(...).await;
    } else {
        return run_task_mode(...).await;
    }
}
```

**To:**
```rust
if let Some(task_text) = cli.task.clone() {
    return run_subagent_mode(...).await;
}
```

### 4. APChat Struct - Imports

**File:** `apchat-main/src/apchat.rs`

**Remove these imports (lines 14-17):**
```rust
use apchat_agents::{
    PlanningCoordinator, GroqLlmClient,
    ChatMessage, ExecutionContext,
};
```

**Add if needed for other code:**
```rust
use apchat_llm_api::{ChatMessage, ToolCall as LlmToolCall, FunctionCall as LlmFunctionCall};
```

### 5. APChat Struct - Fields

**File:** `apchat-main/src/apchat.rs`

**Remove these fields (around lines 45-46):**
```rust
pub(crate) agent_coordinator: Option<PlanningCoordinator>,
pub(crate) use_agents: bool,
```

### 6. APChat Constructor

**File:** `apchat-main/src/apchat.rs`

**Remove the agent coordinator initialization (lines 198-215):**
```rust
let agent_coordinator = if use_agents {
    match initialize_agent_system(&client_config, &tool_registry, &policy_manager) {
        Ok(coordinator) => Some(coordinator),
        Err(e) => {
            // ... error handling
            None
        }
    }
} else {
    None
};
```

**Remove from struct construction:**
```rust
agent_coordinator,
use_agents,
```

### 7. APChat - process_with_agents Method

**File:** `apchat-main/src/apchat.rs`

**Remove the entire method (around lines 302-342):**
```rust
pub async fn process_with_agents(...) -> Result<String> {
    // ... entire method
}
```

### 8. APChat - Inline Type References

**File:** `apchat-main/src/apchat.rs`

**Change (around lines 327-329):**
```rust
apchat_agents::agent::ToolCall
apchat_agents::agent::FunctionCall
```

**To:**
```rust
apchat_llm_api::ToolCall
apchat_llm_api::FunctionCall
```

### 9. Config Module - Imports and Function

**File:** `apchat-main/src/config/mod.rs`

**Remove these imports (lines 5-7):**
```rust
use apchat_agents::{
    PlanningCoordinator, AgentFactory,
};
```

**Remove the entire `initialize_agent_system` function (lines 189-240).**

### 10. API Client - Imports (CRITICAL FOR TOOL USE)

**File:** `apchat-main/src/api/client.rs`

**Change line 10:**
```rust
use apchat_agents::{ToolDefinition, ChatMessage};
```

**To:**
```rust
use apchat_llm_api::{ToolDefinition, ChatMessage};
```

### 11. API Client - Inline References (CRITICAL FOR TOOL USE)

**File:** `apchat-main/src/api/client.rs`

**Change lines 226-228:**
```rust
calls.into_iter().map(|call| apchat_agents::ToolCall {
    id: call.id,
    function: apchat_agents::FunctionCall {
```

**To:**
```rust
calls.into_iter().map(|call| apchat_llm_api::ToolCall {
    id: call.id,
    function: apchat_llm_api::FunctionCall {
```

### 12. API Streaming - Imports (CRITICAL FOR TOOL USE)

**File:** `apchat-main/src/api/streaming.rs`

**Change line 7:**
```rust
use apchat_agents::{ToolDefinition, ChatMessage};
```

**To:**
```rust
use apchat_llm_api::{ToolDefinition, ChatMessage};
```

### 13. API Streaming - Inline References (CRITICAL FOR TOOL USE)

**File:** `apchat-main/src/api/streaming.rs`

**Change lines 447-449:**
```rust
calls.into_iter().map(|call| apchat_agents::ToolCall {
    id: call.id,
    function: apchat_agents::FunctionCall {
```

**To:**
```rust
calls.into_iter().map(|call| apchat_llm_api::ToolCall {
    id: call.id,
    function: apchat_llm_api::FunctionCall {
```

### 14. Chat Session - Progress Evaluator (CRITICAL)

**File:** `apchat-main/src/chat/session.rs`

**Change these references:**

Line 39-40:
```rust
let mut progress_evaluator = Some(apchat_agents::progress_evaluator::ProgressEvaluator::new(
    std::sync::Arc::new(apchat_agents::GroqLlmClient::new(
```

**To:**
```rust
let mut progress_evaluator = Some(apchat_progress::ProgressEvaluator::new(
    std::sync::Arc::new(apchat_llm_api::client::groq::GroqLlmClient::new(
```

Line 51:
```rust
let mut tool_call_history: Vec<apchat_agents::progress_evaluator::ToolCallInfo> = Vec::new();
```

**To:**
```rust
let mut tool_call_history: Vec<apchat_progress::ToolCallInfo> = Vec::new();
```

Line 285:
```rust
let summary = apchat_agents::progress_evaluator::ToolCallSummary {
```

**To:**
```rust
let summary = apchat_progress::ToolCallSummary {
```

Line 510:
```rust
let call_info = apchat_agents::progress_evaluator::ToolCallInfo {
```

**To:**
```rust
let call_info = apchat_progress::ToolCallInfo {
```

### 15. Task Mode Logic

**File:** `apchat-main/src/app/task.rs`

**Remove the agent check (around lines 44-47):**
```rust
if chat.use_agents && chat.agent_coordinator.is_some() {
    match chat.process_with_agents(&task_text, None).await {
        // ...
    }
}
```

**Always use direct chat:**
```rust
match chat.chat(&mut chat, &task_text, None).await {
    // ...
}
```

### 16. Subagent Mode Logic

**File:** `apchat-main/src/app/subagent.rs`

**Change lines 70-73:**
```rust
let result = if cli.agents && subagent.agent_coordinator.is_some() {
    subagent.process_with_agents(&task_text, None).await
} else {
    crate::chat::session::chat(&mut subagent, &task_text, None).await
};
```

**To:**
```rust
let result = crate::chat::session::chat(&mut subagent, &task_text, None).await;
```

### 17. REPL Inference

**File:** `apchat-main/src/app/repl/inference.rs`

**Remove the agent check (around lines 35-59):**
```rust
if chat.use_agents && chat.agent_coordinator.is_some() {
    match chat.process_with_agents(input, Some(cancel_token.clone())) {
        // ...
    }
}
```

**Always use direct chat.**

### 18. Web Server Routes

**File:** `apchat-main/src/web/routes.rs`

**Remove (around lines 705-706):**
```rust
let use_agents = apchat.use_agents;
```

**Remove the agent handling (around lines 730-746):**
```rust
if use_agents {
    match session.apchat.lock().await
        .process_with_agents(&content, None)
        .await
    {
        // ...
    }
}
```

**Always use direct chat.**

### 19. Test Files

**File:** `apchat-main/src/chat/tests.rs`

**Remove:**
```rust
use_agents: false,
agent_coordinator: None,
```

### 20. Agent Configuration Files

**Remove the entire directory:**
```bash
rm -rf agents/
```

### 21. Agent Crate

**Remove the entire directory:**
```bash
rm -rf crates/apchat-agents/
```

---

## Type Reference Summary

These types are **defined in `apchat-llm-api`**, NOT `apchat-agents`:

| Type | Definition Location |
|------|---------------------|
| `ChatMessage` | `apchat-llm-api/src/client/mod.rs:12` |
| `ToolCall` | `apchat-llm-api/src/client/mod.rs:26` |
| `FunctionCall` | `apchat-llm-api/src/client/mod.rs:33` |
| `ToolDefinition` | `apchat-llm-api/src/client/mod.rs:100` |
| `LlmClient` | `apchat-llm-api/src/client/mod.rs:65` |
| `LlmResponse` | `apchat-llm-api/src/client/mod.rs:84` |
| `TokenUsage` | `apchat-llm-api/src/client/mod.rs:91` |
| `StreamingChunk` | `apchat-llm-api/src/client/mod.rs:55` |
| `GroqLlmClient` | `apchat-llm-api/src/client/groq.rs` |
| `AnthropicLlmClient` | `apchat-llm-api/src/client/anthropic.rs` |

The `apchat-agents` crate re-exports these in `agent.rs:7-10`:
```rust
pub use apchat_llm_api::{
    LlmClient, ChatMessage, ToolCall, FunctionCall, LlmResponse,
    TokenUsage, ToolDefinition, StreamingChunk,
};
```

**NEVER** try to relocate these types - just change import paths.

---

## Files Summary

### Files to Modify (14 files):

| File | Changes |
|------|---------|
| `apchat-main/src/cli.rs` | Remove `agents` field |
| `apchat-main/src/main.rs` | Simplify task mode logic |
| `apchat-main/src/apchat.rs` | Remove agent fields, imports, methods; fix inline types |
| `apchat-main/src/config/mod.rs` | Remove agent imports and `initialize_agent_system` |
| `apchat-main/src/api/client.rs` | Change imports from `apchat_agents` to `apchat_llm_api` |
| `apchat-main/src/api/streaming.rs` | Change imports from `apchat_agents` to `apchat_llm_api` |
| `apchat-main/src/chat/session.rs` | Change to use `apchat_progress` |
| `apchat-main/src/app/task.rs` | Remove agent processing logic |
| `apchat-main/src/app/subagent.rs` | Remove agent processing logic |
| `apchat-main/src/app/repl/inference.rs` | Remove agent processing logic |
| `apchat-main/src/web/routes.rs` | Remove agent handling |
| `apchat-main/src/chat/tests.rs` | Remove agent test fields |
| `Cargo.toml` | Remove from workspace, add `apchat-progress` |
| `apchat-main/Cargo.toml` | Remove `apchat-agents`, add `apchat-progress` |

### Files/Directories to Remove:

- `crates/apchat-agents/` (entire directory)
- `agents/` (entire directory)

### Files/Directories to Create:

- `crates/apchat-progress/` (new crate for ProgressEvaluator)

---

## Testing After Removal

```bash
# 1. Clean build
cargo clean
cargo build --release

# 2. Verify --agents flag is gone
cargo run -- --agents -i
# Should print: error: unexpected argument '--agents' found

# 3. Test interactive mode
cargo run -- -i

# 4. Test task mode
cargo run -- --task "hello world"

# 5. Test web server
cargo run -- --web

# 6. Test tool use (CRITICAL)
# In interactive mode, test that tools work:
# - File operations (open_file, write_file, edit_file)
# - Search operations (grep_search, glob_search)
# - System operations (execute_command)
```

---

## Why Tool Use Breaks Without Correct Fixes

The `call_api_with_llm_client` and `call_api_streaming_with_llm_client` functions in `client.rs` and `streaming.rs` convert between formats:

1. **Old format** (`apchat_models::Message`) → **New format** (`apchat_llm_api::ChatMessage`)
2. **Old tools** (`apchat_models::Tool`) → **New format** (`apchat_llm_api::ToolDefinition`)

If the imports or inline type references are broken, this conversion fails and:
- Tools aren't sent to the API correctly
- Tool call responses aren't parsed correctly
- The entire tool-calling loop breaks

This is why changing `apchat_agents::ToolCall` to `apchat_llm_api::ToolCall` (and similar) is **critical**.
