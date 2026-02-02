# Multi-Agent Mode Removal Guide

This document identifies all configuration files, command-line arguments, environment variables, and settings that enable multi-agent mode in APChat. Removing these will completely eliminate the multi-agent functionality.

## Summary

Multi-agent mode is controlled solely through the `--agents` CLI flag. There are NO environment variables or feature flags that control multi-agent mode. The agent system is implemented in the `crates/apchat-agents` crate, which is a direct dependency (not optional).

## Complete Removal Checklist

### 1. CLI Argument (Primary Control)

**File:** `apchat-main/src/cli.rs`
**Line:** 30
```rust
/// Enable multi-agent system for specialized task handling
#[arg(long, action = clap::ArgAction::SetTrue)]
pub agents: bool,
```

**ACTION:** Remove the `agents` field from the `Cli` struct.

### 2. Dependency Removal

**File:** `apchat-main/Cargo.toml`
**Line:** 19
```toml
apchat-agents = { path = "../crates/apchat-agents" }
```

**File:** `Cargo.toml`
**Line:** 4
```toml
"crates/apchat-agents",
```

**ACTION:** Remove the `apchat-agents` dependency from both files and remove `crates/apchat-agents` from the workspace members list.

### 3. Main Entry Point Logic

**File:** `apchat-main/src/main.rs`
**Lines:** 45-64 (task mode handling)
```rust
// Handle task mode if requested
if let Some(task_text) = cli.task.clone() {
    // Use subagent mode for single-agent mode (when --agents is NOT specified)
    if !cli.agents {
        return run_subagent_mode(
            &cli,
            task_text,
            app_config.client_config,
            app_config.work_dir,
            app_config.policy_manager,
        )
        .await;
    } else {
        // Use regular task mode for multi-agent system (when --agents IS specified)
        return run_task_mode(
            &cli,
            task_text,
            app_config.client_config,
            app_config.work_dir,
            app_config.policy_manager,
        )
        .await;
    }
}
```

**ACTION:** Simplify to always call `run_task_mode` (or rename it back to the non-agent version).

### 4. APChat Struct Fields

**File:** `apchat-main/src/apchat.rs`
**Lines:** 14-17 (imports)
```rust
use apchat_agents::{
    PlanningCoordinator, GroqLlmClient,
    ChatMessage, ExecutionContext,
};
```

**Lines:** 45-46 (struct fields)
```rust
// Agent system
pub(crate) agent_coordinator: Option<PlanningCoordinator>,
pub(crate) use_agents: bool,
```

**ACTION:** Remove the agent-related imports and fields from the `APChat` struct.

### 5. APChat Constructor Logic

**File:** `apchat-main/src/apchat.rs`
**Lines:** 198-215 (agent coordinator initialization)
```rust
let agent_coordinator = if use_agents {
    match initialize_agent_system(&client_config, &tool_registry, &policy_manager) {
        Ok(coordinator) => Some(coordinator),
        Err(e) => {
            print_heart_yellow(&format!("{} Failed to initialize agent system: {}", "❌".red(), e), true);
            print_heart_yellow(&format!("{} Falling back to non-agent mode", "⚠️".yellow()), true);
            None
        }
    }
} else {
    None
};
```

**Line:** 245 (passed to struct)
```rust
agent_coordinator,
```

**Line:** 246 (passed to struct)
```rust
use_agents,
```

**ACTION:** Remove the conditional agent coordinator initialization and pass `None` for `agent_coordinator` and set `use_agents` to `false` in the struct construction.

### 6. Agent System Import in config/mod.rs

**File:** `apchat-main/src/config/mod.rs`
**Lines:** 5-6
```rust
use apchat_agents::{
    PlanningCoordinator, AgentFactory,
};
```

**Lines:** 189-240 (initialize_agent_system function)
```rust
/// Initialize the agent system with configuration files
pub fn initialize_agent_system(client_config: &ClientConfig, tool_registry: &ToolRegistry, policy_manager: &PolicyManager) -> Result<PlanningCoordinator> {
    // ... entire function implementation
}
```

**ACTION:** Remove the imports and the `initialize_agent_system` function.

### 7. Task Mode Logic

**File:** `apchat-main/src/app/task.rs`
**Lines:** 44-45 (check for agents)
```rust
if chat.use_agents && chat.agent_coordinator.is_some() {
    // Use agent system
    match chat.process_with_agents(&task_text, None).await {
        Ok(response) => response,
        // ... error handling
    }
}
```

**Line:** 47 (agent processing)
```rust
chat.process_with_agents(&task_text, None).await
```

