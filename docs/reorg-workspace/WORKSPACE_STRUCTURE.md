# Workspace Structure

This document provides a detailed breakdown of the proposed workspace structure for KimiChat, including the responsibilities of each crate and their public APIs.

## Crate Overview

```
kimichat-workspace/
├── Cargo.toml                    # Workspace configuration
├── kimichat-core/               # Foundation layer
│   ├── Cargo.toml
│   └── src/
├── kimichat-terminal/           # Terminal operations
│   ├── Cargo.toml
│   └── src/
├── kimichat-agents/             # Multi-agent system
│   ├── Cargo.toml
│   └── src/
├── kimichat-web/                # Web interface
│   ├── Cargo.toml
│   └── src/
├── kimichat-tools/              # Tool implementations
│   ├── Cargo.toml
│   └── src/
└── kimichat-cli/                # Main application
    ├── Cargo.toml
    └── src/
```

## 1. kimichat-core - Foundation Layer

### Purpose
Central hub providing core types, tool system, and configuration management that other crates depend on.

### Responsibilities
- Tool registry and execution framework
- Policy management for access control
- Configuration management and validation
- Core error types and result handling
- Tool call parsing and validation
- Common utilities and traits

### Key Components
- `ToolRegistry` and `ToolParameters`
- `ToolContext` for execution environment
- `PolicyManager` for access control
- Configuration management and validation
- Tool validation and parsing
- Core error types and utilities

### Dependencies (Minimal)
```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
thiserror = "1.0"
async-trait = "0.1"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
tokio = { version = "1.41", features = ["sync"] }
```

### Public API
```rust
// Core types and traits
pub use tool_registry::{ToolRegistry, ToolParameters};
pub use tool_context::ToolContext;
pub use tool::{Tool, ToolResult, ToolError};

// Configuration and policy
pub use config::{ClientConfig, ConfigError, ConfigLoader};
pub use policy::{PolicyManager, Policy, PolicyError};

// Validation and parsing
pub use validation::{validate_tool_calls, repair_tool_call, ValidationError};

// Error types
pub use errors::{CoreError, Result};

// Utilities
pub use utils::{JsonSchema, json_schema};
```

## 2. kimichat-terminal - Terminal Operations

### Purpose
Complete terminal session management with PTY support and multiple backend implementations.

### Responsibilities
- PTY session management and lifecycle
- Terminal backend implementations (PTY, Tmux)
- Screen buffer and VT100/ANSI interpretation
- Session logging and capture features
- Cursor management and screen operations
- Terminal session persistence

### Key Components
- `TerminalManager` for session lifecycle management
- `TerminalSession` with full VT100/ANSI support
- Backend implementations (PtyBackend, TmuxBackend)
- Screen buffer and cursor management
- Session logger with capture functionality
- Terminal tools integration

### Dependencies
```toml
[dependencies]
kimichat-core = { path = "../kimichat-core" }
portable-pty = "0.8"
vt100 = "0.15"
tokio = { version = "1.41", features = ["full"] }
tokio-util = "0.7"
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

### Public API
```rust
// Session management
pub use manager::TerminalManager;
pub use session::{TerminalSession, SessionId, SessionStatus, SessionMetadata};

// Backend implementations
pub use backend::{TerminalBackend, TerminalBackendType, SessionInfo, CursorPosition};
pub use pty_backend::PtyBackend;
pub use tmux_backend::TmuxBackend;

// Screen and buffer management
pub use screen_buffer::{ScreenBuffer, ScreenCell, CursorPosition};

// Logging and capture
pub use logger::{SessionLogger, CaptureMode};

