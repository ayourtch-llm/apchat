# AGENTS.md

## Project Overview

APChat is a sophisticated Rust-based AI development assistant with two operational modes:

1. **Single LLM Mode** (default) - Direct conversation with one LLM with full tool access
2. **Multi-Agent Mode** (`--agents` flag) - Planner-first architecture where a planning agent decomposes tasks and delegates to specialized agents

The project is organized as a Rust workspace with 15 crates and ~11,270 lines of Rust code. It supports 4 LLM providers (Groq, Anthropic Claude, OpenAI, llama.cpp) and includes 26 tools, 7 specialized agents, and 21 mandatory skills (proven workflows).

## Workspace Structure

```
apchat/
├── apchat-main/           # Main binary crate (CLI, REPL, web server)
├── crates/
│   ├── apchat-llm-api/   # Unified LLM client interface
│   ├── apchat-logging/    # Conversation logging (JSONL)
│   ├── apchat-models/     # Data structures and types
│   ├── apchat-mspc/       # Message passing channels
│   ├── apchat-policy/     # Security and approval system
│   ├── apchat-progress/    # Progress tracking for streaming
│   ├── apchat-skills/     # Skill registry and loading
│   ├── apchat-terminal/    # PTY session management with VT100
│   ├── apchat-todo/       # Task tracking
│   ├── apchat-toolcore/   # Tool execution framework
│   ├── apchat-tools/      # 26 tool implementations
│   ├── apchat-vty/        # VTY/readline abstractions
│   ├── apchat-wasm/       # WebAssembly frontend
│   └── apchat-webex/     # Webex bot integration
├── agents/configs/        # Agent JSON configurations (7 agents)
├── skills/               # Skill definitions (21 skills)
├── hooks/                # Session lifecycle hooks
└── docs/                 # Extensive documentation
```

## Build Commands

```bash
# Build all workspace members
cargo build

# Release build with optimizations
cargo build --release

# Build without embeddings feature (faster builds)
cargo build --no-default-features

# Build WebAssembly frontend
cd crates/apchat-wasm && ./build.sh
```

## Test Commands

```bash
# Run all tests in workspace
cargo test

# Run tests for a specific crate
cargo test -p apchat-tools
cargo test -p apchat-agents
cargo test -p apchat-llm-api

# Run a specific test
cargo test -p apchat-tools llm_oneshot
cargo test -p apchat-main test_mspc_repl

# Run tests with output
cargo test -- --nocapture

# Run specific test with output
cargo test -p apchat-main test_mspc_repl -- --nocapture
```

## Development Scripts

```bash
# Verify MSPC (Message Passing) integration
./verify_mspc.sh

# Test PTY terminal functionality
./run_pty_test.sh

# Startup verification
./test_startup.sh

# Task mode tests
./apchat-main/test_task5_integration.sh
```

## Running the Application

```bash
# Interactive REPL mode with single LLM
cargo run -- -i

# Interactive mode with multi-agent system
cargo run -- --agents -i

# One-shot task mode
cargo run -- --task "analyze the codebase"

# Web server mode
cargo run -- --web --bind 127.0.0.1:8080

# With streaming (default)
cargo run -- -i --stream

# Auto-confirm all operations (skip confirmations)
cargo run -- -i --auto-confirm
```

## Code Style and Conventions

### Rust Code Style
- Use standard `cargo fmt` formatting
- Clippy lints: `cargo clippy`
- No comments unless explicitly requested
- Async/await throughout the codebase (tokio runtime)
- Proper error handling with `Result<T, E>` types
- Trait-based abstractions for extensibility

### Error Handling
- Use `anyhow::Error` for application errors
- Use `thiserror` for library errors with proper error variants
- Provide context with `.context()` or `.with_context()` from anyhow

### Testing Patterns
- Unit tests alongside implementation in `src/`
- Integration tests in `tests/` directories
- E2E tests via shell scripts in project root
- Test utilities: `tempfile`, `mockall`, `wiremock`, `pretty_assertions`
- Use `#[tokio::test]` for async tests

### Dependencies Management
- Workspace dependencies managed in root `Cargo.toml` `[workspace.dependencies]`
- Always check if dependencies already exist before adding new ones
- Prefer using workspace versions: `tokio = { workspace = true }`

## Architecture Patterns

