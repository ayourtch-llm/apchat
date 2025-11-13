# KimiChat - Multi-Agent AI CLI

## Project Overview

KimiChat is a Rust CLI application providing a Claude Code-like experience with multi-agent AI orchestration. It supports multiple LLM backends (Groq, Anthropic, llama.cpp) and features a sophisticated planner-first architecture for complex task handling.

## Core Architecture

### Multi-Agent System

**Planner-First Flow:**
1. User request → PlanningCoordinator
2. Planner agent analyzes and decomposes request into subtasks
3. Planner assigns subtasks to specialized agents
4. Agents execute tasks using their available tools
5. Results are synthesized into final response

**Specialized Agents:**
- `planner` - Task decomposition and agent assignment
- `code_analyzer` - Code analysis and architecture review
- `file_manager` - File operations (read, write, edit)
- `search_specialist` - Search and discovery across codebase
- `system_operator` - Command execution, builds, and batch operations

Each agent has:
- Specific tool access (defined in `agents/configs/*.json`)
- Iteration limits with dynamic extension via `request_more_iterations` tool
- System prompts tuned for their specialty

### Key Design Patterns

**Tool Registry:**
- Central registry (`src/core/tool_registry.rs`)
- Tools implement `Tool` trait
- Categories: file_ops, search, system, model_management, agent_control

**Agent Factory:**
- Creates agents from JSON configurations
- Filters tools by agent's allowed list
- Handles tool execution loop with iteration management

**Visibility System:**
- Tracks task execution hierarchically
- Shows task progress and agent assignments
- Phases: Planning → AgentSelection → TaskExecution → Aggregation → Completed

## Project Structure

### Core Modules

```
src/
├── main.rs              # KimiChat struct, main entry point
├── app/                 # Application modes (setup, REPL, task)
├── agents/              # Multi-agent orchestration system
│   ├── coordinator.rs   # Planning and task distribution
│   ├── agent_factory.rs # Agent creation and execution loop
│   └── visibility.rs    # Task tracking and progress display
├── api/                 # LLM API clients (streaming/non-streaming)
├── chat/                # Conversation management
│   ├── session.rs       # Main chat loop
│   ├── history.rs       # History summarization
│   └── state.rs         # State persistence
├── config/              # Configuration management
│   └── helpers.rs       # API URL/key resolution
├── tools/               # Tool implementations
│   ├── file_ops.rs      # File operations with confirmations
│   ├── search.rs        # Code search
│   ├── system.rs        # Command execution
│   └── iteration_control.rs # Dynamic iteration requests
├── models/              # Data structures and types
└── logging/             # Request/response logging (JSONL format)
```

### Agent Configurations

Agent configs in `agents/configs/*.json` define:
- Model selection
- Available tools
- System prompts
- Tool access permissions

## CLI Usage

```bash
# Interactive mode with multi-agent system
cargo run -- --agents -i

# Interactive mode with single LLM
cargo run -- -i

# One-off task with agents
cargo run -- --agents --task "analyze the codebase"

# Stream responses (default)
cargo run -- --agents -i --stream

# Auto-confirm all actions
cargo run -- --agents -i --auto-confirm

# Use custom llama.cpp server
cargo run -- --llama-cpp-url http://localhost:8080 -i
```

## Key Features

### Tool Confirmations
- File edits show unified diffs before applying
- Commands require confirmation before execution
- Batch edits (`plan_edits`/`apply_edit_plan`) validate all changes first
- Optional user feedback on rejection

### Iteration Management
- Default 10 iterations per agent (prevents infinite loops)
- Warnings at iteration 8+
- Agents can request more iterations with justification via `request_more_iterations` tool
- Dynamic limit adjustment mid-execution

### Smart Error Handling
- Tool call repair using AI for malformed JSON
- XML parsing support for models that prefer XML format
- Automatic line range clamping for file operations
- Model switching on API errors

### Logging
- JSONL format in `logs/` directory
- Captures full conversation history
- Task context (task_id, parent_task_id, agent_name)
- Request/response logging with timestamps

## Working with the Codebase

### Adding New Agents
1. Create config in `agents/configs/your_agent.json`
2. Define tools, model, and system prompt
3. Agent will be automatically loaded on startup

### Adding New Tools
1. Implement `Tool` trait in `src/tools/`
2. Register in tool registry initialization
3. Add to relevant agent configs

### Modifying Agent Behavior
- Edit system prompts in agent config files
- Adjust tool permissions in `allowed_tools` list
- Configure iteration limits (future: via config)

## Configuration

### Environment Variables
- `GROQ_API_KEY` - Groq API access
- `ANTHROPIC_API_KEY` - Anthropic API access

### CLI Options
- `--agents` - Enable multi-agent system
- `--api-url-*-model` - Custom API endpoints
- `--model-*-model` - Override model names
- `--policy-file` - Custom policy file path
- `--learn-policies` - Learn from user decisions
- `--auto-confirm` - Skip all confirmations
- `--verbose` - Debug output

## Design Philosophy

1. **Explicit over Implicit** - Direct module calls rather than wrapper methods
2. **Modularity** - Single-responsibility modules with clear boundaries
3. **Safety** - Confirmations for destructive operations
4. **Extensibility** - Easy to add new agents, tools, and models
5. **Transparency** - Detailed logging and progress visibility

## Notes

- Planner agent is used only for planning, not execution
- Tool execution is async with proper error handling
- Conversation state persists across tool calls
- All file operations respect gitignore patterns
- Batch edits use file-based state (`.kimichat_edit_plan.json`)

For detailed refactoring history, see `REFACTORING_SUMMARY.md`.
