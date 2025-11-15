# Workspace Reorganization Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Reorganize KimiChat from single-crate to multi-crate workspace structure

**Architecture:** Functional separation into 6 crates: core, terminal, agents, web, tools, cli

**Tech Stack:** Rust, Cargo workspace, Tokio, existing dependencies

---

## Phase 1: Create Workspace Structure and Core Crate

### Task 1: Backup Current State

**Files:**
- Modify: Current repository state

**Step 1: Create initial commit point**

```bash
# Ensure current state is committed
git add .
git status
git commit -m "feat: save pre-reorganization state"
```

**Step 2: Verify current functionality**

```bash
# Test current application builds and runs
cargo build --release
cargo test

# Expected: All tests pass, application builds successfully
```

**Step 3: Commit**

```bash
git add .
git commit -m "backup: save working state before workspace reorganization"
```

### Task 2: Create Workspace Root Configuration

**Files:**
- Create: `Cargo.toml` (new workspace root)
- Move: `Cargo.toml` → `kimichat-cli/Cargo.toml`

**Step 1: Read current Cargo.toml**

```bash
# Examine current configuration
cat Cargo.toml
```

**Step 2: Create new workspace root Cargo.toml**

```toml
[workspace]
members = [
    "kimichat-core",
    "kimichat-terminal", 
    "kimichat-agents",
    "kimichat-web",
    "kimichat-tools",
    "kimichat-cli",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["KimiChat Team"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/your-org/kimichat"
homepage = "https://github.com/your-org/kimichat"
description = "A Rust-based CLI application for AI-powered chat and tool execution"

[workspace.dependencies]
# Async runtime
tokio = { version = "1.41", features = ["full"] }
tokio-util = "0.7"

# HTTP and streaming
reqwest = { version = "0.12", features = ["json", "stream"] }
futures-util = "0.3"
futures = "0.3"
async-stream = "0.3"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# CLI
clap = { version = "4.5", features = ["derive", "env"] }
clap_complete = "4.5"
colored = "2.1"
rustyline = "14.0"

# File operations and search
glob = "0.3"
ignore = "0.4"
regex = "1.0"
similar = { version = "2.6", features = ["inline"] }

# Utilities
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
dotenvy = "0.15"
async-trait = "0.1"

# Web server
axum = { version = "0.7", features = ["ws", "macros"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "cors", "trace"] }

# Terminal/PTY support
portable-pty = "0.8"
vt100 = "0.15"

# Optional embedding support
fastembed = { version = "5", optional = true }

# Unix-specific dependencies
[target.'cfg(unix)'.dependencies]
libc = "0.2"

[workspace.features]
# Default features include embeddings for better skill matching
default = ["embeddings"]

# Semantic embeddings for skill search (using fastembed)
embeddings = ["dep:fastembed"]

[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true

[profile.dev]
debug = true
```

**Step 3: Move existing Cargo.toml to CLI crate**

```bash
# Create CLI directory and move Cargo.toml
mkdir -p kimichat-cli
mv Cargo.toml kimichat-cli/
```

**Step 4: Test workspace configuration**

```bash
# Verify workspace recognizes members
cargo check --workspace
# Expected: Error about missing crates, but workspace structure recognized
```

**Step 5: Commit**

```bash
git add Cargo.toml kimichat-cli/Cargo.toml
git commit -m "feat: create workspace structure and move CLI crate config"
```

### Task 3: Create kimichat-core Crate Structure

**Files:**
- Create: `kimichat-core/Cargo.toml`
- Create: `kimichat-core/src/lib.rs`
- Create: `kimichat-core/src/` directories

**Step 1: Create core crate Cargo.toml**

```toml
[package]
name = "kimichat-core"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
description = "Core functionality for KimiChat including tool system and configuration"

[dependencies]
# Serialization
serde.workspace = true
serde_json.workspace = true
toml.workspace = true

# Error handling
anyhow.workspace = true
thiserror.workspace = true

# Utilities
uuid.workspace = true
chrono.workspace = true
async-trait.workspace = true

# Async runtime (minimal features)
tokio = { workspace = true, features = ["sync", "rt"] }

[dev-dependencies]
tokio-test = "0.4"

[features]
default = []
embeddings = []

[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"
```

**Step 2: Create core directory structure**

```bash
mkdir -p kimichat-core/src/{core,config,policy,models,validation,errors,utils}
```

**Step 3: Create core lib.rs**

```rust
//! KimiChat Core Library
//! 
//! Provides core functionality including tool system, configuration management,
//! policy enforcement, and shared types used across the KimiChat workspace.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod core;
pub mod config;
pub mod policy;
pub mod models;
pub mod validation;
pub mod errors;
pub mod utils;

// Re-export commonly used types
pub use core::{ToolRegistry, ToolParameters, ToolContext, Tool};
pub use config::{ClientConfig, ConfigError, ConfigLoader};
pub use policy::{PolicyManager, Policy, PolicyError};
pub use models::{Message, ToolCall, FunctionCall, ModelType};
pub use validation::{validate_tool_calls, repair_tool_call, ValidationError};
pub use errors::{CoreError, Result};
pub use utils::{JsonSchema, json_schema};
```

**Step 4: Test core crate compilation**

```bash
cargo check -p kimichat-core
# Expected: FAIL - missing modules, but structure recognized
```

**Step 5: Commit**

