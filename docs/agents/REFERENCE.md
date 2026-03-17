# Reference Guide

## Workspace Structure

```
apchat/
├── apchat-main/           # Main binary crate (CLI, REPL, web server)
├── crates/
│   ├── apchat-common/     # Common utilities and types
│   ├── apchat-core/       # Core functionality
│   ├── apchat-designer/   # Design-related tools
│   ├── apchat-finserv/    # Financial services tools
│   ├── apchat-llm-api/    # Unified LLM client interface
│   ├── apchat-logging/    # Conversation logging (JSONL)
│   ├── apchat-mcp-pty-server/  # MCP PTY server integration
│   ├── apchat-models/     # Data structures and types
│   ├── apchat-mspc/       # Message passing channels
│   ├── apchat-ocr/        # Optical character recognition
│   ├── apchat-policy/     # Security and approval system
│   ├── apchat-pptx/       # PowerPoint processing
│   ├── apchat-progress/   # Progress tracking for streaming
│   ├── apchat-skills/     # Skill registry and loading
│   ├── apchat-terminal/   # PTY session management with VT100
│   ├── apchat-todo/       # Task tracking
│   ├── apchat-toolcore/   # Tool execution framework
│   ├── apchat-tools/      # 50+ tool implementations
│   ├── apchat-types/      # Type definitions
│   ├── apchat-vty/        # VTY/readline abstractions
│   └── apchat-webex/      # Webex bot integration
├── agents/configs/        # Agent JSON configurations (8 agents)
├── skills/                # Skill definitions (38 skills)
├── hooks/                 # Session lifecycle hooks
└── docs/                  # Extensive documentation
```

## Complete Skills List (38 Total)

### Core Workflows
- `test-driven-development` - Red-Green-Refactor workflow
- `systematic-debugging` - 4-phase debugging process
- `writing-plans` - Implementation planning
- `executing-plans` - Plan execution with checkpoints
- `verification-before-completion` - Pre-delivery verification
- `requesting-code-review` / `receiving-code-review` - Code review workflow
- `brainstorming` - Collaborative idea development
- `clarify-before-coding` - Requirements clarification
- `condition-based-waiting` - Race condition handling
- `context-management` - Context window optimization
- `defense-in-depth` - Multi-layer validation
- `dispatching-parallel-agents` - Parallel investigation
- `finishing-a-development-branch` - Integration decisions
- `forecasting-reverso` - Reversal forecasting
- `refactoring-for-clarity` - Progressive decomposition
- `root-cause-tracing` - Error source identification
- `sharing-skills` - Skill contribution workflow
- `subagent-driven-development` - Independent task execution
- `testing-anti-patterns` - Test quality prevention
- `testing-skills-with-subagents` - Skill testing
- `using-git-worktrees` - Git isolation
- `using-superpowers` - Skill discovery and usage

### Advanced Skills
- `coding-conventions` - Code style guidelines
- `convening-experts` - Multi-expert collaboration
- `crafting-instructions` - Prompt engineering
- `learning-opportunities` - Feedback integration
- `reverse-socratic-examination` - Critical analysis
- `reviewing-ai-papers` - Academic paper analysis
- `skill-creator` - New skill development
- `socratic` - Socratic questioning
- `specification` - Specification writing
- `tiling-tree` - Hierarchical organization
- `writing-clearly-and-concisely` - Clear communication
- `writing-skills` - Skill creation workflow
- `commands` - Command management

## Agent Configuration Examples

### Planner Agent
```json
{
  "name": "planner",
  "description": "Strategic task planning and decomposition",
  "model": "grn_model",
  "tools": [],
  "system_prompt": "You are a strategic planner. Break down complex tasks into specific, actionable subtasks for specialized agents. Be specific, not generic. Don't explore files - that's for execution agents."
}
```

### File Manager Agent
```json
{
  "name": "file_manager",
  "description": "File operations specialist",
  "model": "blu_model",
  "tools": [
    "peek_file_top_10_lines",
    "write_file",
    "edit_file",
    "read_file",
    "list_files",
    "read_pdf",
    "load_skill",
    "list_skills",
    "find_relevant_skills",
    "todo_write",
    "todo_list"
  ],
  "system_prompt": "You are a file management specialist. Handle reading, writing, and organizing files efficiently. Use test-driven-development for code changes."
}
```