// Terminal tools
pub use tools::{
    PtyLaunchTool, PtySendKeysTool, PtyGetScreenTool,
    PtyListTool, PtyKillTool, PtyGetCursorTool,
    PtyResizeTool, PtySetScrollbackTool,
    PtyStartCaptureTool, PtyStopCaptureTool,
    PtyRequestUserInputTool,
};
```

## 3. kimichat-agents - Multi-Agent System

### Purpose
Agent framework with LLM client implementations and coordination logic.

### Responsibilities
- Agent framework and coordination
- LLM client implementations (Groq, Anthropic, llama.cpp)
- Task execution and progress evaluation
- Model switching logic and visibility management
- Agent factory and embedded configurations
- Multi-agent conversation management

### Key Components
- `PlanningCoordinator` for agent orchestration
- LLM clients (GroqClient, AnthropicClient, LlamaCppClient)
- Task execution and progress evaluation
- Model switching logic and visibility management
- Agent factory and embedded configurations
- Agent message handling and routing

### Dependencies
```toml
[dependencies]
kimichat-core = { path = "../kimichat-core" }
kimichat-tools = { path = "../kimichat-tools" }
reqwest = { version = "0.12", features = ["json", "stream"] }
futures-util = "0.3"
futures = "0.3"
async-stream = "0.3"
tokio = { version = "1.41", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

### Public API
```rust
// Agent coordination and management
pub use coordinator::PlanningCoordinator;
pub use agent::{Agent, AgentConfig, AgentType, AgentStatus};
pub use agent_factory::AgentFactory;

// LLM clients
pub use clients::{GroqClient, AnthropicClient, LlamaCppClient, LlmClient};
pub use embedded_configs::EmbeddedConfigs;

// Task execution
pub use task::{Task, TaskExecutor, TaskStatus, TaskResult};
pub use progress_evaluator::ProgressEvaluator;

// Model management
pub use visibility::{VisibilityManager, ModelVisibility};
pub use model_switching::{ModelSwitcher, SwitchModelArgs};

// Messaging
pub use message::{ChatMessage, MessageRole, MessageContent};
```

## 4. kimichat-web - Web Interface

### Purpose
Real-time web interface with WebSocket communication and session management.

### Responsibilities
- WebSocket server implementation
- Real-time bidirectional communication
- Session management for web clients
- Protocol message handling and routing
- Web-specific authentication and authorization
- Integration with terminal and agent systems

### Key Components
- WebSocket server using Axum
- Session management for web clients
- Protocol message handling and validation
- Real-time bidirectional communication
- Web-specific authentication middleware

### Dependencies
```toml
[dependencies]
kimichat-core = { path = "../kimichat-core" }
kimichat-terminal = { path = "../kimichat-terminal" }
kimichat-agents = { path = "../kimichat-agents" }
axum = { version = "0.7", features = ["ws", "macros"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "cors", "trace"] }
tokio = { version = "1.41", features = ["full"] }
futures-util = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
anyhow = "1.0"
```

### Public API
```rust
// Server and session management
pub use server::WebServer;
pub use session_manager::{SessionManager, SessionId, SessionType, WebSession};

// Protocol and communication
pub use protocol::{ClientMessage, ServerMessage, MessageHandler, ProtocolError};
pub use routes::{WebRoutes, RouteHandler};

// WebSocket management
pub use websocket::{WebSocketManager, ConnectionManager};

// Authentication and middleware
pub use auth::{AuthMiddleware, WebAuth};
```

## 5. kimichat-tools - Tool Implementations

### Purpose
All tool implementations that extend the core tool system with specific functionality.

### Responsibilities
- File operations (read, write, edit, search)
- System tools (iteration control, model management)
- Project tools and skill discovery
- Todo management and workspace tools
- Integration points for external tools

### Key Components
- File operation tools (read, write, edit, search, list)
- System tools (model management, iteration control)
- Project tools (workspace analysis, dependency management)
- Skill tools (skill discovery, loading, execution)
- Todo management tools

### Dependencies
```toml
[dependencies]
kimichat-core = { path = "../kimichat-core" }
kimichat-terminal = { path = "../kimichat-terminal" }
glob = "0.3"
ignore = "0.4"
regex = "1.0"
similar = { version = "2.6", features = ["inline"] }
tokio = { version = "1.41", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
```

### Public API
```rust
// File operations
pub use file_ops::{
    ReadFileTool, WriteFileTool, EditFileTool, ListFilesTool,
    SearchFilesTool, FindRelevantSkillsTool, ApplyEditPlanTool
};

// System tools
pub use system::{
    ModelManagementTool, IterationControlTool, SystemInfoTool
};

// Project tools
pub use project_tools::{
    ProjectAnalyzerTool, DependencyTool, WorkspaceTool
};

// Skill tools
pub use skill_tools::{
    LoadSkillTool, ListSkillsTool, ExecuteSkillTool
};

// Todo management
pub use todo_tools::{
    TodoListTool, TodoWriteTool, TodoStatusTool
};
```

## 6. kimichat-cli - Main Application

### Purpose
CLI interface, REPL, and application coordination that ties all other crates together.

### Responsibilities
- Command-line interface using Clap
- REPL implementation with rustyline
- Application setup and initialization
- Chat history and state management
- Main application orchestrator
- Integration of all other crates

### Key Components
- CLI interface and command parsing
- REPL with rustyline integration
- Application setup and initialization
- Chat history and state management
- Main application orchestrator
- Configuration loading and validation

### Dependencies
```toml
[dependencies]
kimichat-core = { path = "../kimichat-core" }
kimichat-terminal = { path = "../kimichat-terminal" }
kimichat-agents = { path = "../kimichat-agents" }
kimichat-web = { path = "../kimichat-web" }
kimichat-tools = { path = "../kimichat-tools" }

clap = { version = "4.5", features = ["derive", "env"] }
clap_complete = "4.5"
rustyline = "14.0"
colored = "2.1"
tokio = { version = "1.41", features = ["full"] }
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Public API
```rust
// CLI and commands
pub use cli::{Cli, Commands, TerminalCommands, WebCommands};
pub use repl::Repl;

// Application modes
pub use app::{
    run_repl_mode, run_task_mode, run_web_mode, setup_from_cli,
    Application, ApplicationConfig
};

// State and history
pub use state::{ApplicationState, StateManager};
pub use history::{ChatHistory, HistoryManager};
```

## Inter-Crate Dependencies

```
kimichat-cli
    ├── kimichat-web
    ├── kimichat-agents
    ├── kimichat-tools
    ├── kimichat-terminal
    └── kimichat-core

kimichat-web
    ├── kimichat-agents
    ├── kimichat-terminal
    └── kimichat-core

kimichat-agents
    ├── kimichat-tools
    └── kimichat-core

kimichat-tools
    ├── kimichat-terminal
    └── kimichat-core

kimichat-terminal
    └── kimichat-core

kimichat-core (no internal dependencies)
```

## Benefits of This Structure

1. **Clear Boundaries:** Each crate has well-defined responsibilities
2. **Minimal Dependencies:** Core crate has minimal external dependencies
3. **Reusability:** Components can be used independently in other projects
4. **Testing:** Each crate can be tested in isolation
5. **Build Performance:** Parallel compilation and better caching
6. **Publishing:** Individual crates can be published as needed