```bash
git add kimichat-core/
git commit -m "feat: create kimichat-core crate structure"
```

### Task 4: Extract Core Modules

**Files:**
- Move: `src/core/` → `kimichat-core/src/core/`
- Move: `src/policy.rs` → `kimichat-core/src/policy.rs`
- Move: `src/config/` → `kimichat-core/src/config/`
- Move: `src/models/` → `kimichat-core/src/models/`
- Move: `src/tools_execution/validation.rs` → `kimichat-core/src/validation.rs`

**Step 1: Move core modules**

```bash
# Move core directory
cp -r src/core/* kimichat-core/src/core/

# Move other core files
cp src/policy.rs kimichat-core/src/
cp -r src/config/* kimichat-core/src/config/
cp -r src/models/* kimichat-core/src/models/
cp src/tools_execution/validation.rs kimichat-core/src/validation.rs

# Create placeholder modules for missing pieces
touch kimichat-core/src/errors.rs kimichat-core/src/utils.rs
```

**Step 2: Create error types**

```rust
// kimichat-core/src/errors.rs
//! Core error types for KimiChat

use thiserror::Error;

/// Core error type
#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Tool execution failed: {0}")]
    ToolExecution(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Policy violation: {0}")]
    Policy(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Core result type
pub type Result<T> = std::result::Result<T, CoreError>;
```

**Step 3: Create utility types**

```rust
// kimichat-core/src/utils.rs
//! Utility types and functions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// JSON Schema type alias
pub type JsonSchema = serde_json::Value;

/// Create a JSON schema
pub fn json_schema() -> JsonSchema {
    serde_json::json!({})
}

/// JSON serializable trait
pub trait JsonSerializable {
    fn to_json(&self) -> Result<JsonSchema, serde_json::Error>;
    fn from_json(json: &JsonSchema) -> Result<Self, serde_json::Error>
    where
        Self: Sized;
}

/// Configuration map type
pub type ConfigMap = HashMap<String, JsonSchema>;
```

**Step 4: Fix module imports in moved files**

```bash
# This will require updating import statements in moved files
# We'll need to edit each file to fix imports
```

**Step 5: Test core compilation**

```bash
cargo check -p kimichat-core
# Expected: May have import errors, but basic structure should compile
```

**Step 6: Commit**

```bash
git add kimichat-core/src/
git commit -m "feat: extract core modules to kimichat-core crate"
```

### Task 5: Update CLI Crate for Workspace

**Files:**
- Modify: `kimichat-cli/Cargo.toml`
- Modify: `kimichat-cli/src/main.rs`

**Step 1: Update CLI crate dependencies**

```toml
[package]
name = "kimichat"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
description = "A Rust-based CLI application providing Claude Code-like experience with AI-powered chat and tool execution"
categories = ["command-line-utilities", "development-tools"]
keywords = ["cli", "ai", "chat", "tools", "assistant"]

[[bin]]
name = "kimichat"
path = "src/main.rs"

[dependencies]
# Core dependency
kimichat-core = { path = "../kimichat-core" }

# CLI interface
clap.workspace = true
colored.workspace = true
rustyline.workspace = true

# Temporary - will be moved to other crates later
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true
async-trait.workspace = true

# Other dependencies - will be refactored later
reqwest.workspace = true
futures-util.workspace = true
axum.workspace = true
portable-pty.workspace = true
# ... (keep existing dependencies for now)
```

**Step 2: Update main.rs imports**

```rust
// At the top of main.rs, add kimichat-core dependency
use kimichat_core::{CoreError, Result};

// Keep existing imports for now, will be updated incrementally
```

**Step 3: Test workspace compilation**

```bash
cargo check --workspace
# Expected: May have errors but workspace structure should work
cargo build -p kimichat-core
# Expected: Core crate should compile
```

**Step 4: Commit**

```bash
git add kimichat-cli/Cargo.toml kimichat-cli/src/main.rs
git commit -m "feat: update CLI crate for workspace structure"
```

### Task 6: Validate Phase 1 Completion

**Files:**
- Test: Entire workspace

**Step 1: Test workspace compilation**

```bash
cargo check --workspace
cargo build --workspace
# Expected: Core crate compiles, CLI crate may have errors but structure is sound
```

**Step 2: Test core crate independently**

```bash
cargo test -p kimichat-core
cargo build -p kimichat-core --release
# Expected: Core crate builds and tests pass
```

**Step 3: Validate workspace structure**

```bash
cargo tree --format "{p}" | grep kimichat
# Expected: Shows proper dependency tree with kimichat-core
```

**Step 4: Final Phase 1 commit**

```bash
git add .
git commit -m "feat: complete Phase 1 - workspace structure and core crate

- Created workspace configuration with 6 member crates
- Extracted core functionality to kimichat-core crate
- Updated CLI crate to use workspace structure
- Established dependency management patterns
- Core crate compiles independently"
```

## Next Phases (Overview)

After Phase 1 is complete and validated:
- **Phase 2:** Extract terminal functionality to kimichat-terminal
- **Phase 3:** Extract agent system to kimichat-agents  
- **Phase 4:** Extract web components to kimichat-web
- **Phase 5:** Extract tools to kimichat-tools
- **Phase 6:** Clean up CLI crate
- **Phase 7:** Comprehensive testing and validation

Each phase follows the same pattern: create crate structure, move modules, update dependencies, test compilation, commit changes.