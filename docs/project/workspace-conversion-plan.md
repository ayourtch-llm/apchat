# KimiChat Workspace Conversion Plan

## Executive Summary

This plan outlines the conversion of the kimichat monolithic crate into a Rust workspace with 10 distinct crates. The design preserves the current architecture while improving modularity, compile times, and code organization. Special attention is paid to maintaining `pub(crate)` visibility semantics by shrinking visibility scopes where appropriate.

## Goals

1. **Modularity**: Separate concerns into distinct crates with clear boundaries
2. **Compile Performance**: Enable parallel compilation and incremental builds
3. **Visibility Control**: Convert `pub(crate)` to narrower scopes where possible
4. **Maintainability**: Clearer dependency graphs and reduced coupling
5. **Backward Compatibility**: Minimal changes to existing code structure

## Workspace Structure

```
kimichat/
├── Cargo.toml (workspace root)
├── crates/
│   ├── kimichat-types/      # Core types and traits
│   ├── kimichat-models/     # API request/response models
│   ├── kimichat-policy/     # Security and policy management
│   ├── kimichat-terminal/   # PTY and terminal management
│   ├── kimichat-tools/      # Tool system (core + implementations)
│   ├── kimichat-skills/     # Skill registry and embeddings
│   ├── kimichat-api/        # LLM API clients
│   ├── kimichat-agents/     # Agent system and coordination
│   ├── kimichat-chat/       # Conversation management
│   └── kimichat-app/        # Application logic and CLI (main binary)
├── skills/                   # Skill library (unchanged)
├── agents/                   # Agent configs (unchanged)
├── docs/                     # Documentation (unchanged)
└── projects/                 # Subprojects (unchanged)
```

---

## Crate Breakdown and Dependencies

### 1. `kimichat-types` (Foundation Crate)

**Purpose**: Core types, traits, and shared structures used across all crates.

**Contents**:
- Core types from `src/models/types.rs`:
  - `ModelType` enum
  - `Message` struct
  - `ToolCall` struct
  - `ClientConfig` struct
- Core traits:
  - `Tool` trait (from `src/core/tool.rs`)
  - `LlmClient` trait (from agents)
- Error types and result wrappers
- Constants: `MAX_CONTEXT_TOKENS`, `MAX_RETRIES`
- `ToolResult` struct (from core)
- Logging types (from `src/logging/`)

**Dependencies**:
```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = "0.1"
thiserror = "1.0"
anyhow = "1.0"
chrono = "0.4.42"
uuid = { version = "1.0", features = ["v4", "serde"] }
reqwest = { version = "0.12", features = ["json", "stream"] }
tokio = { version = "1.41", features = ["sync", "fs", "io-util"] }
```

