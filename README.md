# APChat - AI-powered Development Assistant

A sophisticated Rust-based AI development assistant that provides a Claude Code-like experience with multi-model support, rich tool integration, web API, and terminal management capabilities. Built for developers who need powerful AI assistance with fine-grained control and extensibility.

## Overview

APChat is a production-ready AI assistant that seamlessly integrates multiple LLM providers (Groq, Anthropic Claude, OpenAI, and llama.cpp) with a comprehensive toolset for file operations, terminal interaction, task management, and multi-agent workflows. Whether you're working in CLI mode, task mode, or through a web interface, APChat adapts to your development workflow.

## Key Features

### 🤖 Multi-Model & Multi-Provider Support
- **Four LLM Providers**: Groq, Anthropic Claude, OpenAI, and llama.cpp (local inference)
- **Flexible Model Slots**: BluModel, GrnModel, RedModel with per-slot configuration and custom model support
- **Intelligent Model Switching**: Models can autonomously switch based on task requirements
- **Streaming Support**: Real-time response streaming for all providers
- **Automatic Backend Detection**: Smart provider selection from API URLs
- **Unified Model Syntax**: `model@backend[url]` syntax for easy configuration

### 🛠️ Comprehensive Tool System (67+ Tools)

#### File Operations
- **read_file** - Display file contents with optional line ranges
- **peek_file_top_10_lines** - Quick file preview (first 10 lines)
- **write_file** - Create and write files to workspace
- **edit_file** - Edit files with old/new content replacement (with unified diff preview)
- **list_files** - List files matching glob patterns
- **file_curly_glance** - Quick structural overview of source files
- **plan_edits** - Plan batch edits with unified diff format previews
- **apply_edit_plan** - Apply pre-planned edit operations atomically
- **diff_fuzz** - Generate fuzz tests based on code analysis

#### Document Processing
- **read_pdf** - Extract text from PDF files
- **read_image** - Encode images to base64 for multimodal LLMs (JPEG, PNG, WebP, BMP)
- **read_pptx** - Read existing PowerPoint presentations and extract content
- **rlm_context_chunk** - Split large files/directories for Recursive Language Model processing

#### PowerPoint/PPTX Tools
- **create_presentation** - Create new PowerPoint presentations from scratch
- **edit_pptx_slide** - Edit slide content (text, bullets, layouts)
- **set_slide_background** - Set background colors for slides
- **add_slide_to_pptx** - Add new slides to existing presentations
- **remove_slide_from_pptx** - Remove slides from presentations
- **apply_pptx_style** - Apply templates and styling to presentations
- **clone_pptx_template** - Clone presentations using templates

#### Search & Analysis
- **search_files** - Full-text search with regex, glob patterns, and `.gitignore` support
- **web_search** - Web search via SearXNG metasearch engine
- **project_analysis** - Analyze project structure, dependencies, and file types

#### Web
- **fetch_url** - Fetch content from URLs (auto, raw, JSON, markdown formats)

#### LLM & AI
- **llm_oneshot** - One-shot LLM calls for simple tasks without agent overhead
- **python_sandbox** - Execute Python code in sandboxed environment (ouros)
- **forecast** - Time series forecasting and prediction (Reverso)
- **mcp_client** - MCP (Model Context Protocol) client for tool integration

#### Metacognition
- **become** - Transform into specialized expert persona
- **drugs** - Apply cognitive enhancement patterns
- **ritual** - Execute structured workflows and ceremonies
- **self_regulate** - Self-monitoring and adjustment

#### Context Management
- **save_full_state** - Save complete conversation and workspace state
- **load_full_state** - Restore previously saved state
- **edit_item** - Edit conversation items
- **delete_items** - Remove conversation items

#### Terminal Management (PTY-based)
- **pty_launch** - Launch new terminal sessions
- **pty_send_keys** - Send keyboard input to terminals
- **pty_send_credential_keys** - Send sensitive input (e.g., passwords) to terminals
- **pty_get_screen** - Capture terminal screen content
- **pty_get_cursor** - Get cursor position
- **pty_resize** - Resize terminal dimensions
- **pty_set_scrollback** - Configure scrollback buffer
- **pty_start_capture** / **pty_stop_capture** - Output capture control
- **pty_list** - List active sessions (max 15 concurrent)
- **pty_kill** - Terminate sessions
- **pty_request_user_input** - Request user input

