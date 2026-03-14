# AGENTS.md

## Important Notes - MUST FOLLOW!

**Conversation History**: Check existing conversation history before deciding whether to perform operations - avoid redundant calls
**File Operations**: Use specific patterns like `"src/*.rs"` instead of `"*.rs"` to locate files in the src directory
**Repeat operations**: If your history already has a file read, do not read it again - as this will overload the history. Likewise, if you are doing an edit - do not attempt to do it multiple times, if something fails, ask the user to verify.

**Tools Documentation**: For best practices on using tools efficiently (especially the `read_file` tool), see [docs/tools/README.md](docs/tools/README.md) for an overview, or [docs/tools/QUICK_REFERENCE.md](docs/tools/QUICK_REFERENCE.md) for immediate guidance.

## Useful shortcuts

### Build the project without warnings
```bash
RUSTFLAGS=-Awarnings cargo build
```

### Run the application
```bash
cargo run 
```

### Run inside the pty tool, with a real model behind, to be able to interact:
```bash
cargo run -- --stream --interactive --llama-cpp-url http://127.0.0.1:4000/v1/ --auto-confirm
```

## Tool Usage Best Practices

### 🚀 Subagents: Your Best Weapon Against Context Rot

**Use `launch_subagent_pretty` liberally!** Subagents are independent workers that return clean, summarized JSON output. This dramatically reduces context window usage.

#### When to Use Subagents

- **Investigating code**: "Find all usages of function X", "Describe the architecture of module Y"
- **Independent subtasks**: Any self-contained task that doesn't require shared state
- **Information gathering**: Research, documentation lookup, file analysis
- **Parallel work**: Multiple independent investigations (one subagent at a time, wait for results)

#### Why Subagents Save Context

- ✅ Return **summarized results** instead of raw tool output
- ✅ Run **independently** - their internal conversation doesn't pollute your context
- ✅ Provide **structured JSON** output that's easy to parse
- ✅ Can be **launched with specific tasks** and return focused answers

#### Example Usage

Instead of:
1. Reading multiple files yourself (context bloat)
2. Running multiple commands (more output)
3. Trying to remember what you found

**Launch a subagent**:
```
Task: "Find all functions in src/ that handle authentication and summarize their purpose"
```

The subagent will explore the codebase and return a clean summary like:
```json
{
  "functions_found": 3,
  "summary": "Auth handled by: auth::login(), auth::validate_token(), auth::logout()",
  "files_modified": ["src/auth.rs"]
}
```

**This is your primary defense against context rot!**

### read_file Tool

#### IMPORTANT: Use 20-line chunks for large files!

**Do NOT read entire large files at once.** The `read_file` tool has a default behavior of reading the entire file, which can cause:
1. **Output truncation** - Very large files may be cut off
2. **Tool call overhead** - Reading full files costs extra iterations
3. **Context bloat** - Unnecessary content competes with conversation history limits

#### Recommended Usage Pattern

```rust
// ❌ BAD - Reads entire file (potential issues above)
read_file {
  "file_path": "src/main.rs"
}

// ✅ GOOD - Read in 20-line chunks
read_file {
  "file_path": "src/main.rs",
  "limit": 20
}

// ✅ GOOD - Read specific range
read_file {
  "file_path": "src/main.rs",
  "offset": 101,
  "limit": 30
}
```

#### Pro Tip: Combine with curly_glance

For large code files, use `file_curly_glance` to find sections of interest, then use `read_file` with line ranges to read specific parts:

```rust
// Step 1: Get overview
file_curly_glance {
  "file_path": "src/main.rs"
}

// Step 2: Drill into specific section (e.g., line 42)
file_curly_glance {
  "file_path": "src/main.rs",
  "starting_line": 42
}

// Step 3: Read detailed content in chunks
read_file {
  "file_path": "src/main.rs",
  "offset": 42,
  "limit": 25
}
```

This iterative approach is **far more efficient** than reading entire files!

## Project Overview

APChat is a sophisticated Rust-based AI development assistant with two operational modes:

1. **Single LLM Mode** (default) - Direct conversation with one LLM with full tool access
2. **Multi-Agent Mode** (`--agents` flag) - Planner-first architecture where a planning agent decomposes tasks and delegates to specialized agents

The project is organized as a Rust workspace with **24 crates** and **~100,000 lines** of Rust code. It supports 4 LLM providers (Groq, Anthropic Claude, OpenAI, llama.cpp) and includes **50+ tools**, **8 specialized agents**, and **38 skills** (proven workflows).

## Skills System (MANDATORY Workflows)

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

### Core Skills (Always Available)

- `using-superpowers` - Skill discovery and usage
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
- `finishing-a-development-branch` - Integration decisions
- `refactoring-for-clarity` - Progressive decomposition
- `root-cause-tracing` - Error source identification
- `sharing-skills` - Skill contribution workflow
- `subagent-driven-development` - Independent task execution
- `testing-anti-patterns` - Test quality prevention
- `testing-skills-with-subagents` - Skill testing
- `using-git-worktrees` - Git isolation

**See [docs/agents/REFERENCE.md](docs/agents/REFERENCE.md) for complete skills list (38 total)**

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

### 8. Financial Analyst Agent (`financial_analyst.json`)
- **Role**: Financial services specialist for building financial models, equity research, investment banking analysis, and wealth management workflows
- **Model**: blu_model
- **Tools**: peek_file_top_10_lines, read_file, write_file, edit_file, list_files, search_files, run_command, fetch_url, load_skill, list_skills, find_relevant_skills, todo_write, todo_list, request_more_iterations
- **Capabilities**: Financial analysis, financial modeling, equity research, investment banking, private equity, wealth management
- **Skills**: Uses financial services skills (fsi-* prefix skills)
- **Permissions**: Read-write file access, python/python3/pip command execution, network access
- **Purpose**: Building institutional-quality financial deliverables including comps analysis, DCF models, LBO models, CIMs, earnings analysis, and portfolio management

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

## Quick Reference Links

- **Build/Test Commands**: See [docs/agents/QUICKSTART.md](docs/agents/QUICKSTART.md)
- **Configuration & CLI Options**: See [docs/agents/CONFIG.md](docs/agents/CONFIG.md)
- **Complete Skills List**: See [docs/agents/REFERENCE.md](docs/agents/REFERENCE.md)
- **Tool Documentation**: See [docs/tools/README.md](docs/tools/README.md)
- **Architecture Details**: See [docs/project/CLAUDE.md](docs/project/CLAUDE.md)