**ACTION:** Remove the conditional check and `process_with_agents` call. Always use `chat()` function (non-agent mode).

### 8. Subagent Mode Logic

**File:** `apchat-main/src/app/subagent.rs`
**Lines:** 70-73 (conditional agent processing)
```rust
let result = if cli.agents && subagent.agent_coordinator.is_some() {
    // Use agent system if enabled and available
    subagent.process_with_agents(&task_text, None).await
} else {
    // Use regular chat
    crate::chat::session::chat(&mut subagent, &task_text, None).await
};
```

**ACTION:** Remove the conditional check and always use `chat()` function (non-agent mode).

### 9. REPL Inference Logic

**File:** `apchat-main/src/app/repl/inference.rs`
**Lines:** 35-59 (agent processing)
```rust
if chat.use_agents && chat.agent_coordinator.is_some() {
    match chat.process_with_agents(input, Some(cancel_token.clone())) {
        // ... agent handling
    }
}
```

**ACTION:** Remove the conditional check and agent processing logic.

### 10. Web Server Routes Agent Flag Capture

**File:** `apchat-main/src/web/routes.rs`
**Lines:** 705-706
```rust
// Capture the use_agents flag before dropping the lock
let use_agents = apchat.use_agents;
```

**Lines:** 730-746 (agent handling)
```rust
if use_agents {
    // Multi-agent mode - use existing process_with_agents
    match session.apchat.lock().await
        .process_with_agents(&content, None)
        .await
    {
        // ... agent response handling
    }
}
```

**ACTION:** Remove the `use_agents` flag capture and conditional agent handling.

### 11. process_with_agents Method

**File:** `apchat-main/src/apchat.rs`
**Lines:** 302-342
```rust
pub async fn process_with_agents(&mut self, user_request: &str, cancellation_token: Option<tokio_util::sync::CancellationToken>) -> Result<String> {
    if let Some(coordinator) = &mut self.agent_coordinator {
        // ... agent processing logic
    } else {
        Err(anyhow::anyhow!("Agent coordinator not initialized"))
    }
}
```

**ACTION:** Remove the entire `process_with_agents` method.

### 12. API Client Imports

**File:** `apchat-main/src/api/client.rs`
**Line:** 10
```rust
use apchat_agents::{ToolDefinition, ChatMessage};
```

**File:** `apchat-main/src/api/streaming.rs`
**Line:** 7
```rust
use apchat_agents::{ToolDefinition, ChatMessage};
```

**ACTION:** Remove or replace these imports. Note: These types might need to be re-exported from another crate or defined locally if they're used outside of agent context.

**WARNING:** These types (ToolDefinition, ChatMessage) are used in the non-agent API code as well. You'll need to either:
- Keep the `apchat-agents` crate for just these type definitions, OR
- Move these types to a more appropriate location (like `apchat-models` or create a new shared types crate)

### 13. Functions that use use_agents check

**File:** `apchat-main/src/apchat.rs`
**Lines:** 492-494
```rust
if !self.use_agents {
    // ... logic
}
```

**ACTION:** Remove any conditional logic based on `use_agents` flag.

### 14. Configuration Files in src/config/mod.rs

**File:** `apchat-main/src/config/mod.rs`
Line 189-240: The entire `initialize_agent_system` function

**ACTION:** Remove this function entirely.

### 15. Import in chat/tests.rs

**File:** `apchat-main/src/chat/tests.rs`
**Line:** 29
```rust
use_agents: false,
```

**Line:** 28
```rust
agent_coordinator: None,
```

