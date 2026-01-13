# APChat - Rust CLI Application

## Important Notes - MUST FOLLOW!

**Conversation History**: Check existing conversation history before deciding whether to perform operations - avoid redundant calls
**File Operations**: Use specific patterns like `"src/*.rs"` instead of `"*.rs"` to locate files in the src directory
**Repeat operations**: If your history already has a file read, do not read it again - as this will overload the history. Likewise, if you are doing an edit - do not attempt to do it multiple times, if something fails, ask the user to verify.

## Useful shortcuts

### Build the project
```bash
cargo build --release
```

### Run the application
```bash
cargo run --release
```

### Run inside the pty tool, with a real model behind, to be able to interact:
```bash
cargo run --release -- --stream --interactive --llama-cpp-url http://192.168.0.201:8081
```

## Project Overview

have a look at CLAUDE.md for more info
