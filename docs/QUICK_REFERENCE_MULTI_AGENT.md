# Quick Reference: Multi-Agent Mode Configuration

## Primary Control (One Single Location)

### CLI Flag: `--agents`

**File:** `apchat-main/src/cli.rs` (line 30)
```rust
/// Enable multi-agent system for specialized task handling
#[arg(long, action = clap::ArgAction::SetTrue)]
pub agents: bool,
```

This is the ONLY way to enable multi-agent mode. When this flag is NOT present (default), the application runs in single-agent mode.

## What Controls Multi-Agent Mode

| Control Type | Location | Value | Notes |
|-------------|----------|-------|-------|
| **CLI Flag** | `--agents` | `true`/`false` | Only control mechanism |
| **Environment Variable** | None | N/A | Does not exist |
| **Feature Flag** | None | N/A | Not behind a Cargo feature |
| **Config File** | None | N/A | No config file setting |

## Dependencies

**Required Crate:**
- `crates/apchat-agents` - Multi-agent orchestration and coordination

**Dependency Status:**
- Workspace member: Yes (Cargo.toml line 4)
- Main crate dependency: Yes (apchat-main/Cargo.toml line 19)
- Optional dependency: NO - always included

## Agent Configuration Files

**Location:** `agents/configs/`
- `code_analyzer.json`
- `code_reviewer.json`
- `file_manager.json`
- `planner.json`
- `search_specialist.json`
- `system_operator.json`
- `terminal_specialist.json`

## Key Code Locations

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

## Default Behavior

**Without `--agents` flag:**
- Application runs in single-agent mode
- Direct conversation with one LLM
- All 20+ tools are available
- Subagent tools (`launch_subagent`) still work for task delegation

**With `--agents` flag:**
- Planner-first architecture
- Task decomposition by planner agent
- Delegation to specialized agents:
  - planner (task decomposition)
  - code_analyzer (code review)
  - code_reviewer (mandatory skill workflows)
  - file_manager (file operations)
  - search_specialist (code search)
  - system_operator (command execution)
  - terminal_specialist (PTY/terminal management)

## Environment Variables That Don't Control Agents

There are NO environment variables that control multi-agent mode. The following environment variables exist but control OTHER things:
- `GROQ_API_KEY` - API key for Groq models
- `ANTHROPIC_AUTH_TOKEN` - API key for Anthropic models
- `OPENAI_API_KEY` - API key for OpenAI models
- `APCHAT_MEMORY_DB_PATH` - Path to memory database
- `APCHAT_TERMINAL_BACKEND` - Terminal backend choice (pty/tmux)
- `APCHAT_WEB_PORT` - Web server port
- `APCHAT_WEB_BIND` - Web server bind address
- `OKAYCHAT_SESSIONS_DIR` - Web session storage directory

## Feature Flags

There are NO feature flags related to multi-agent mode. The only feature flags in the codebase are:
- `embeddings` - Enables semantic skill search (default: true)

## Configuration Files

There are NO configuration files that control multi-agent mode. The only configuration files are:
- `policies.toml` - Action approval policies
- `.env` - Environment variables (for API keys, not agent control)
- `agents/configs/*.json` - Agent definitions (only used when `--agents` is set)

## Summary

To completely disable multi-agent mode, you have TWO options:

### Option 1: Remove --agents flag (Recommended)
Simply never use the `--agents` flag. The application defaults to single-agent mode.

### Option 2: Remove entire multi-agent system
Follow the detailed removal guide in `docs/MULTI_AGENT_REMOVAL_GUIDE.md`. This removes:
- The `apchat-agents` crate
- All agent configuration files
- All agent-related code
- The `--agents` CLI flag
- Documentation references to multi-agent mode

## Key Insight

**Multi-agent mode is optional and OFF by default.** The application is designed to work perfectly in single-agent mode. The multi-agent system is an enhancement for complex tasks, not a requirement for basic functionality.