### Code Analyzer Agent
```json
{
  "name": "code_analyzer",
  "description": "Code analysis and architecture review",
  "model": "grn_model",
  "tools": [
    "peek_file_top_10_lines",
    "read_file",
    "list_files",
    "search_files",
    "request_more_iterations",
    "load_skill",
    "list_skills",
    "find_relevant_skills",
    "todo_write",
    "todo_list"
  ],
  "system_prompt": "You are a code analysis specialist. Understand code structure, patterns, and architecture. Read-only file access, no command execution."
}
```

### Code Reviewer Agent
```json
{
  "name": "code_reviewer",
  "description": "Senior code reviewer",
  "model": "blu_model",
  "tools": [
    "peek_file_top_10_lines",
    "read_file",
    "list_files",
    "search_files",
    "run_command",
    "request_more_iterations",
    "load_skill",
    "list_skills",
    "find_relevant_skills",
    "todo_write",
    "todo_list"
  ],
  "system_prompt": "You are a senior code reviewer. Review completed work against plans and ensure quality standards. MANDATORY: Use requesting-code-review and receiving-code-review skills. Read-only file access, limited command execution (git, cargo, npm, pytest, go)."
}
```

### Search Specialist Agent
```json
{
  "name": "search_specialist",
  "description": "Code search and discovery",
  "model": "blu_model",
  "tools": [
    "peek_file_top_10_lines",
    "read_file",
    "list_files",
    "search_files",
    "load_skill",
    "list_skills",
    "find_relevant_skills",
    "todo_write",
    "todo_list"
  ],
  "system_prompt": "You are a code search specialist. Find specific code patterns, functions, or files efficiently."
}
```

### System Operator Agent
```json
{
  "name": "system_operator",
  "description": "System operations specialist",
  "model": "blu_model",
  "tools": [
    "run_command",
    "peek_file_top_10_lines",
    "write_file",
    "edit_file",
    "read_file",
    "list_files",
    "plan_edits",
    "apply_edit_plan",
    "load_skill",
    "list_skills",
    "find_relevant_skills",
    "todo_write",
    "todo_list"
  ],
  "system_prompt": "You are a system operations specialist. Run shell commands, manage development workflows, and handle system administration. Use test-driven-development, systematic-debugging, and verification-before-completion skills. Read-write file access, all command execution."
}
```

### Terminal Specialist Agent
```json
{
  "name": "terminal_specialist",
  "description": "PTY session management specialist",
  "model": "blu_model",
  "tools": [
    "pty_launch",
    "pty_send_keys",
    "pty_get_screen",
    "pty_list",
    "pty_kill",
    "pty_get_cursor",
    "pty_resize",
    "pty_set_scrollback",
    "pty_start_capture",
    "pty_stop_capture",
    "pty_request_user_input",
    "peek_file_top_10_lines",
    "read_file",
    "list_files",
    "load_skill",
    "list_skills",
    "find_relevant_skills",
    "todo_write",
    "todo_list"
  ],
  "system_prompt": "You are a terminal specialist. Manage interactive terminal sessions and execute commands that require persistent shell environments. Read-only file access, all command execution."
}
```

### Financial Analyst Agent
```json
{
  "name": "financial_analyst",
  "description": "Financial services specialist",
  "model": "blu_model",
  "tools": [
    "peek_file_top_10_lines",
    "read_file",
    "write_file",
    "edit_file",
    "list_files",
    "search_files",
    "run_command",
    "fetch_url",
    "load_skill",
    "list_skills",
    "find_relevant_skills",
    "todo_write",
    "todo_list",
    "request_more_iterations"
  ],
  "system_prompt": "You are a financial analyst. Build financial models, conduct equity research, perform investment banking analysis, and manage wealth management workflows. Build institutional-quality financial deliverables including comps analysis, DCF models, LBO models, CIMs, earnings analysis, and portfolio management. Read-write file access, python/python3/pip command execution, network access."
}
```

## Tool Categories

### File Operations
- `read_file`, `write_file`, `edit_file`, `peek_file_top_10_lines`, `list_files`
- `plan_edits`, `apply_edit_plan`, `read_pdf`

### Search & Discovery
- `search_files`, `find_relevant_skills`, `list_skills`, `load_skill`

### System & Commands
- `run_command`, `fetch_url`

### Task Management
- `todo_write`, `todo_list`, `request_more_iterations`