**Terminal Features**:
- VT100/ANSI escape sequence interpretation
- Persistent screen buffer (1000 lines default)
- Multiple backend support (native PTY, tmux)
- Session logging

#### Memory & Knowledge
- **store_memory** - Store key-value pairs in persistent memory
- **query_memory** - Query stored memories
- **update_memory** - Update existing memories
- **delete_memory** - Delete memories
- **list_memories** - List all stored memories

#### Scheduled Instructions
- **add_scheduled_instruction** - Create time-based instructions
- **list_scheduled_instructions** - List scheduled instructions
- **delete_scheduled_instruction** - Remove scheduled instructions

#### Task & Workflow Management
- **todo_write** - Create and manage task lists with status tracking (pending, in_progress, completed)
- **todo_list** - View task progress
- **load_skill** - Load proven workflow patterns
- **list_skills** - Discover available skills
- **find_relevant_skills** - AI-powered skill discovery with semantic search

**Available Skills** (37 curated workflows):
- Brainstorming, Clarify Before Coding, Coding Conventions
- Commands, Condition-based Waiting, Context Management
- Convening Experts, Crafting Instructions
- Defense-in-Depth, Dispatching Parallel Agents
- Executing Plans, Finishing a Development Branch
- Forecasting Reverso, Learning Opportunities
- Receiving Code Review, Recursive Context Processing
- Refactoring for Clarity, Requesting Code Review
- Reverse Socratic Examination, Reviewing AI Papers
- Root Cause Tracing, Sharing Skills, Skill Creator
- Socratic, Specification, Subagent-Driven Development
- Systematic Debugging, Test-Driven Development
- Testing Anti-Patterns, Testing Skills with Subagents
- Tiling Tree, Using Git Worktrees, Using Superpowers
- Verification Before Completion, Writing Clearly and Concisely
- Writing Plans, Writing Skills

#### Subagents
- **launch_subagent** - Launch specialized subagents for delegated tasks
- **launch_subagent_pretty** - Launch subagents with formatted output

#### System & Control
- **run_command** - Execute shell commands with security checks
- **switch_model** - Request model switching with justification
- **request_more_iterations** - Request additional processing iterations
- **long_wait** - Wait for a specified duration with progress updates

#### Webex Integration
- **send_webex_message** - Send messages to Webex rooms
- **delete_webex_message** - Delete Webex messages

### 🌐 Web Server & API

#### HTTP API Endpoints
- `GET /api/sessions` - List all active sessions
- `POST /api/sessions` - Create new session
- `GET /api/sessions/:id` - Get session details
- `DELETE /api/sessions/:id` - Close session

#### WebSocket Support
- **Real-time Streaming** (`/ws/:session_id`) - Bidirectional communication
- **Multi-client Sessions** - Multiple clients per session
- **Tool Confirmation Flow** - Approve/deny tool execution
- **Persistent Storage** - Sessions saved to disk automatically

#### Session Features
- **Editable Titles** - Auto-generated and customizable
- **Chat History** - Full conversation persistence
- **UUID-based IDs** - Unique session identification
- **Session Types** - Web, TUI, and Shared sessions

### 🔒 Security & Policy System

- **Action-based Policies** - Fine-grained control over:
  - File operations (read, write, edit, delete)
  - Command execution
  - Edit planning and application

- **Policy Types**:
  - `Allow` - Auto-approve actions
  - `Deny` - Block actions
  - `Ask` - Require user confirmation

- **Pattern Matching** - Glob patterns for files, string patterns for commands

### 🚀 Operating Modes

1. **REPL Mode** - Interactive command-line conversation (default with `-i`)
2. **Task Mode** - One-shot task execution with `--task` flag (uses subagent orchestration)
3. **Web Server Mode** - Full HTTP/WebSocket API for web interfaces (`--web` flag)