**ACTION:** Remove these test struct field initializations (they'll no longer exist).

### 16. Documentation Updates

Multiple documentation files reference the `--agents` flag:

**Files to update:**
- `CLAUDE.md` - Lines 46, 81-127, 240, 242-263
- `README.md` - Lines 113, 202, 290
- `docs/dev/CUSTOMIZING_AGENTS_AND_SKILLS.md` - Entire file
- `docs/reorg-workspace/MIGRATION_PLAN.md` - Line 131
- `docs/project/CLAUDE.md` - Lines 8, 31, 262, 300, 306, 309, 312, 386

**ACTION:** Remove or update all references to multi-agent mode and the `--agents` flag.

### 17. Agent Configuration Files

**Directory:** `agents/configs/`
**Files:**
- `agents/configs/code_analyzer.json`
- `agents/configs/code_reviewer.json`
- `agents/configs/file_manager.json`
- `agents/configs/planner.json`
- `agents/configs/search_specialist.json`
- `agents/configs/system_operator.json`
- `agents/configs/terminal_specialist.json`

**ACTION:** Remove the entire `agents/` directory and all its contents.

### 18. Subagent Tools (Keep These!)

**File:** `crates/apchat-tools/src/subagent_tools.rs`

**IMPORTANT:** The `launch_subagent` and `launch_subagent_pretty` tools should be KEPT. These are tools that allow the AI to delegate to an isolated instance of itself for specific tasks. This is different from multi-agent mode and provides benefits like context savings and parallel execution.

### 19. Test Files

**File:** `apchat-main/src/app/subagent_tests.rs`
**File:** `crates/apchat-agents/src/agent_tests.rs`

**ACTION:** Remove `crates/apchat-agents/src/agent_tests.rs`. Review `apchat-main/src/app/subagent_tests.rs` - this tests subagent mode (which should be kept), not multi-agent mode.

### 20. Workspace Member

**File:** `Cargo.toml`
**Line:** 3
```toml
"crates/apchat-agents",
```

**ACTION:** Remove from workspace members list.

## Critical Path Analysis

If you want to remove multi-agent mode cleanly, follow this order:

1. **Start with the dependency** - Remove `apchat-agents` from Cargo.toml and workspace
2. **This will cause compilation errors** - Fix them by:
   - Removing imports from `apchat_agents`
   - Removing the `agents` field from `Cli` struct
   - Removing `agent_coordinator` and `use_agents` from `APChat` struct
   - Removing `initialize_agent_system` function
   - Removing `process_with_agents` method
   - Simplifying task/subagent/repl/web-server logic to NOT check for agents
3. **Handle type re-exports** - If `ToolDefinition` and `ChatMessage` are needed outside of agent context, move them to `apchat-models` or create a shared types crate
4. **Remove agent config directory** - Delete `agents/` directory
5. **Update documentation** - Remove references to `--agents` flag

## What to Keep

Do NOT remove:
- **Subagent tools** (`launch_subagent`, `launch_subagent_pretty`) - these are different from multi-agent mode
- **Task mode** (`--task`) - this works in single-agent mode too
- **Web server mode** - works in single-agent mode
- **All other tools** - they work in single-agent mode

## Testing After Removal

After removal, test:
1. `cargo build --release` - ensure clean compilation
2. `cargo run -- -i` - interactive mode should work
3. `cargo run -- --task "hello world"` - task mode should work
4. `cargo run -- --web` - web server should work
5. `cargo run -- -i --auto-confirm` - auto-confirm mode should work
6. Tool usage - all 20+ tools should work in single-agent mode

## No Environment Variables

There are NO environment variables that control multi-agent mode. The only control is the `--agents` CLI flag.

## No Feature Flags

Multi-agent mode is NOT behind a feature flag. The `apchat-agents` crate is a direct dependency, not an optional one.

## Complexity Warning

The removal is moderately complex because:
1. The `apchat-agents` crate defines some types (`ToolDefinition`, `ChatMessage`) that are used in the non-agent API code
2. The removal affects multiple files across the codebase
3. Documentation references are widespread

## Alternative: Disable via CLI Flag

If you want to keep the code but disable multi-agent mode temporarily, you can:
1. Keep all code as-is
2. Simply never use the `--agents` flag
3. The application will always use single-agent mode (which is the default behavior when `--agents` is not specified)

## Summary of Files to Modify

### Remove completely (7 files):
- `crates/apchat-agents/src/` - entire directory (6 files)
- `agents/configs/` - entire directory (7 config files)
- `agents/` - entire directory

### Modify (10 files):
1. `apchat-main/src/main.rs` - simplify task mode logic
2. `apchat-main/src/cli.rs` - remove `agents` field
3. `apchat-main/src/apchat.rs` - remove agent fields, imports, and methods
4. `apchat-main/src/config/mod.rs` - remove agent imports and initialization function
5. `apchat-main/src/app/task.rs` - remove agent processing logic
6. `apchat-main/src/app/subagent.rs` - remove agent processing logic
7. `apchat-main/src/app/repl/inference.rs` - remove agent processing logic
8. `apchat-main/src/web/routes.rs` - remove agent handling
9. `apchat-main/src/api/client.rs` - remove or replace agent imports
10. `apchat-main/src/api/streaming.rs` - remove or replace agent imports
11. `apchat-main/src/chat/tests.rs` - remove agent test fields
12. `Cargo.toml` - remove from workspace members
13. `apchat-main/Cargo.toml` - remove dependency
14. Documentation files (5+ files) - remove references

### Type definitions to relocate (if needed):
- `ToolDefinition` - from `apchat-agents`
- `ChatMessage` - from `apchat-agents`

These two types are used in the non-agent API code and may need to be moved to a more appropriate location.