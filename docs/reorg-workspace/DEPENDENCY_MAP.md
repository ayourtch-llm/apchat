# Inter-Crate Dependency Map

This document outlines the dependency relationships between crates in the new workspace structure and provides guidance for managing these dependencies effectively.

## Dependency Graph Overview

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

kimichat-core
    (no internal dependencies)
```

## Detailed Dependency Analysis

### 1. kimichat-core (Foundation Layer)

**Purpose:** Provides core types, tool system, and configuration management

**Internal Dependencies:** None
**External Dependencies:** Minimal (serde, anyhow, tokio-sync)

**Provides to other crates:**
- `ToolRegistry` and tool system infrastructure
- `ToolContext` for execution environment
- `PolicyManager` for access control
- Configuration types and management
- Core error types and result handling
- Tool validation and parsing utilities

**Key Public Types:**
```rust
// Core types that other crates depend on
pub struct ToolRegistry;
pub struct ToolContext;
pub struct PolicyManager;
pub struct ClientConfig;
pub enum CoreError;

// Traits that other crates implement
pub trait Tool;
pub trait ToolExecutor;
pub trait ConfigLoader;
```

### 2. kimichat-terminal (Terminal Operations)

**Purpose:** Terminal session management with PTY support

**Internal Dependencies:**
- `kimichat-core` - For tool system integration and core types

**External Dependencies:**
- `portable-pty` - PTY implementation
- `vt100` - Terminal emulation
- `tokio` - Async runtime

**Provides to other crates:**
- `TerminalManager` for session lifecycle
- `TerminalSession` with VT100 support
- Terminal backend implementations
- Screen buffer and cursor management
- Session logging and capture

**Key Public Types:**
```rust
pub struct TerminalManager;
pub struct TerminalSession;
pub trait TerminalBackend;
pub struct ScreenBuffer;
pub enum TerminalBackendType;
```

### 3. kimichat-tools (Tool Implementations)

**Purpose:** Concrete implementations of tools for various operations

**Internal Dependencies:**
- `kimichat-core` - Tool system integration and core types
- `kimichat-terminal` - Terminal-related tools

**External Dependencies:**
- `glob`, `ignore` - File system operations
- `regex` - Pattern matching
- `similar` - File diffing

**Provides to other crates:**
- File operation tools (read, write, edit, search)
- System tools (model management, iteration control)
- Project tools and skill discovery
- Todo management tools

**Key Public Types:**
```rust
// Tool implementations
pub struct ReadFileTool;
pub struct WriteFileTool;
pub struct SearchFilesTool;
pub struct PtyLaunchTool;
pub struct ModelManagementTool;

// Tool registration helper
pub fn register_all_tools(registry: &mut ToolRegistry);
```

### 4. kimichat-agents (Multi-Agent System)

**Purpose:** Agent framework and LLM client implementations

**Internal Dependencies:**
- `kimichat-core` - Core types and configuration
- `kimichat-tools` - Tool execution capabilities

**External Dependencies:**
- `reqwest` - HTTP client for LLM APIs
- `futures` - Async utilities
- `async-stream` - Streaming support

**Provides to other crates:**
- Agent coordination and management
- LLM client implementations
- Task execution framework
- Model switching logic

**Key Public Types:**
```rust
pub struct PlanningCoordinator;
pub trait LlmClient;
pub struct Agent;
pub struct TaskExecutor;
pub enum ModelType;
```

### 5. kimichat-web (Web Interface)

**Purpose:** Web server and real-time communication

**Internal Dependencies:**
- `kimichat-core` - Core types and configuration
- `kimichat-terminal` - Terminal session integration
- `kimichat-agents` - Agent coordination

**External Dependencies:**
- `axum` - Web framework
- `tower` - HTTP middleware
- `tokio` - Async runtime

**Provides to other crates:**
- WebSocket server implementation
- Session management for web clients
- Protocol message handling
- Real-time communication

**Key Public Types:**
```rust
pub struct WebServer;
pub struct SessionManager;
pub struct WebSocketManager;
pub enum ClientMessage;
pub enum ServerMessage;
```

### 6. kimichat-cli (Main Application)

**Purpose:** CLI interface and application coordination

**Internal Dependencies:**
- `kimichat-core` - Core functionality
- `kimichat-terminal` - Terminal operations
- `kimichat-agents` - Agent system
- `kimichat-web` - Web interface
- `kimichat-tools` - Tool implementations

**External Dependencies:**
- `clap` - CLI argument parsing
- `rustyline` - REPL implementation
- `colored` - Terminal colors

**Provides to other crates:**
- None (leaf node in dependency graph)

**Key Public Types:**
```rust
// Main application struct
pub struct Application;

