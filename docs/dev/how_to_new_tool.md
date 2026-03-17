# How to Add New Tools to APChat

## Overview

APChat uses a trait-based tool system. Each tool implements the `Tool` trait from `apchat-toolcore`, is registered in a central registry at startup, and becomes available to LLM agents.

## Tool System Architecture

**Key types** (all in `apchat-toolcore`):
- `Tool` trait — defines name, description, parameters, and async execute
- `ToolParameters` — typed parameter access via `get_required<T>()` / `get_optional<T>()`
- `ToolResult` — success/error result with optional truncation info
- `ParameterDefinition` — parameter schema (type, description, required, default)
- `ToolContext` — execution context (work_dir, policy checking, permissions, etc.)
- `param!` macro — convenient parameter definition

## Step-by-Step Process

### 1. Create Tool File

Create `crates/apchat-tools/src/your_tool.rs`:

```rust
use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;

pub struct YourTool;

#[async_trait]
impl Tool for YourTool {
    fn name(&self) -> &str {
        "your_tool"
    }

    fn description(&self) -> &str {
        "Brief description of what the tool does"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("file_path", "string", "Path to the file", required),
            param!("limit", "number", "Max results to return", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        // Extract parameters
        let file_path = match params.get_required::<String>("file_path") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let limit = params.get_optional::<u64>("limit")
            .ok()
            .flatten()
            .unwrap_or(10);

        // Use context.work_dir for resolving paths
        let full_path = context.work_dir.join(&file_path);

        // Do work...

        ToolResult::success(format!("Processed {} with limit {}", file_path, limit))
    }
}
```

### 2. Export from Module

Add to `crates/apchat-tools/src/lib.rs`:

```rust
pub mod your_tool;
pub use your_tool::YourTool;
```

### 3. Register in Tool Registry

Add to `apchat-main/src/config/mod.rs` in the registry initialization:

```rust
registry.register_with_categories(YourTool, vec!["your_category".to_string()]);
```

Common categories: `file_ops`, `search`, `system`, `web`, `model_management`, `agent_control`, `skills`, `task_tracking`.

### 4. Add to Agent Configs

Add `"your_tool"` to the `tools` array in relevant `agents/configs/*.json` files.

## Key Patterns

### Permission Checking

For operations that need user approval (file writes, command execution):

```rust
let (approved, _) = context.check_permission_async(
    apchat_policy::ActionType::FileEdit,  // or CommandExecution
    &description,
).await.unwrap_or((false, false));

if !approved {
    return ToolResult::error("Operation not approved".to_string());
}
```

### The `param!` Macro

```rust
// Required parameter
param!("name", "string", "Description", required)

// Optional parameter
param!("count", "number", "Description", optional)
```

Supported types: `"string"`, `"number"`, `"boolean"`, `"array"`, `"object"`.

### ToolResult Variants

```rust
ToolResult::success(content)                           // Normal success
ToolResult::success_with_truncation(content, path)     // Truncated output with full path
ToolResult::error(message)                             // Error
```

## Reference

- Tool trait definition: `crates/apchat-toolcore/src/tool.rs`
- Tool context: `crates/apchat-toolcore/src/tool_context.rs`
- Tool registry: `crates/apchat-toolcore/src/tool_registry.rs`
- Example tools: `crates/apchat-tools/src/system.rs` (RunCommandTool), `crates/apchat-tools/src/file_ops.rs` (ReadFileTool)
- Registration: `apchat-main/src/config/mod.rs`