### PTY/Terminal
- `pty_launch`, `pty_send_keys`, `pty_get_screen`, `pty_list`, `pty_kill`
- `pty_get_cursor`, `pty_resize`, `pty_set_scrollback`, `pty_start_capture`
- `pty_stop_capture`, `pty_request_user_input`

### Model Management
- `llm_oneshot`, `switch_model`

### Agent Control
- `launch_subagent`, `launch_subagent_pretty`

### Memory & Storage
- `store_memory`, `query_memory`, `update_memory`, `delete_memory`
- `list_memories`

## Build & Test Commands

```bash
# Build all workspace members
cargo build

# Build without warnings
RUSTFLAGS=-Awarnings cargo build

# Release build
cargo build --release

# Build without embeddings feature (faster)
cargo build --no-default-features

# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p apchat-tools
cargo test -p apchat-agents
cargo test -p apchat-llm-api

# Run a specific test
cargo test -p apchat-tools llm_oneshot
cargo test -p apchat-main test_mspc_repl

# Format and lint
cargo fmt
cargo clippy
```

## CLI Options

### Model Configuration
```bash
--api-url-blu-model <URL>        # Custom API URL for blu_model
--api-url-grn-model <URL>        # Custom API URL for grn_model
--api-url-red-model <URL>        # Custom API URL for red_model
--model-blu-model <NAME>          # Override model name for blu_model
--model-grn-model <NAME>          # Override model name for grn_model
--model-red-model <NAME>          # Override model name for red_model
--model <NAME>                    # Base model for all models
--llama-cpp-url <URL>             # Quick llama.cpp setup for all models
--blu-backend <BACKEND>           # Backend (groq, anthropic, llama, openai)
--grn-backend <BACKEND>           # Backend for grn_model
--red-backend <BACKEND>           # Backend for red_model
```

### Mode Selection
```bash
--agents                          # Enable multi-agent system
--task "Your task"                # One-shot task mode
--web                             # Web server mode
--interactive / -i                # Force interactive REPL
```

### Behavior
```bash
--stream                          # Streaming responses (default)
--auto-confirm                    # Skip confirmations
--early-superpowers               # Load all skills at startup
--policy-file <PATH>              # Custom policy file
--learn-policies                  # Learn from user decisions
```

### Web Server
```bash
--web                             # Enable web server
--web-port <PORT>                 # Web server port (default: 8080)
--web-bind <ADDRESS>              # Bind address (default: 127.0.0.1)
--web-attachable                  # Allow TUI session attachment from web
--sessions-dir <PATH>             # Persistent web session storage
```

### Debug
```bash
--verbose / -v                    # Verbose output
--debug <LEVEL>                   # Debug level 0-5
--pretty                          # Pretty-print JSON
```

### Environment Variables
```bash
export GROQ_API_KEY=your_groq_api_key
export ANTHROPIC_AUTH_TOKEN_BLU=your_claude_key_for_blu_model
export ANTHROPIC_AUTH_TOKEN_GRN=your_claude_key_for_grn_model
export ANTHROPIC_AUTH_TOKEN_RED=your_claude_key_for_red_model
export OPENAI_API_KEY=your_openai_api_key
```

## Code Style and Conventions

- Use standard `cargo fmt` formatting
- Clippy lints: `cargo clippy`
- Async/await throughout (tokio runtime)
- `anyhow::Error` for application errors, `thiserror` for library errors
- Workspace dependencies managed in root `Cargo.toml`
- `#[tokio::test]` for async tests

## Git Operations Best Practices

### Safe Rebasing
```bash
GIT_EDITOR=true git rebase --continue
GIT_EDITOR=true git cherry-pick --continue
```

### Commit Guidelines
- Verify `git status` before committing
- Group logically related files together
- Always try normal push first; only `--force-with-lease` after user confirmation

## Adding New Components

### Adding a New Tool
See [docs/dev/how_to_new_tool.md](../dev/how_to_new_tool.md) for detailed guide.

### Adding a New Agent
See [docs/dev/CUSTOMIZING_AGENTS_AND_SKILLS.md](../dev/CUSTOMIZING_AGENTS_AND_SKILLS.md).

### Adding a New Skill
See [docs/dev/CUSTOMIZING_AGENTS_AND_SKILLS.md](../dev/CUSTOMIZING_AGENTS_AND_SKILLS.md).