// CLI configuration
pub struct Cli;

// REPL implementation
pub struct Repl;
```

## Dependency Flow Patterns

### 1. Core Abstraction Layer
```
External Libraries → kimichat-core → Other Crates
```
The core crate provides abstractions that other crates implement and use.

### 2. Feature-Specific Layers
```
External Libraries + kimichat-core → Feature Crate → CLI/Web
```
Feature-specific crates (terminal, agents, web, tools) build on core and are consumed by application crates.

### 3. Application Coordination
```
All Feature Crates → kimichat-cli
```
The CLI crate orchestrates all functionality through well-defined APIs.

## Managing Dependencies

### 1. Version Management
```toml
[workspace.dependencies]
# Centralized version management
tokio = { version = "1.41", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
# ... other dependencies
```

### 2. Feature Flags
```toml
[workspace.features]
default = ["embeddings"]
embeddings = ["dep:fastembed"]

# Crate-specific features
kimichat-terminal = ["kimichat-core/embeddings"]
kimichat-agents = ["kimichat-core/embeddings", "kimichat-tools/embeddings"]
```

### 3. Dependency Validation
```toml
# Prevent circular dependencies
[workspace.lints.rust]
# Enable dependency analysis warnings
```

## Best Practices

### 1. Dependency Direction
- **Always**: Lower-level crates → Higher-level crates
- **Never**: Higher-level crates → Lower-level crates
- **Exception**: Shared traits/interfaces in kimichat-core

### 2. API Design
- Keep public APIs minimal and stable
- Use traits for extensible interfaces
- Avoid exposing internal implementation details
- Document all public APIs thoroughly

### 3. Version Compatibility
- Use semantic versioning for public APIs
- Maintain backward compatibility when possible
- Document breaking changes clearly
- Use feature flags for optional functionality

### 4. Testing Dependencies
- Test each crate independently
- Use mock implementations for external dependencies
- Test integration points between crates
- Validate dependency contracts

## Potential Issues and Solutions

### 1. Circular Dependencies
**Problem:** Crate A depends on Crate B, which depends on Crate A

**Solution:**
- Move shared abstractions to kimichat-core
- Use trait objects to break cycles
- Reorganize responsibilities

### 2. Dependency Bloat
**Problem:** Too many transitive dependencies

**Solution:**
- Use feature flags for optional dependencies
- Prefer minimal external dependencies
- Regular dependency audits

### 3. Version Conflicts
**Problem:** Different crates require different versions of the same dependency

**Solution:**
- Use workspace dependency management
- Coordinate version updates across crates
- Use compatible version ranges

### 4. API Breakage
**Problem:** Changes in lower-level crates break higher-level crates

**Solution:**
- Maintain API stability guarantees
- Use semantic versioning
- Provide migration guides for breaking changes

## Migration Validation

### 1. Dependency Graph Analysis
```bash
# Check for circular dependencies
cargo tree --duplicates --format "{p}"

# Visualize dependency graph
cargo tree --format "{p}" | dot -Tpng > deps.png
```

### 2. Compilation Order
```bash
# Test incremental compilation
cargo build -p kimichat-core
cargo build -p kimichat-terminal
cargo build -p kimichat-tools
cargo build -p kimichat-agents
cargo build -p kimichat-web
cargo build -p kimichat-cli
```

### 3. Feature Flag Testing
```bash
# Test with different feature combinations
cargo build --workspace --no-default-features
cargo build --workspace --features "embeddings"
cargo build --workspace --all-features
```

This dependency structure provides a solid foundation for the workspace while maintaining clean separation of concerns and minimizing coupling between components.