### 📊 Session Persistence & Logging

- **Conversation Logging** - Automatic logging to files
- **Session Metadata** - Model info, tokens, timestamps
- **State Management** - Save/load conversation history (JSON format)
- **Token Tracking** - Usage metrics per session

### 🌍 WebAssembly Frontend

- Browser-based chat interface (WASM)
- WebSocket client integration
- Markdown rendering
- Local storage for session persistence

## Prerequisites

- **Rust** (latest stable version)
- **API Keys** for your chosen provider(s):
  - Groq API key (for Groq models)
  - Anthropic API key (for Claude models)
  - OpenAI API key (for OpenAI models)
  - Or use llama.cpp for local models (no API key needed)

## Installation

1. Clone this repository:
```bash

git clone <repository-url>
cd apchat
```

2. Build the project:
```bash
# For quick syntax/type checks (fastest!)
cargo check

# For development builds (fast builds with debug symbols)
cargo build

# For optimized release builds (slower but faster runtime)
cargo build --release

# Pro tip: Suppress warnings during development to reduce context churn
export RUSTFLAGS="-Awarnings"
cargo check  # or cargo build / cargo build --release

# Pro tip: Use tail to see just the last few lines of output
cargo build --release 2>&1 | tail -n 10
cargo check 2>&1 | tail -n 10
```

### Build Speed Guide
- **`cargo check`**: ~1-5 seconds (syntax/type checking only, no compilation)
- **`cargo build`**: ~10-60 seconds (debug mode, fast iteration)
- **`cargo build --release`**: ~2-10 minutes (optimized, slower but faster runtime)

**Recommendation**: Use `cargo check` or `cargo build` during development, switch to `--release` only when testing final performance or creating distributable binaries.

3. (Optional) Generate shell completions:
```bash
./target/release/apchat --generate bash > apchat-completion.bash
source apchat-completion.bash
```

### Docker Installation

APChat is also available as a Docker container from GitHub Container Registry:

```bash
# Pull the latest image
docker pull ghcr.io/ayourtch-llm/apchat:latest

# Run in interactive mode
docker run -it --rm \
  -e GROQ_API_KEY=$GROQ_API_KEY \
  ghcr.io/ayourtch-llm/apchat:latest -i

# Run with your project directory mounted
docker run -it --rm \
  -v $(pwd):/workspace \
  -e GROQ_API_KEY=$GROQ_API_KEY \
  ghcr.io/ayourtch-llm/apchat:latest -i

# Run with a one-shot task
docker run --rm \
  -e GROQ_API_KEY=$GROQ_API_KEY \
  ghcr.io/ayourtch-llm/apchat:latest \
  --task "Your task here"
```

For comprehensive Docker usage examples, including volume mounting, web server mode, and Docker Compose configurations, see [DOCKER.md](DOCKER.md).

## Configuration

### Environment Variables

Set API keys for your chosen provider(s):

```bash
# For Groq
export GROQ_API_KEY=your_groq_api_key

# For Anthropic Claude (supports multiple model slots)
export ANTHROPIC_AUTH_TOKEN_BLU=your_claude_key_for_blu_model
export ANTHROPIC_AUTH_TOKEN_GRN=your_claude_key_for_grn_model
export ANTHROPIC_AUTH_TOKEN_RED=your_claude_key_for_red_model

# For OpenAI
export OPENAI_API_KEY=your_openai_api_key
```

### Command-Line Options

#### Model Configuration
```bash
# Use custom API URLs for each model slot
--api-url-blu-model <URL>
--api-url-grn-model <URL>
--api-url-red-model <URL>

# Override model names
--model-blu-model <NAME>
--model-grn-model <NAME>
--model-red-model <NAME>

# Quick llama.cpp setup
--llama-cpp-url <URL>
```

#### Mode Selection
```bash
# Run single task and exit (uses subagent orchestration)
--task "Your task here"

# Enable streaming responses
--stream

# Force interactive mode
--interactive
```