### Tool System

**Central Registry Pattern** (`apchat-toolcore/src/tool_registry.rs`):
- All tools implement the `Tool` trait
- Registered at startup in a central registry
- Filtered by agent permissions in multi-agent mode
- Categories: file_ops, search, system, model_management, agent_control, skills

**Tool Execution Flow**:
1. LLM requests tool call (JSON format with tool name and arguments)
2. Registry looks up tool by name
3. Tool validates arguments against schema
4. Policy system checks permissions (Allow/Deny/Ask)
5. Tool executes with access to `ToolContext`
6. Results returned to LLM for next iteration

### Skills System (MANDATORY Workflows)

Skills are proven workflows that agents MUST follow when applicable. This is non-negotiable.

**Three Access Patterns**:
1. **Agent Tools**: `load_skill`, `list_skills`, `find_relevant_skills`
2. **User Slash Commands**: `/brainstorm`, `/write-plan`, `/execute-plan`
3. **Session Hook**: `hooks/session-start.sh` injects `using-superpowers` skill at startup

**Implementation**:
- Skills loaded from `skills/*/SKILL.md` files
- YAML frontmatter for metadata
- SkillRegistry shared across agents via `Arc<SkillRegistry>`
- Agents have mandatory skill-checking in system prompts

**Common Skills**:
- `test-driven-development` - Red-Green-Refactor workflow
- `systematic-debugging` - 4-phase debugging process
- `writing-plans` - Implementation planning
- `executing-plans` - Plan execution with checkpoints
- `verification-before-completion` - Pre-delivery verification
- `requesting-code-review` / `receiving-code-review` - Code review workflow

### Agent Configuration System

Agents are **not hardcoded** - they're defined by JSON configs in `agents/configs/*.json`:

```json
{
  "name": "file_manager",
  "description": "Handles file operations",
  "model": "blu_model",
  "tools": ["read_file", "write_file", "edit_file", "list_files"],
  "system_prompt": "You are a file management specialist..."
}
```

**Dynamic Discovery**: Planner discovers available agents at runtime by reading all configs. Adding new agents requires only creating a JSON file - no code changes.

### Terminal/PTY Architecture

**Background Reader Pattern** (`apchat-terminal/src/session.rs`):
- Each PTY session spawns a background thread that continuously reads output
- Thread updates a shared screen buffer (protected by `Arc<Mutex<ScreenBuffer>>`)
- Tools read from the buffer without manual polling
- Graceful shutdown prevents thread hangs

**VT100 Emulation** (`apchat-terminal/src/screen_buffer.rs`):
- Interprets ANSI escape sequences
- Maintains cursor position, scrollback buffer
- Tools can capture screen content, cursor position, scrollback

## Specialized Agents

### 1. Planner Agent (`planner.json`)
- **Role**: Strategic task planning and decomposition
- **Model**: grn_model
- **Tools**: None (planning only)
- **Purpose**: Breaks down user requests into specific, actionable subtasks for specialized agents
- **Key Principle**: Be specific, not generic. Don't explore files - that's for execution agents.

### 2. File Manager Agent (`file_manager.json`)
- **Role**: File operations specialist
- **Model**: blu_model
- **Tools**: peek_file_top_10_lines, write_file, edit_file, read_file, list_files, read_pdf, load_skill, list_skills, find_relevant_skills, todo_write, todo_list
- **Purpose**: Reading, writing, and organizing files efficiently
- **Skills**: Must use test-driven-development for code changes

### 3. Code Analyzer Agent (`code_analyzer.json`)
- **Role**: Code analysis and architecture review
- **Model**: grn_model
- **Tools**: peek_file_top_10_lines, read_file, list_files, search_files, request_more_iterations, load_skill, list_skills, find_relevant_skills, todo_write, todo_list
- **Permissions**: Read-only file access, no command execution
- **Purpose**: Understanding code structure, patterns, and architecture

### 4. Code Reviewer Agent (`code_reviewer.json`)
- **Role**: Senior code reviewer
- **Model**: blu_model
- **Tools**: peek_file_top_10_lines, read_file, list_files, search_files, run_command, request_more_iterations, load_skill, list_skills, find_relevant_skills, todo_write, todo_list
- **Permissions**: Read-only file access, limited command execution (git, cargo, npm, pytest, go)
- **Skills**: MANDATORY: requesting-code-review, receiving-code-review
- **Purpose**: Review completed work against plans and ensure quality standards

