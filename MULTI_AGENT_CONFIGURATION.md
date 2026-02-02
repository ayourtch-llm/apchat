# Configuration Settings for Multi-Agent Mode

## Answer: How is Multi-Agent Mode Configured?

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

### ❌ Environment Variables
There are NO environment variables that control multi-agent mode. The application does NOT check for environment variables like:
- `APCHAT_AGENTS` - Does NOT exist
- `APCHAT_MULTI_AGENT` - Does NOT exist
- `APCHAT_ENABLE_AGENTS` - Does NOT exist
- `AGENTS_ENABLED` - Does NOT exist

### ❌ Feature Flags
Multi-agent mode is NOT behind a Cargo feature flag. The `apchat-agents` crate is a **direct dependency**, not optional.

In `apchat-main/Cargo.toml`:
```toml
apchat-agents = { path = "../crates/apchat-agents" }  # No "optional = true"
```

There is NO `[features]` section related to agents.

### ❌ Configuration Files
There are NO configuration files that control multi-agent mode. The application does NOT read from:
- `~/.apchat/config.toml` - Does NOT exist
- `~/.apchatrc` - Does NOT exist
- `policies.toml` - Controls action approvals, NOT multi-agent mode
- `.env` - Contains API keys, NOT agent control

### ❌ Build-Time Configuration
Multi-agent mode is NOT controlled by build-time settings. It's a **runtime-only** configuration.

## How the --agents Flag is Used

### 1. Entry Point: main.rs

**File:** `apchat-main/src/main.rs` (lines 45-64)

```rust
if let Some(task_text) = cli.task.clone() {
    // Check the --agents flag
    if !cli.agents {
        // Single-agent mode: use subagent mode (isolated task execution)
        return run_subagent_mode(...).await;
    } else {
        // Multi-agent mode: use agent system with coordinator
        return run_task_mode(...).await;
    }
}
```

### 2. APChat Constructor

**File:** `apchat-main/src/apchat.rs` (lines 198-215)

```rust
let agent_coordinator = if use_agents {
    // Initialize agent system: load configs, create coordinator
    match initialize_agent_system(&client_config, &tool_registry, &policy_manager) {
        Ok(coordinator) => Some(coordinator),
        Err(e) => {
            print_heart_yellow(&format!("Failed to initialize agent system: {}", e), true);
            print_heart_yellow(&format!("Falling back to non-agent mode"), true);
            None
        }
    }
} else {
    // No coordinator needed for single-agent mode
    None
};
```

### 3. Task Processing Decision

**File:** `apchat-main/src/app/task.rs` (lines 44-47)

```rust
let response = if chat.use_agents && chat.agent_coordinator.is_some() {
    // Use agent system: planner decomposes task, delegates to specialists
    match chat.process_with_agents(&task_text, None).await {
        Ok(response) => response,
        // ... fallback to single-agent mode on error
    }
} else {
    // Single-agent mode: direct conversation with LLM
    match chat.chat(&mut chat, &task_text, None).await {
        // ... handle response
    }
};
```

### 4. REPL Mode Decision

**File:** `apchat-main/src/app/repl/inference.rs` (lines 35-59)

```rust
if chat.use_agents && chat.agent_coordinator.is_some() {
    // Multi-agent mode with cancellation token support
    match chat.process_with_agents(input, Some(cancel_token.clone())) {
        // ... agent handling
    }
} else {
    // Single-agent mode: direct conversation
    match chat.chat(input) {
        // ... handle response
    }
}
```

### 5. Web Server Decision

**File:** `apchat-main/src/web/routes.rs` (lines 730-746)

```rust
if use_agents {
    // Multi-agent mode: use agent system
    match session.apchat.lock().await
        .process_with_agents(&content, None)
        .await
    {
        // ... agent response handling
    }
} else {
    // Single-agent mode: direct conversation
    match session.apchat.lock().await
        .process_chat(&content)
        .await
    {
        // ... direct response handling
    }
}
```

## What Happens When --agents is Set

### Initialization Phase (One-time)

1. **Load Agent Configurations**
   - Read from `agents/configs/*.json` (7 files)
   - Load embedded configs (from source code)
   - Register agents: planner, code_analyzer, code_reviewer, file_manager, search_specialist, system_operator, terminal_specialist

2. **Create Agent Factory**
   - Register LLM clients for blu_model, grn_model, red_model
   - Each agent can use a different model
   - Skill registry is passed to agents

3. **Initialize PlanningCoordinator**
   - Creates task queue
   - Manages agent dispatch
   - Coordinates task execution

### Execution Phase (Per Request)

1. **Task submitted to PlanningCoordinator**
   - Planner agent receives task
   - Decomposes into subtasks
   - Identifies which agents to use
   - Creates execution plan

