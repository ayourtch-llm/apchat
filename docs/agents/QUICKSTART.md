# Quick Start Guide

## Useful Shortcuts

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

## Build Commands

```bash
# Build all workspace members
cargo build

# Release build with optimizations
cargo build --release

# Build without embeddings feature (faster builds)
cargo build --no-default-features
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

## Testing Strategy

**Unit Tests**: In `src/` alongside implementation
**Integration Tests**: In `tests/` directories
**E2E Tests**: Shell scripts in project root

Focus areas with test coverage:
- LLM API model configuration (`apchat-llm-api/src/tests/`)
- Tool execution and registry (`apchat-toolcore/tests/`)
- File operations and edit planning (`apchat-tools/tests/`)
- MSPC readline integration (`apchat-main/tests/test_mspc_*.rs`)
- Webex bot integration (`apchat-webex/tests/`)
- Financial services tools (`apchat-finserv/tests/`)
- PowerPoint processing (`apchat-pptx/tests/`)