### 5. Search Specialist Agent (`search_specialist.json`)
- **Role**: Code search and discovery
- **Model**: blu_model
- **Purpose**: Finding specific code patterns, functions, or files

### 6. System Operator Agent (`system_operator.json`)
- **Role**: System operations specialist
- **Model**: blu_model
- **Tools**: run_command, file operations, plan_edits, apply_edit_plan, load_skill, list_skills, find_relevant_skills, todo_write, todo_list
- **Permissions**: Read-write file access, all command execution
- **Skills**: test-driven-development, systematic-debugging, verification-before-completion
- **Purpose**: Running shell commands, managing development workflows, system administration

### 7. Terminal Specialist Agent (`terminal_specialist.json`)
- **Role**: PTY session management specialist
- **Model**: blu_model
- **Tools**: 11 PTY tools (pty_launch, pty_send_keys, pty_get_screen, pty_list, pty_kill, pty_get_cursor, pty_resize, pty_set_scrollback, pty_start_capture, pty_stop_capture, pty_request_user_input), peek_file_top_10_lines, read_file, list_files, load_skill, list_skills, find_relevant_skills, todo_write, todo_list
- **Permissions**: Read-only file access, all command execution
- **Purpose**: Managing interactive terminal sessions and executing commands that require persistent shell environments

## Important Implementation Details

### Conversation History Management

**Summarization** (`apchat-main/src/chat/history.rs`):
- Triggers when conversation exceeds 200KB
- AI-powered summarization preserves context while reducing size
- Maintains recent messages with full detail

### Iteration Control

**Multi-Agent Mode**:
- Default 50 iterations per agent (prevents infinite loops)
- Warnings at iteration 47+
- Agents can request more via `request_more_iterations` tool with justification
- Dynamic limit adjustment mid-execution

### Tool Confirmations

**Policy-Based Approvals** (`apchat-policy/`):
- File edits show unified diffs before execution
- Commands can require approval based on patterns
- Three policy types: Allow, Deny, Ask
- Glob patterns for files, string patterns for commands

### Loop Detection

**Enhanced Detection** (`apchat-main/src/tools_execution/`):
- Separate thresholds for consecutive vs scattered repeats
- Leniency for read-only operations (searches, file reads)
- Write operations flagged more aggressively

### MSPC Integration

**Message Passing for Readline** (`apchat-main/src/mspc/`):
- Recent architectural change to decouple input handling
- Multi-producer, single-consumer channels for readline coordination
- Prevents race conditions in REPL input

## Configuration and Environment

### Required Environment Variables

```bash
# For Groq models
export GROQ_API_KEY=your_groq_api_key

# For Anthropic Claude (supports multiple model slots)
export ANTHROPIC_AUTH_TOKEN_BLU=your_claude_key_for_blu_model
export ANTHROPIC_AUTH_TOKEN_GRN=your_claude_key_for_grn_model
export ANTHROPIC_AUTH_TOKEN_RED=your_claude_key_for_red_model

# For OpenAI
export OPENAI_API_KEY=your_openai_api_key
```

### Model Configuration

Three model slots available:
- **BluModel** - Primary model for most agents
- **GrnModel** - Model for planning and code analysis
- **RedModel** - Additional model slot

### CLI Options

```bash
# Model configuration
--api-url-blu-model <URL>        # Custom API URLs
--model-blu-model <NAME>          # Override model names
--llama-cpp-url <URL>             # Quick llama.cpp setup

# Mode selection
--agents                          # Enable multi-agent system
--task "Your task"                # One-shot task mode
--web                             # Web server mode
--interactive / -i                # Force interactive REPL

# Behavior
--stream                          # Streaming responses (default)
--auto-confirm                    # Skip confirmations
--policy-file <PATH>              # Custom policy file
--learn-policies                  # Learn from user decisions

# Debug
--verbose                         # Verbose output
--debug <LEVEL>                   # Debug level 0-5
--pretty                          # Pretty-print JSON
```

## Adding New Components

### Adding a New Tool