2. **Agent Dispatch**
   - Each subtask dispatched to appropriate agent
   - Agents execute with tool access
   - Progress evaluator monitors execution

3. **Result Aggregation**
   - Coordinator collects results from agents
   - Synthesizes final response
   - Returns to user

## What Happens When --agents is NOT Set (Default)

### Initialization Phase

1. **No agent coordinator created**
   - `agent_coordinator` field is `None`
   - `use_agents` field is `false`

2. **Direct LLM client setup**
   - Single LLM instance
   - Full tool registry access
   - No agent overhead

### Execution Phase

1. **Direct conversation**
   - User message sent directly to LLM
   - LLM can call any tool directly
   - No planning or coordination
   - Simpler, faster execution

## Configuration Files Only Used With --agents

These files are ONLY read when `--agents` is set:

1. **Agent Definitions:** `agents/configs/*.json`
   - `planner.json` - Task planning agent
   - `code_analyzer.json` - Code analysis agent
   - `code_reviewer.json` - Code review agent
   - `file_manager.json` - File operations agent
   - `search_specialist.json` - Search agent
   - `system_operator.json` - Command execution agent
   - `terminal_specialist.json` - Terminal management agent

2. **Embedded Agent Configs:**
   - Located in `crates/apchat-agents/src/embedded_configs.rs`
   - Loaded as fallback if filesystem configs don't exist

## What to Remove to Eliminate Multi-Agent Mode

### Minimal Removal (Keep Code, Just Disable)

If you want to keep the code but ensure multi-agent mode can NEVER be used:

1. **Remove the CLI flag definition**
   - File: `apchat-main/src/cli.rs`
   - Line: 30
   - Remove the `agents` field from `Cli` struct

2. **Set use_agents to false everywhere**
   - File: `apchat-main/src/apchat.rs`
   - Line: 246
   - Change from `use_agents` to `false`

3. **Remove agent coordinator initialization**
   - File: `apchat-main/src/apchat.rs`
   - Lines: 198-215
   - Replace with: `let agent_coordinator = None;`

4. **Remove conditional checks**
   - Remove `if chat.use_agents && chat.agent_coordinator.is_some()` checks
   - Always use single-agent code path

5. **Remove process_with_agents method**
   - File: `apchat-main/src/apchat.rs`
   - Lines: 302-342
   - Remove the entire method

### Complete Removal (Remove All Agent Code)

Follow the detailed guide in `docs/MULTI_AGENT_REMOVAL_GUIDE.md`.

Key points:
- Remove `apchat-agents` crate dependency
- Remove all agent-related imports
- Remove agent configuration files
- Remove agent coordinator fields and methods
- Remove conditional logic checking `use_agents`
- Handle type re-exports (`ToolDefinition`, `ChatMessage`)

## Verify Multi-Agent Mode is Disabled

### Check 1: Build succeeds without agent dependency
```bash
cargo clean
cargo build --release
```

### Check 2: --agents flag is unrecognized
```bash
cargo run -- --agents -i
# Should print: error: unexpected argument '--agents' found
```

### Check 3: Application runs normally
```bash
cargo run -- -i
# Should work fine in single-agent mode
```

### Check 4: Task mode works
```bash
cargo run -- --task "hello world"
# Should work fine
```

### Check 5: All tools still work
Test each tool in single-agent mode to ensure they weren't accidentally removed.

## Summary Table

| Aspect | Multi-Agent Mode Setting | Location |
|--------|------------------------|----------|
| **Control Mechanism** | `--agents` CLI flag | `apchat-main/src/cli.rs:30` |
| **Default Value** | `false` (disabled) | N/A |
| **Environment Variable** | None | N/A |
| **Feature Flag** | None | N/A |
| **Config File Setting** | None | N/A |
| **Runtime Check Location** | Multiple entry points | main.rs, task.rs, repl/inference.rs, web/routes.rs |
| **Agent Config Files** | `agents/configs/*.json` | Only read when `--agents` is set |
| **Agent Crate** | `crates/apchat-agents` | Always compiled, only used when `--agents` is set |
| **Coordinator Field** | `agent_coordinator` | `APChat` struct field |
| **Flag Field** | `use_agents` | `APChat` struct field |

## Conclusion

**Multi-agent mode is configured through exactly one mechanism: the `--agents` CLI flag.**

There are NO:
- Environment variables
- Feature flags
- Configuration file settings
- Build-time configurations

To permanently disable multi-agent mode, remove the `--agents` flag definition from the CLI and ensure all code paths default to single-agent mode. For complete removal of multi-agent code, follow the detailed removal guide.