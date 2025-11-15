# Cargo.toml Templates

This document provides template configurations for each crate in the new workspace structure.

## Workspace Root Cargo.toml

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
fastembed = { version = "5", optional = }

# Unix-specific dependencies
[target.'cfg(unix)'.dependencies]
libc = "0.2"

[workspace.features]
# Default features include embeddings for better skill matching
default = ["embeddings"]

# Semantic embeddings for skill search (using fastembed)
embeddings = ["dep:fastembed"]

# Future: Alternative embedding backend using candle
# candle-embeddings = ["candle-core", "candle-transformers"]

[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true

[profile.dev]
debug = true
```

## kimichat-core/Cargo.toml

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

## kimichat-terminal/Cargo.toml

```toml
[package]
name = "kimichat-terminal"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
description = "Terminal session management for KimiChat with PTY support"

[dependencies]
# Core dependencies
kimichat-core = { path = "../kimichat-core" }

# Terminal/PTY support
portable-pty.workspace = true
vt100.workspace = true

# Async runtime
tokio.workspace = true
tokio-util.workspace = true

# Serialization
serde.workspace = true
serde_json.workspace = true

# Error handling and utilities
anyhow.workspace = true
uuid.workspace = true
chrono.workspace = true

# Unix-specific dependencies
[target.'cfg(unix)'.dependencies]
libc.workspace = true

[dev-dependencies]
tokio-test = "0.4"
tempfile = "3.0"

[features]
default = []
embeddings = ["kimichat-core/embeddings"]

[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"
```

## kimichat-agents/Cargo.toml

```toml
[package]
name = "kimichat-agents"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
description = "Multi-agent system and LLM clients for KimiChat"

[dependencies]
# Core dependencies
kimichat-core = { path = "../kimichat-core" }
kimichat-tools = { path = "../kimichat-tools" }

# HTTP and streaming
reqwest.workspace = true
futures-util.workspace = true
futures.workspace = true
async-stream.workspace = true

# Async runtime
tokio.workspace = true

# Serialization
serde.workspace = true
serde_json.workspace = true

# Error handling and utilities
anyhow.workspace = true
uuid.workspace = true
chrono.workspace = true

[dev-dependencies]
tokio-test = "0.4"
mockito = "1.0"

[features]
default = []
embeddings = ["kimichat-core/embeddings", "kimichat-tools/embeddings"]

[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"
```

## kimichat-web/Cargo.toml

```toml
[package]
name = "kimichat-web"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
description = "Web interface and WebSocket server for KimiChat"

[dependencies]
# Core dependencies
kimichat-core = { path = "../kimichat-core" }
kimichat-terminal = { path = "../kimichat-terminal" }
kimichat-agents = { path = "../kimichat-agents" }

# Web server
axum.workspace = true
tower.workspace = true
tower-http.workspace = true

# Async runtime and streaming
tokio.workspace = true
futures-util.workspace = true

# Serialization
serde.workspace = true
serde_json.workspace = true

# Error handling and utilities
anyhow.workspace = true
uuid.workspace = true

[dev-dependencies]
tokio-test = "0.4"
axum-test = "15.0"

[features]
default = []
embeddings = ["kimichat-core/embeddings", "kimichat-terminal/embeddings", "kimichat-agents/embeddings"]

[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"
```

## kimichat-tools/Cargo.toml

```toml
[package]
name = "kimichat-tools"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
description = "Tool implementations for KimiChat including file operations and system tools"

[dependencies]
# Core dependencies
kimichat-core = { path = "../kimichat-core" }
kimichat-terminal = { path = "../kimichat-terminal" }

# File operations and search
glob.workspace = true
ignore.workspace = true
regex.workspace = true
similar.workspace = true

# Async runtime
tokio.workspace = true

# Serialization
serde.workspace = true
serde_json.workspace = true

# Error handling and utilities
anyhow.workspace = true

# Configuration
dotenvy.workspace = true

[dev-dependencies]
tokio-test = "0.4"
tempfile = "3.0"

[features]
default = []
embeddings = ["kimichat-core/embeddings", "kimichat-terminal/embeddings"]

# Optional embedding support
fastembed = { workspace = true, optional = true }

[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"
```

## kimichat-cli/Cargo.toml

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
# Workspace crates
kimichat-core = { path = "../kimichat-core" }
kimichat-terminal = { path = "../kimichat-terminal" }
kimichat-agents = { path = "../kimichat-agents" }
kimichat-web = { path = "../kimichat-web" }
kimichat-tools = { path = "../kimichat-tools" }

# CLI interface
clap.workspace = true
clap_complete.workspace = true
colored.workspace = true
rustyline.workspace = true

# Async runtime
tokio.workspace = true

# Error handling
anyhow.workspace = true

# Configuration
dotenvy.workspace = true

[dev-dependencies]
tokio-test = "0.4"
tempfile = "3.0"
assert_cmd = "2.0"

[features]
default = ["embeddings"]
embeddings = [
    "kimichat-core/embeddings",
    "kimichat-terminal/embeddings", 
    "kimichat-agents/embeddings",
    "kimichat-web/embeddings",
    "kimichat-tools/embeddings",
]

# Build binaries for common targets
[metadata.deb]
maintainer = "KimiChat Team <team@kimichat.dev>"
copyright = "2024, KimiChat Team"
license-file = ["LICENSE", "4"]
extended-description = """\
A Rust-based CLI application that provides Claude Code-like experience 
with AI-powered chat and tool execution capabilities."""
depends = "$auto"
section = "utility"
priority = "optional"
assets = [
    ["target/release/kimichat", "usr/bin/", "755"],
    ["README.md", "usr/share/doc/kimichat/README", "644"],
]

[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"
```

## Key Features of These Templates

### 1. Workspace Configuration
- Centralized dependency management
- Shared version and metadata
- Consistent feature flags
- Common build profiles

### 2. Dependency Management
- Clear separation between internal and external dependencies
- Workspace dependencies with inheritance
- Minimal dependencies per crate
- Optional features for embeddings

### 3. Development Features
- Comprehensive dev-dependencies
- Consistent linting configuration
- Documentation requirements
- Test utilities

### 4. Build Optimization
- Release profile optimizations
- Feature flag management
- Conditional compilation
- Binary configuration for CLI

### 5. Distribution
- Debian package configuration
- Binary asset management
- Metadata for package repositories

### 6. Code Quality
- Linting rules for all crates
- Documentation requirements
- Safety rules (no unsafe code)
- Consistent clippy configuration

These templates provide a solid foundation for the workspace migration while maintaining code quality, performance, and maintainability standards.