1. Create tool in appropriate crate (`apchat-tools/src/your_tool.rs`)
2. Implement the `Tool` trait:
   ```rust
   #[async_trait]
   impl Tool for YourTool {
       fn name(&self) -> &str { "your_tool" }
       fn description(&self) -> &str { "Does something useful" }
       fn parameters(&self) -> serde_json::Value { /* JSON schema */ }
       async fn execute(&self, args: &Value, context: &ToolContext) -> Result<ToolResult> {
           // Implementation
       }
   }
   ```
3. Register in `apchat-main/src/main.rs` tool registry initialization
4. Add to relevant agent configs in `agents/configs/*.json`

See `docs/dev/how_to_new_tool.md` for detailed guide.

### Adding a New Agent

1. Create JSON config in `agents/configs/your_agent.json`
2. Define name, description, model, tools, and system_prompt
3. Agent is automatically discovered and available to planner
4. No code changes required

### Adding a New Skill

1. Create directory `skills/your-skill/`
2. Create `SKILL.md` with YAML frontmatter:
   ```markdown
   ---
   name: your-skill
   description: Brief description
   ---

   # Detailed workflow instructions
   ```
3. Skill is automatically discovered at startup
4. Optionally create slash command in `.claude/commands/` for user access

## Testing Strategy

**Unit Tests**: In `src/` alongside implementation
**Integration Tests**: In `tests/` directories
**E2E Tests**: Shell scripts in project root

Focus areas with test coverage:
- LLM API model configuration (`apchat-llm-api/src/tests/`)
- Tool execution and registry (`apchat-toolcore/tests/`)
- File operations and edit planning (`apchat-tools/tests/`)
- MSPC readline integration (`apchat-main/tests/test_mspc_*.rs`)

## Linting and Type Checking

```bash
# Format code
cargo fmt

# Run Clippy linter
cargo clippy

# Fix Clippy warnings automatically
cargo clippy --fix

# Check for unused dependencies
cargo +nightly udeps
```

## Common Workflows

### Working on File Operations
1. Always use `find_relevant_skills` before starting
2. If doing code changes, use `test-driven-development` skill
3. Use `todo_write` for multi-step tasks (3+ steps)
4. Use `plan_edits` + `apply_edit_plan` for complex multi-file edits

### Running Tests After Changes
1. Run `cargo test` for all tests
2. Run `cargo clippy` for linting
3. Fix any issues before completing

### Debugging Issues
1. Use `systematic-debugging` skill
2. Check logs in `logs/` directory (JSONL format)
3. Use `--debug 5` flag for verbose output

### Code Review Process
1. Use `code_reviewer` agent after major work
2. Agent automatically loads `requesting-code-review` and `receiving-code-review` skills
3. Reviewer checks plan alignment, code quality, tests, and documentation

## Security Considerations

- Policy system restricts file access and command execution
- File edits always show diffs before execution
## Git Operations

### Rebasing and Cherry-Picking

When using `git rebase --continue` or `git cherry-pick --continue`, set `GIT_EDITOR=true` to avoid interactive editor prompts:

```bash
GIT_EDITOR=true git rebase --continue
GIT_EDITOR=true git cherry-pick --continue
```

This accepts the default commit message without opening vim/nano.

### Pushing After Rebase

After rebasing, always try a normal push first:

```bash
git push origin main
```

If it fails with "non-fast-forward", **ask the user** before using `--force-with-lease`. Force pushing rewrites history and should only be done with explicit confirmation.

### Commit Best Practices

- **Double-check directories** before committing - verify `git status` shows only intended files
- **Group logically related files** together in single commits for cleaner history
- **Avoid committing large directories/files** without explicit user confirmation
- **Check file sizes** with `git add --dry-run` before staging
- Use environment variables for sensitive configuration

## Documentation

**For comprehensive architecture details**: See `docs/project/CLAUDE.md`
**Tool documentation**: See `docs/tools/` directory
**Development guides**: See `docs/dev/` directory
**Architecture decisions**: See `docs/architecture/` directory

Key documentation files:
- `docs/architecture/TERMINAL_SESSIONS_DESIGN.md` - PTY architecture
- `docs/architecture/REFACTORING_SUMMARY.md` - Main.rs refactoring
- `docs/dev/how_to_new_tool.md` - Tool addition guide
- `docs/dev/CUSTOMIZING_AGENTS_AND_SKILLS.md` - Agent/skill configuration