#### Behavior
```bash
# Auto-confirm all actions (auto-pilot mode)
--auto-confirm

# Load all skills at conversation start
--early-superpowers

# Learn from user decisions and save to policy file
--learn-policies

# Path to policy file
--policy-file <PATH>
```

#### Web Server
```bash
# Enable web server
--web

# Web server port (default: 8080)
--web-port <PORT>

# Web server bind address (default: 127.0.0.1)
--web-bind <ADDRESS>

# Allow TUI session attachment from web
--web-attachable
```

#### Debug & Output
```bash
# Enable verbose output
--verbose

# Pretty-print JSON output
--pretty
```

## Usage

### REPL Mode (Interactive)

Start an interactive session:

```bash
cargo run -- -i
# or
./target/release/apchat -i
```

The application will:
1. Create a workspace directory if needed
2. Start an interactive chat session
3. Allow natural conversation with AI models
4. Execute tools automatically when needed

**Example interaction:**
```
[GrnModel] You: Create a Rust project structure for a web API

🔧 Calling tool: write_file with args: {"file_path":"Cargo.toml", ...}
📋 Result: Successfully wrote to Cargo.toml

🔧 Calling tool: write_file with args: {"file_path":"src/main.rs", ...}
📋 Result: Successfully wrote to src/main.rs

[GrnModel] Assistant: I've created a basic Rust web API project structure...
```

### Task Mode (One-shot)

Execute a single task:

```bash
apchat --task "Analyze all Rust files and create a summary report"
```

Results are logged to `~/.apchat/sessions/` by default.

### Web Server Mode

Start the web server:

```bash
apchat --web --web-bind 127.0.0.1 --web-port 8080
```

Then interact via HTTP API or WebSocket. Example using curl:

```bash
# Create a new session
curl -X POST http://localhost:8080/api/sessions

# List sessions
curl http://localhost:8080/api/sessions

# Connect via WebSocket (use any WebSocket client)
# ws://localhost:8080/ws/<session-id>
```

### Task Mode (with Subagent Orchestration)

Execute a task with multi-agent coordination:

```bash
apchat --task "Design and implement a complete authentication system"
```

In task mode, specialized agents coordinate to complete the task. In interactive mode, you can also use the `launch_subagent` tool to delegate subtasks to specialized agents.

## Advanced Features

### Custom Model Configuration

Use llama.cpp for local inference:

```bash
apchat --llama-cpp-url http://localhost:8080/v1 \
         --model-blu-model "llama3-8b" \
         --model-grn-model "llama3-70b"
```

### Policy-Based Security

Create a policy file (TOML) to control tool behavior:

```toml
[[policy]]
action = "FileWrite"
pattern = "*.rs"
decision = "Ask"  # Require confirmation for Rust files

[[policy]]
action = "CommandExecution"
pattern = "rm *"
decision = "Deny"  # Block dangerous commands
```

### Skill System

Load proven workflows:

```
[Model] You: Load the test-driven-development skill

[Model] Assistant: Loaded TDD skill. I'll now follow test-first methodology...
```

Find relevant skills:

```
[Model] You: Find skills related to debugging

[Model] Assistant: Found: systematic-debugging, root-cause-tracing...
```

## Documentation

### Tool Documentation

Comprehensive documentation for individual tools is available in the `docs/tools/` directory:

- **[llm_oneshot](docs/tools/llm_oneshot.md)** - One-shot LLM calls for simple tasks without agent overhead
- **[curly_glance_usage](docs/tools/curly_glance_usage.md)** - Drill-down pattern for exploring code structure

For information on how to add new tools to the system, see [docs/dev/how_to_new_tool.md](docs/dev/how_to_new_tool.md).

### Development Guides

- **Code Review Process**: [docs/dev/CODE_REVIEW_CHECKLIST.md](docs/dev/CODE_REVIEW_CHECKLIST.md)
- **Customizing Agents**: [docs/dev/CUSTOMIZING_AGENTS_AND_SKILLS.md](docs/dev/CUSTOMIZING_AGENTS_AND_SKILLS.md)
- **Skill System**: [docs/dev/enhanced-superpowers-prompt.md](docs/dev/enhanced-superpowers-prompt.md)