**Visibility Changes**:
- Current `pub(crate)` items become `pub` (they're foundational)
- No shrinking possible at this level

**Rationale**: This is the foundation crate. All other crates depend on it. Keeping types separate enables clean dependency graphs.

---

### 2. `kimichat-models` (API Models)

**Purpose**: API request/response structures for LLM backends.

**Contents**:
- From `src/models/`:
  - `requests.rs`: `ChatRequest`, `Tool`, `FunctionDef`
  - `responses.rs`: `ChatResponse`, `StreamChunk`, `Usage`
  - All tool argument structures (moved from `types.rs`)
- Serialization helpers

**Dependencies**:
```toml
[dependencies]
kimichat-types = { path = "../kimichat-types" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

**Visibility Changes**:
- All types are `pub` (API models are inherently public to their crate)
- No `pub(crate)` items existed here

**Rationale**: API models are stable interfaces shared between API clients and chat logic.

---

### 3. `kimichat-policy` (Security & Policy)

**Purpose**: Security policy management and permission checking.

**Contents**:
- `src/policy.rs` → `lib.rs`:
  - `PolicyManager` struct
  - `ActionType` enum
  - `Decision` enum
  - Policy evaluation logic
  - Pattern-based rules

**Dependencies**:
```toml
[dependencies]
kimichat-types = { path = "../kimichat-types" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
regex = "*"
anyhow = "1.0"
```

**Visibility Changes**:
- No `pub(crate)` items in original code
- Keep all public items as `pub`
- Internal policy rule matching can be `pub(crate)` or private

**Rationale**: Policy is a cross-cutting concern used by tools and terminal backends. Isolating it makes security audits easier.

---

### 4. `kimichat-terminal` (Terminal Management)

**Purpose**: PTY and terminal session management with pluggable backends.

**Contents**:
- From `src/terminal/`:
  - `manager.rs`: `TerminalManager`
  - `session.rs`: `TerminalSession`
  - `backend.rs`: `TerminalBackend` trait
  - `pty_backend.rs`: PTY implementation
  - `tmux_backend.rs`: Tmux implementation
  - `pty_handler.rs`: `PtyHandler`
  - `screen_buffer.rs`: VT100 emulation
  - `logger.rs`: Session logging

**Dependencies**:
```toml
[dependencies]
kimichat-types = { path = "../kimichat-types" }
portable-pty = "0.8"
vt100 = "0.15"
tokio = { version = "1.41", features = ["sync", "process", "io-util", "time"] }
anyhow = "1.0"
thiserror = "1.0"
chrono = "0.4.42"
regex = "*"
```

**Visibility Changes**:
- `PtyHandler` struct:
  - Current: `pub(crate) pty` field
  - New: Keep `pty` as `pub(crate)` within this crate, or make private with accessor methods
  - Recommendation: **Make private** with `pub fn get_pty(&mut self) -> &mut Box<dyn MasterPty + Send>`
- All other items become `pub` for external use

**Rationale**: Terminal management is a self-contained subsystem with minimal dependencies. Separating it allows testing in isolation.

---

### 5. `kimichat-tools` (Tool System)

**Purpose**: Tool registry, core execution framework, and all tool implementations.

**Contents**:
- From `src/core/`:
  - `tool_registry.rs`: `ToolRegistry`
  - `tool_context.rs`: `ToolContext`
- From `src/tools/`:
  - `file_ops.rs`: File operation tools
  - `search.rs`: Search tools
  - `system.rs`: Command execution
  - `model_management.rs`: Model switching
  - `iteration_control.rs`: Agent iteration
  - `project_tools.rs`: Project management
  - `skill_tools.rs`: Skill loading tools
  - `todo_tools.rs`: Todo management
  - `helpers.rs`: Utilities
- Terminal tools (from `src/terminal/tools.rs`)

**Dependencies**:
```toml
[dependencies]
kimichat-types = { path = "../kimichat-types" }
kimichat-models = { path = "../kimichat-models" }
kimichat-policy = { path = "../kimichat-policy" }
kimichat-terminal = { path = "../kimichat-terminal" }
kimichat-skills = { path = "../kimichat-skills" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = "0.1"
tokio = { version = "1.41", features = ["fs", "process", "io-util"] }
regex = "*"
glob = "0.3"
ignore = "0.4"
similar = "2.6"
anyhow = "1.0"
colored = "2.1"
```

**Visibility Changes**:
- No `pub(crate)` in original tools code
- All tool implementations are `pub`
- Registry and context are `pub`
- Helper functions can remain `pub(crate)` or private within this crate

**Rationale**: Tools are the primary extension point. Grouping all tools together makes discovery easier and enables tool-specific testing.

---

### 6. `kimichat-skills` (Skill Library)

**Purpose**: Skill registry, loading, and semantic search.

**Contents**:
- From `src/skills/`:
  - `mod.rs`: `SkillRegistry`
  - `embeddings/`: Embedding backend
  - `embedded.rs`: Compiled-in skills

**Dependencies**:
```toml
[dependencies]
kimichat-types = { path = "../kimichat-types" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.41", features = ["fs", "io-util"] }
fastembed = { version = "5", optional = true }
anyhow = "1.0"
regex = "*"

[features]
default = ["embeddings"]
embeddings = ["fastembed"]
```

**Visibility Changes**:
- No `pub(crate)` in original skills code
- All public APIs remain `pub`
- Embedding internals can be `pub(crate)` or private

**Rationale**: Skills are optional and have a large dependency (fastembed). Separating them allows conditional compilation.

---

### 7. `kimichat-api` (LLM API Clients)

**Purpose**: LLM API communication and streaming.

**Contents**:
- From `src/api/`:
  - `mod.rs`: Re-exports
  - `client.rs`: Non-streaming calls
  - `streaming.rs`: Streaming calls
- From `src/agents/`:
  - `groq_client.rs`: Groq implementation
  - `anthropic_client.rs`: Anthropic/Claude client
  - `llama_cpp_client.rs`: Local Llama.cpp

**Dependencies**:
```toml
[dependencies]
kimichat-types = { path = "../kimichat-types" }
kimichat-models = { path = "../kimichat-models" }
reqwest = { version = "0.12", features = ["json", "stream"] }
tokio = { version = "1.41", features = ["sync"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
futures = "0.3"
futures-util = "0.3"
async-stream = "0.3"
anyhow = "1.0"
base64 = "0.22"
```

**Visibility Changes**:
- Current `pub(crate)` functions in `api/client.rs` and `api/streaming.rs`:
  - `call_api()` → **becomes `pub`**
  - `call_api_streaming()` → **becomes `pub`**
  - `call_api_with_llm_client()` → **becomes `pub`**
  - `call_api_streaming_with_llm_client()` → **becomes `pub`**
- Current `pub(crate)` re-exports in `api/mod.rs` → **become `pub` re-exports**
- LLM client implementations (Groq, Anthropic, Llama) are `pub`

**Rationale**: API clients are stateless and reusable. Separating them enables mocking and testing without the full application.

---

### 8. `kimichat-agents` (Agent System)

**Purpose**: Multi-agent orchestration and planning.

**Contents**:
- From `src/agents/`:
  - `mod.rs`: Module organization
  - `agent.rs`: Core agent implementation
  - `agent_config.rs`: Configuration structures
  - `agent_factory.rs`: Agent instantiation
  - `coordinator.rs`: `PlanningCoordinator`
  - `task.rs`: Task structures
  - `progress_evaluator.rs`: Progress evaluation
  - `visibility.rs`: Agent visibility
  - `embedded_configs.rs`: Pre-compiled configs

**Dependencies**:
```toml
[dependencies]
kimichat-types = { path = "../kimichat-types" }
kimichat-models = { path = "../kimichat-models" }
kimichat-tools = { path = "../kimichat-tools" }
kimichat-api = { path = "../kimichat-api" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.41", features = ["sync", "fs"] }
toml = "0.8"
anyhow = "1.0"
async-trait = "0.1"
chrono = "0.4.42"
```

**Visibility Changes**:
- No `pub(crate)` in original agent code
- All agent types and coordinator are `pub`
- Internal evaluation logic can be `pub(crate)` or private

**Rationale**: Agent system is complex and has many internal dependencies. Separating it allows focused development and testing.

---

### 9. `kimichat-chat` (Conversation Management)

**Purpose**: Chat history, state management, and conversation loop.

**Contents**:
- From `src/chat/`:
  - `mod.rs`: Module organization
  - `state.rs`: State save/load
  - `history.rs`: History management
  - `session.rs`: Chat loop

**Dependencies**:
```toml
[dependencies]
kimichat-types = { path = "../kimichat-types" }
kimichat-models = { path = "../kimichat-models" }
kimichat-api = { path = "../kimichat-api" }
kimichat-tools = { path = "../kimichat-tools" }
kimichat-agents = { path = "../kimichat-agents" }
kimichat-policy = { path = "../kimichat-policy" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.41", features = ["fs", "sync"] }
tokio-util = "0.7"
anyhow = "1.0"
chrono = "0.4.42"
colored = "2.1"
```

**Visibility Changes**:
- Current `pub(crate)` items:
  - `chat()` function → **becomes `pub`** (main entry point)
  - `safe_truncate()` → **becomes `pub`** (utility used by other crates)
  - `summarize_and_trim_history()` → **becomes `pub`** (core functionality)
- Tool execution and parsing from `src/tools_execution/`:
  - `repair_tool_call_with_model()` → **becomes `pub`**
  - `validate_and_fix_tool_calls_in_place()` → **becomes `pub`**

**Rationale**: Chat management is the orchestration layer that ties together APIs, tools, and agents. It's the primary consumer of most other crates.

---

### 10. `kimichat-app` (Application & CLI Binary)

**Purpose**: Main application logic, CLI interface, and application modes.

**Contents**:
- `src/main.rs` (modified):
  - `KimiChat` struct and implementation
  - Main entry point
- From `src/`:
  - `cli.rs`: CLI argument parsing
  - `todo.rs`: Todo management
  - `preview.rs`: File preview
  - `open_file.rs`: File opening
- From `src/app/`:
  - `setup.rs`: App configuration
  - `repl.rs`: REPL mode
  - `task.rs`: Task mode
  - `web_server.rs`: Web server mode
- From `src/web/`:
  - `protocol.rs`: WebSocket protocol
  - `session_manager.rs`: Web sessions
  - `routes.rs`: HTTP routes
  - `server.rs`: Server implementation
- From `src/config/`:
  - `helpers.rs`: Configuration utilities
  - Initialization logic

**Dependencies**:
```toml
[dependencies]
kimichat-types = { path = "../kimichat-types" }
kimichat-models = { path = "../kimichat-models" }
kimichat-policy = { path = "../kimichat-policy" }
kimichat-terminal = { path = "../kimichat-terminal" }
kimichat-tools = { path = "../kimichat-tools" }
kimichat-skills = { path = "../kimichat-skills" }
kimichat-api = { path = "../kimichat-api" }
kimichat-agents = { path = "../kimichat-agents" }
kimichat-chat = { path = "../kimichat-chat" }

clap = { version = "4.5", features = ["derive", "env"] }
clap_complete = "4.5"
tokio = { version = "1.41", features = ["full"] }
tokio-util = "0.7"
rustyline = "14.0"
reqwest = { version = "0.12", features = ["json", "stream"] }
axum = { version = "0.7", features = ["ws"] }
tower = "0.5"
tower-http = { version = "0.5", features = ["fs", "cors"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
colored = "2.1"
dotenvy = "0.15"
anyhow = "1.0"
chrono = "0.4.42"
uuid = { version = "1.0", features = ["v4"] }

[features]
default = ["embeddings"]
embeddings = ["kimichat-skills/embeddings"]
```

**Visibility Changes**:
- `KimiChat` struct:
  - All 19 `pub(crate)` fields → **remain `pub`** (this is a library type now)
  - Methods like `set_debug_level()` → **remain `pub`**
- No shrinking possible since this is the top-level orchestrator

**Rationale**: This is the final integration point and binary target. It assembles all components and provides CLI/web interfaces.

---

## Migration Strategy

### Phase 1: Preparation (Day 1)

1. **Create workspace structure**:
   ```bash
   mkdir -p crates/{kimichat-types,kimichat-models,kimichat-policy,kimichat-terminal,kimichat-tools,kimichat-skills,kimichat-api,kimichat-agents,kimichat-chat,kimichat-app}
   ```

2. **Create root Cargo.toml**:
   ```toml
   [workspace]
   resolver = "2"
   members = [
       "crates/kimichat-types",
       "crates/kimichat-models",
       "crates/kimichat-policy",
       "crates/kimichat-terminal",
       "crates/kimichat-tools",
       "crates/kimichat-skills",
       "crates/kimichat-api",
       "crates/kimichat-agents",
       "crates/kimichat-chat",
       "crates/kimichat-app",
   ]

   [workspace.package]
   version = "0.1.0"
   edition = "2021"
   authors = ["Your Name"]
   license = "MIT OR Apache-2.0"

   [workspace.dependencies]
   # Shared dependencies
   tokio = { version = "1.41", features = ["full"] }
   serde = { version = "1.0", features = ["derive"] }
   serde_json = "1.0"
   anyhow = "1.0"
   # ... (others)
   ```

3. **Initialize each crate**:
   ```bash
   for crate in crates/*; do
       cd $crate
       cargo init --lib  # or --bin for kimichat-app
       cd ../..
   done
   ```

### Phase 2: Bottom-Up Migration (Days 2-5)

**Order of migration** (respecting dependency graph):

1. **kimichat-types** (no dependencies)
   - Move core types, traits, errors
   - Update visibility from `pub(crate)` to `pub`

2. **kimichat-models** (depends on types)
   - Move API models
   - Update imports to `use kimichat_types::*`

3. **kimichat-policy** (depends on types)
   - Move policy.rs
   - Update imports

4. **kimichat-terminal** (depends on types)
   - Move terminal module
   - Shrink `PtyHandler::pty` visibility (make private with accessor)

5. **kimichat-skills** (depends on types)
   - Move skills module
   - Preserve feature flags

6. **kimichat-api** (depends on types, models)
   - Move API clients
   - Change `pub(crate)` to `pub` for main functions
   - Update imports

7. **kimichat-tools** (depends on types, models, policy, terminal, skills)
   - Move core and tools modules
   - Update tool implementations to use crate imports

8. **kimichat-agents** (depends on types, models, tools, api)
   - Move agents module
   - Update agent factory to import LLM clients correctly

9. **kimichat-chat** (depends on types, models, api, tools, agents, policy)
   - Move chat module
   - Move tools_execution module here
   - Change `pub(crate)` to `pub` for main functions

10. **kimichat-app** (depends on all)
    - Move main.rs, cli.rs, app/, web/, config/
    - `KimiChat` fields remain `pub`
    - Update all imports to use workspace crates

### Phase 3: Testing & Validation (Day 6)

1. **Build verification**:
   ```bash
   cargo build --workspace
   cargo test --workspace
   cargo clippy --workspace -- -D warnings
   ```

2. **Integration testing**:
   - Test REPL mode
   - Test task mode
   - Test web server mode
   - Verify agent system
   - Verify tool execution

3. **Visibility audit**:
   - Ensure no unnecessary `pub` items
   - Verify `pub(crate)` usage within new crates
   - Check that visibility is as narrow as possible

### Phase 4: Documentation & Cleanup (Day 7)

1. **Update documentation**:
   - Add README.md to each crate
   - Document crate purpose and API
   - Update main README with workspace structure

2. **Cleanup**:
   - Remove old src/ directory (after verification)
   - Update CI/CD configs
   - Update .gitignore if needed

---

## Dependency Graph

```
                     kimichat-types
                     (foundation)
                           │
          ┌────────────────┼────────────────┐
          │                │                │
    kimichat-models   kimichat-policy  kimichat-terminal
          │                │                │
          │                └────┬───────────┘
          │                     │
          └──────────┬──────────┴───────────┐
                     │                      │
              kimichat-skills         kimichat-api
                     │                      │
                     └──────────┬───────────┘
                                │
                        kimichat-tools
                                │
                                │
                        kimichat-agents
                                │
                                │
                         kimichat-chat
                                │
                                │
                         kimichat-app
                           (binary)
```

**Key observations**:
- Linear dependency chain from foundation → binary
- No circular dependencies
- Clean separation of concerns
- Terminal and skills are optional dependencies for tools

---

## Visibility Transformation Summary

| Original Location | Item | Old Visibility | New Crate | New Visibility | Rationale |
|-------------------|------|----------------|-----------|----------------|-----------|
| `src/main.rs` | `KimiChat` fields (19) | `pub(crate)` | `kimichat-app` | `pub` | Top-level orchestrator, used as library type |
| `src/main.rs` | `MAX_CONTEXT_TOKENS` | `pub(crate)` | `kimichat-types` | `pub` | Constant used across crates |
| `src/main.rs` | `MAX_RETRIES` | `pub(crate)` | `kimichat-types` | `pub` | Constant used across crates |
| `src/api/client.rs` | `call_api()` | `pub(crate)` | `kimichat-api` | `pub` | Public API for chat crate |
| `src/api/client.rs` | `call_api_with_llm_client()` | `pub(crate)` | `kimichat-api` | `pub` | Public API for chat crate |
| `src/api/streaming.rs` | `call_api_streaming()` | `pub(crate)` | `kimichat-api` | `pub` | Public API for chat crate |
| `src/api/streaming.rs` | `call_api_streaming_with_llm_client()` | `pub(crate)` | `kimichat-api` | `pub` | Public API for chat crate |
| `src/chat/history.rs` | `safe_truncate()` | `pub(crate)` | `kimichat-chat` | `pub` | Utility used by multiple modules |
| `src/chat/history.rs` | `summarize_and_trim_history()` | `pub(crate)` | `kimichat-chat` | `pub` | Core chat functionality |
| `src/chat/session.rs` | `chat()` | `pub(crate)` | `kimichat-chat` | `pub` | Main chat loop entry point |
| `src/tools_execution/validation.rs` | `repair_tool_call_with_model()` | `pub(crate)` | `kimichat-chat` | `pub` | Used by chat loop |
| `src/tools_execution/validation.rs` | `validate_and_fix_tool_calls_in_place()` | `pub(crate)` | `kimichat-chat` | `pub` | Used by chat loop |
| `src/terminal/pty_handler.rs` | `PtyHandler::pty` field | `pub(crate)` | `kimichat-terminal` | `private` (with accessor) | **SHRUNK** - Better encapsulation |

**Summary**:
- **12 items expanded** from `pub(crate)` to `pub` (necessary for cross-crate usage)
- **1 item shrunk** from `pub(crate)` to `private` with accessor (improved encapsulation)
- Net result: Better separation with minimal visibility expansion

---

## Benefits of This Design

### 1. **Improved Compile Times**
- Parallel compilation of independent crates
- Incremental builds only rebuild changed crates
- Estimated 30-40% improvement on incremental builds

### 2. **Better Modularity**
- Clear boundaries between subsystems
- Easier to understand dependencies
- Prevents accidental coupling

### 3. **Enhanced Testing**
- Test each crate in isolation
- Mock dependencies more easily
- Faster test iteration

### 4. **Code Reusability**
- Extract useful crates (e.g., `kimichat-terminal`) for other projects
- Clean API boundaries enable library usage

### 5. **Improved Documentation**
- Per-crate documentation
- Clearer API surfaces
- Better examples and usage guides

### 6. **Visibility Control**
- Most `pub(crate)` converted to `pub` (necessary for cross-crate)
- One visibility shrink (PtyHandler::pty → private with accessor)
- Overall: cleaner API boundaries despite visibility expansion

---

## Risks and Mitigations

### Risk 1: Circular Dependencies
**Mitigation**: Dependency graph is designed to be strictly linear. No circular dependencies.

### Risk 2: Build Time Regression (Initial)
**Mitigation**: First build will be slower due to workspace overhead. Subsequent builds will be faster.

### Risk 3: Over-Granularity
**Mitigation**: Crates are sized appropriately (100-1000 LOC each). Not too fine-grained.

### Risk 4: Breaking Changes
**Mitigation**: This is an internal refactor. External API (CLI) remains unchanged.

### Risk 5: Import Churn
**Mitigation**: Use workspace dependencies and re-exports to minimize import changes.

---

## Success Criteria

1. ✅ All tests pass: `cargo test --workspace`
2. ✅ Clean builds: `cargo build --workspace --release`
3. ✅ No clippy warnings: `cargo clippy --workspace -- -D warnings`
4. ✅ Faster incremental builds (measured)
5. ✅ REPL, task, and web modes function identically
6. ✅ No regression in functionality
7. ✅ Documentation updated for each crate
8. ✅ Visibility audit passes (no unnecessary `pub`)

---

## Timeline Estimate

| Phase | Duration | Tasks |
|-------|----------|-------|
| Preparation | 1 day | Create structure, initialize crates |
| Bottom-up migration | 4 days | Migrate code crate by crate |
| Testing & validation | 1 day | Build, test, verify |
| Documentation | 1 day | READMEs, API docs |
| **Total** | **7 days** | Full workspace conversion |

---

## Next Steps

1. **Review this plan** with stakeholders
2. **Create a feature branch** for workspace conversion
3. **Execute Phase 1** (preparation)
4. **Iterate through Phase 2** (migration) crate by crate
5. **Validate with Phase 3** (testing)
6. **Finalize with Phase 4** (documentation)
7. **Merge to main** after validation

---

## Appendix: Alternative Designs Considered

### Alternative 1: Fewer Crates (5 total)
- Combine tools + agents + chat into single "core" crate
- **Rejected**: Too coarse-grained, loses modularity benefits

### Alternative 2: More Crates (15 total)
- Split tools into file_ops, search, system, etc.
- **Rejected**: Over-granular, excessive build overhead

### Alternative 3: Binary-Only Approach
- Keep single crate, use modules
- **Rejected**: No compile time improvement, harder to extract reusable components

### Alternative 4: Feature Flags Instead of Crates
- Use Cargo features to enable/disable components
- **Rejected**: Doesn't improve compile times, complex feature matrix

**Chosen design (10 crates)** balances granularity, compile performance, and maintainability.
