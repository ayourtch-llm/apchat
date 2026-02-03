# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

### Building the Project

```bash
# Build all workspace members
cargo build

# Build with release optimizations
cargo build --release

# Build without embeddings feature (faster builds, no semantic skill search)
cargo build --no-default-features
```

### Running Tests

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
```

### Running the Application

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

### Development Scripts

```bash
# Verify MSPC (Message Passing) integration
./verify_mspc.sh

# Test PTY terminal functionality
./run_pty_test.sh

# Build WebAssembly frontend
cd crates/apchat-wasm && ./build.sh
```

## Architecture Overview

APChat is a sophisticated Rust-based AI development assistant with two distinct operational modes:

### 1. Single LLM Mode (Default)
Direct conversation with one LLM that has full tool access. Suitable for most tasks and straightforward interactions.

### 2. Multi-Agent Mode (`--agents` flag)
Planner-first architecture where a planning agent decomposes tasks and delegates to specialized agents:
- **planner** - Task decomposition (no tools, dynamically discovers available agents)
- **code_analyzer** - Code analysis and architecture review
- **code_reviewer** - Code review with mandatory skill-based workflows
- **file_manager** - File operations
- **search_specialist** - Code search and discovery
- **system_operator** - Command execution and builds
- **terminal_specialist** - PTY/terminal session management

### Workspace Structure

```
apchat/
├── apchat-main/          # Main binary, CLI, REPL, web server
├── crates/
│   ├── apchat-agents/    # Multi-agent orchestration and coordination
│   ├── apchat-llm-api/   # Unified LLM client (Groq, Anthropic, OpenAI, llama.cpp)
│   ├── apchat-models/    # Data structures and types
│   ├── apchat-toolcore/  # Tool execution framework
│   ├── apchat-tools/     # 20+ tool implementations
│   ├── apchat-terminal/  # PTY session management with VT100 emulation
│   ├── apchat-skills/    # Skill registry and loading
│   ├── apchat-policy/    # Security and approval system
│   ├── apchat-logging/   # Conversation logging (JSONL)
│   ├── apchat-todo/      # Task tracking
│   └── apchat-wasm/      # WebAssembly frontend
├── agents/configs/       # Agent JSON configurations (dynamically loaded)
├── skills/               # 20+ proven workflow patterns (SKILL.md files)
└── hooks/                # Session lifecycle hooks
```

## Key Architectural Patterns

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

### Skills System (MANDATORY workflows)

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

### Agent Configuration System

Agents are **not hardcoded** - they're defined by JSON configs in `agents/configs/*.json`:

```json
{
  "name": "file_manager",
  "description": "Handles file operations",
  "model": "GrnModel",
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

## Additional Documentation

**For comprehensive architecture details**: See `docs/project/CLAUDE.md`
**Tool documentation**: See `docs/tools/` directory
**Development guides**: See `docs/dev/` directory
**Architecture decisions**: See `docs/architecture/` directory
