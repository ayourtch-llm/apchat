# Configuration Settings for Multi-Agent Mode

## How is Multi-Agent Mode Configured?

Multi-agent mode is configured through **ONE single CLI flag** and has **NO environment variables, feature flags, or configuration file settings**.

## The Single Control Point

### `--agents` CLI Flag

**Location:** `apchat-main/src/cli.rs` line 30

```rust
/// Enable multi-agent system for specialized task handling
#[arg(long, action = clap::ArgAction::SetTrue)]
pub agents: bool,
```

**Usage:**
```bash
# Enable multi-agent mode (planner-first architecture)
cargo run -- --agents -i

# Run single-agent mode (default, no flag needed)
cargo run -- -i
```

**Default Value:** `false` (single-agent mode is default)

## What Does NOT Control Multi-Agent Mode

### Environment Variables
There are NO environment variables that control multi-agent mode.

### Feature Flags
Multi-agent mode is NOT behind a Cargo feature flag. The `apchat-agents` crate is a **direct dependency**, not optional.

### Configuration Files
There are NO configuration files that control multi-agent mode.

---

## Code Locations

### Primary Check Points

| Component | File | Lines |
|-----------|------|-------|
| CLI Argument | `apchat-main/src/cli.rs` | 30 |
| Task Mode Handler | `apchat-main/src/main.rs` | 45-64 |
| Agent Coordinator Field | `apchat-main/src/apchat.rs` | 45 |
| use_agents Field | `apchat-main/src/apchat.rs` | 46 |
| Agent Initialization | `apchat-main/src/apchat.rs` | 198-215 |
| Agent Processing Method | `apchat-main/src/apchat.rs` | 302-342 |
| Agent System Init | `apchat-main/src/config/mod.rs` | 189-240 |
| Task Mode Logic | `apchat-main/src/app/task.rs` | 44-47 |
| Subagent Mode Logic | `apchat-main/src/app/subagent.rs` | 70-73 |
| REPL Inference | `apchat-main/src/app/repl/inference.rs` | 35-59 |
| Web Server Handler | `apchat-main/src/web/routes.rs` | 705-746 |

---

## Critical: Type Re-exports

The `apchat-agents` crate **re-exports** types from `apchat-llm-api`. These types are NOT defined in `apchat-agents`:

| Type | Defined In |
|------|------------|
| `ToolDefinition` | `apchat-llm-api/src/client/mod.rs:100` |
| `ChatMessage` | `apchat-llm-api/src/client/mod.rs:12` |
| `ToolCall` | `apchat-llm-api/src/client/mod.rs:26` |
| `FunctionCall` | `apchat-llm-api/src/client/mod.rs:33` |
| `LlmClient` | `apchat-llm-api/src/client/mod.rs:65` |
| `GroqLlmClient` | `apchat-llm-api/src/client/groq.rs` |

The re-export in `apchat-agents/src/agent.rs:7-10`:
```rust
pub use apchat_llm_api::{
    LlmClient, ChatMessage, ToolCall, FunctionCall, LlmResponse,
    TokenUsage, ToolDefinition, StreamingChunk,
};
```

**Important:** When removing `apchat-agents`, change import paths to `apchat_llm_api`, NOT relocate types.

---

## Files Using apchat_agents

All locations that import from `apchat_agents`:

| File | Import/Usage |
|------|--------------|
| `apchat-main/src/config/mod.rs:5` | `PlanningCoordinator, AgentFactory` |
| `apchat-main/src/apchat.rs:14` | `PlanningCoordinator, GroqLlmClient, ChatMessage, ExecutionContext` |
| `apchat-main/src/api/client.rs:10` | `ToolDefinition, ChatMessage` |
| `apchat-main/src/api/client.rs:226-228` | `apchat_agents::ToolCall, FunctionCall` (inline) |
| `apchat-main/src/api/streaming.rs:7` | `ToolDefinition, ChatMessage` |
| `apchat-main/src/api/streaming.rs:447-449` | `apchat_agents::ToolCall, FunctionCall` (inline) |
| `apchat-main/src/chat/session.rs:39-51` | `ProgressEvaluator, GroqLlmClient, ToolCallInfo` |
| `apchat-main/src/chat/session.rs:285` | `ToolCallSummary` (inline) |
| `apchat-main/src/chat/session.rs:510` | `ToolCallInfo` (inline) |
| `apchat-main/src/apchat.rs:327-329` | `apchat_agents::agent::ToolCall, FunctionCall` (inline) |

---

## ProgressEvaluator

The `ProgressEvaluator` is used in the **main chat loop** (`session.rs`), not just multi-agent mode.

**Location:** `crates/apchat-agents/src/progress_evaluator.rs`

**Types:**
- `ProgressEvaluator` - Main evaluator class
- `ProgressEvaluation` - Evaluation result
- `ToolCallInfo` - Tool call tracking
- `ToolCallSummary` - Summary for evaluation

**Recommendation:** When removing multi-agent mode, move `ProgressEvaluator` to a new `apchat-progress` crate to preserve this functionality.

---

## Removal Options

### Option 1: Keep Code, Never Use Flag (Simplest)

Simply never use the `--agents` flag. The application defaults to single-agent mode.

### Option 2: Complete Removal

See `docs/MULTI_AGENT_REMOVAL_GUIDE.md` for step-by-step instructions.

**Summary of removal:**
1. Create `apchat-progress` crate (preserve ProgressEvaluator)
2. Remove `apchat-agents` dependency
3. Change imports from `apchat_agents` to `apchat_llm_api` for types
4. Change imports to `apchat_progress` for ProgressEvaluator
5. Remove agent-specific code paths
6. Remove `--agents` CLI flag
7. Delete `crates/apchat-agents/` and `agents/` directories

---

## Verification After Removal

```bash
# Build succeeds
cargo build --release

# --agents flag is unrecognized
cargo run -- --agents -i
# Expected: error: unexpected argument '--agents' found

# Single-agent mode works
cargo run -- -i

# Tool use works (CRITICAL)
# Test file operations, search, commands in interactive mode
```

---

## Summary

| Aspect | Value |
|--------|-------|
| **Control Mechanism** | `--agents` CLI flag |
| **Default Value** | `false` (disabled) |
| **Environment Variable** | None |
| **Feature Flag** | None |
| **Config File Setting** | None |
| **Agent Crate** | `crates/apchat-agents` |
| **Type Definitions** | In `apchat-llm-api`, re-exported by `apchat-agents` |
| **ProgressEvaluator** | In `apchat-agents`, used by single-agent chat loop |
