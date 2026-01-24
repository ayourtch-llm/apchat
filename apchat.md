# APChat - Rust CLI Application

## Important Notes - MUST FOLLOW!

**Conversation History**: Check existing conversation history before deciding whether to perform operations - avoid redundant calls
**File Operations**: Use specific patterns like `"src/*.rs"` instead of `"*.rs"` to locate files in the src directory
**Repeat operations**: If your history already has a file read, do not read it again - as this will overload the history. Likewise, if you are doing an edit - do not attempt to do it multiple times, if something fails, ask the user to verify.

**Tools Documentation**: For best practices on using tools efficiently (especially the `open_file` tool), see [docs/tools/README.md](docs/tools/README.md) for an overview, or [docs/tools/QUICK_REFERENCE.md](docs/tools/QUICK_REFERENCE.md) for immediate guidance.

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

have a look at [docs/project/CLAUDE.md](docs/project/CLAUDE.md) for more info

### Project exploration
IMPORTANT: you can save a lot of effort by using file_curly_glance tool to look at the *.rs files! It will give you the parentheses spans - and if you
give the start line inside the span - then you can recursively peek inside the spans too! Read [docs/tools/curly_glance_usage.md](docs/tools/curly_glance_usage.md) for more info.


# Tool Usage Best Practices

## open_file Tool

### IMPORTANT: Use 20-line chunks for large files!

**Do NOT read entire large files at once.** The `open_file` tool has a default behavior of reading the entire file, which can cause:
1. **Output truncation** - Very large files may be cut off
2. **Tool call overhead** - Reading full files costs extra iterations
3. **Context bloat** - Unnecessary content competes with conversation history limits

### Recommended Usage Pattern

```rust
// ❌ BAD - Reads entire file (potential issues above)
open_file {
  "file_path": "src/main.rs"
}

// ✅ GOOD - Read in 20-line chunks
open_file {
  "file_path": "src/main.rs",
  "max_line_count": 20
}

// ✅ GOOD - Read specific range
open_file {
  "file_path": "src/main.rs",
  "start_line": 101,
  "max_line_count": 30
}
```

### When to Use Full File Read

Only read entire files if:
- The file is **small** (< 50 lines)
- You need to see the **complete context** (e.g., small config files)
- You're doing a **quick scan** of structure

### Pro Tip: Combine with curly_glance

For large code files, use `file_curly_glance` to find sections of interest, then use `open_file` with line ranges to read specific parts:

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
open_file {
  "file_path": "src/main.rs",
  "start_line": 42,
  "max_line_count": 25
}
```

This iterative approach is **far more efficient** than reading entire files!