### Architecture Documentation

- **Terminal Backends**: [docs/architecture/PLAN_PLUGGABLE_TERMINAL_BACKENDS.md](docs/architecture/PLAN_PLUGGABLE_TERMINAL_BACKENDS.md)
- **Refactoring**: [docs/architecture/REFACTORING_MAP.md](docs/architecture/REFACTORING_MAP.md)
- **Session Design**: [docs/architecture/TERMINAL_SESSIONS_DESIGN.md](docs/architecture/TERMINAL_SESSIONS_DESIGN.md)

### Web Frontend

- **Design**: [docs/project/WEB_FRONTEND_DESIGN.md](docs/project/WEB_FRONTEND_DESIGN.md)
- **Protocol Examples**: [docs/web_protocol_examples.md](docs/web_protocol_examples.md)
- **UI Wireframes**: [docs/web_ui_wireframes.md](docs/web_ui_wireframes.md)

---



**Core**:
- Rust 2021 Edition
- Tokio (async runtime)
- Axum (web framework)
- Reqwest (HTTP client)

**LLM Integration**:
- Anthropic Claude API
- Groq API
- OpenAI API
- llama.cpp

**Terminal & UI**:
- portable-pty (PTY sessions)
- vt100 (ANSI parsing)
- rustyline (REPL)
- colored (terminal colors)

**Data & Search**:
- serde/serde_json (serialization)
- regex (pattern matching)
- ignore (gitignore-aware traversal)
- fastembed (semantic search for skills)

### Project Structure

```
apchat/
├── Cargo.toml              # Workspace configuration
├── apchat-main/            # Main binary and CLI
├── crates/
│   ├── apchat-llm-api/     # Unified LLM client interface
│   ├── apchat-logging/     # Conversation logging
│   ├── apchat-models/      # Data structures and types
│   ├── apchat-mspc/        # Message passing channels
│   ├── apchat-policy/      # Security and approval system
│   ├── apchat-progress/    # Progress tracking for streaming
│   ├── apchat-skills/      # Skill registry and loading
│   ├── apchat-terminal/    # PTY session management
│   ├── apchat-todo/        # Task tracking
│   ├── apchat-toolcore/    # Tool execution framework
│   ├── apchat-tools/       # 67+ implemented tools
│   ├── apchat-vty/         # VTY/readline abstractions
│   ├── apchat-wasm/        # WebAssembly frontend
│   ├── apchat-webex/       # Webex bot integration
│   └── apchat-pptx/        # PowerPoint/PPTX library
├── agents/configs/         # Agent JSON configurations (8 agents)
└── skills/                 # Skill definitions (37 SKILL.md files)
```

### Component Overview

```
APChat
├── Chat System (messages, history, state)
├── API Layer (multi-provider LLM integration)
├── Tool System (extensible framework)
├── Web Server (HTTP + WebSocket)
├── Terminal Manager (PTY sessions)
├── Policy Manager (security/approval)
├── Skill Registry (workflow patterns)
├── Memory System (persistent key-value store)
├── Logger (conversation tracking)
├── Task Coordinator (todo management)
└── Subagent System (specialized agents for task delegation)
```

## Contributing

Contributions are welcome! Areas for enhancement:

- Additional LLM providers
- New tools and capabilities
- Enhanced web UI features
- Additional skills and workflows
- Performance optimizations
- Documentation improvements

## Code Metrics

- **79,877 lines** of Rust code (106,345 total lines)
- **134,931 lines** of code across all languages (177,852 total lines)
- **451 Rust files** across the codebase
- **15 modular crates** for clean separation
- **67+ tools** for diverse operations
- **37 curated skills** for proven workflows
- **8 specialized agents** for multi-agent orchestration
- **4 LLM providers** supported
- **15 concurrent** terminal sessions
- **Full test coverage** (in progress)

## License

This project is provided as-is for educational and development purposes.

## Credits

Inspired by Anthropic's Claude Code and built with a focus on extensibility, security, and developer experience.