# APChat - Rust CLI Application

## Important Notes - MUST FOLLOW!

**Conversation History**: Check existing conversation history before deciding whether to perform operations - avoid redundant calls
**File Operations**: Use specific patterns like `"src/*.rs"` instead of `"*.rs"` to locate files in the src directory
**Repeat operations**: If your history already has a file read, do not read it again - as this will overload the history. Likewise, if you are doing an edit - do not attempt to do it multiple times, if something fails, ask the user to verify.

**Tools Documentation**: For best practices on using tools efficiently (especially the `read_file` tool), see [docs/tools/README.md](docs/tools/README.md) for an overview, or [docs/tools/QUICK_REFERENCE.md](docs/tools/QUICK_REFERENCE.md) for immediate guidance.

## Useful shortcuts

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
cargo run -- --stream --interactive --llama-cpp-url http://192.168.0.201:8081 --auto-confirm
```

## Project Overview

have a look at [AGENTS.md](AGENTS.md) for more info

### Project exploration
IMPORTANT: you can save a lot of effort by using file_curly_glance tool to look at the *.rs files! It will give you the parentheses spans - and if you
give the start line inside the span - then you can recursively peek inside the spans too! Read [docs/tools/curly_glance_usage.md](docs/tools/curly_glance_usage.md) for more info.


# Tool Usage Best Practices

## 🚀 Subagents: Your Best Weapon Against Context Rot

**Use `launch_subagent_pretty` liberally!** Subagents are independent workers that return clean, summarized JSON output. This dramatically reduces context window usage.

### When to Use Subagents

- **Investigating code**: "Find all usages of function X", "Describe the architecture of module Y"
- **Independent subtasks**: Any self-contained task that doesn't require shared state
- **Information gathering**: Research, documentation lookup, file analysis
- **Parallel work**: Multiple independent investigations (one subagent at a time, wait for results)

### Why Subagents Save Context

- ✅ Return **summarized results** instead of raw tool output
- ✅ Run **independently** - their internal conversation doesn't pollute your context
- ✅ Provide **structured JSON** output that's easy to parse
- ✅ Can be **launched with specific tasks** and return focused answers

### Example Usage

Instead of:
1. Reading multiple files yourself (context bloat)
2. Running multiple commands (more output)
3. Trying to remember what you found

**Launch a subagent**:
```
Task: "Find all functions in src/ that handle authentication and summarize their purpose"
```

The subagent will explore the codebase and return a clean summary like:
```json
{
  "functions_found": 3,
  "summary": "Auth handled by: auth::login(), auth::validate_token(), auth::logout()",
  "files_modified": ["src/auth.rs"]
}
```

**This is your primary defense against context rot!**

## read_file Tool

### IMPORTANT: Use 20-line chunks for large files!

**Do NOT read entire large files at once.** The `read_file` tool has a default behavior of reading the entire file, which can cause:
1. **Output truncation** - Very large files may be cut off
2. **Tool call overhead** - Reading full files costs extra iterations
3. **Context bloat** - Unnecessary content competes with conversation history limits

### Recommended Usage Pattern

```rust
// ❌ BAD - Reads entire file (potential issues above)
read_file {
  "file_path": "src/main.rs"
}

// ✅ GOOD - Read in 20-line chunks
read_file {
  "file_path": "src/main.rs",
  "limit": 20
}

// ✅ GOOD - Read specific range
read_file {
  "file_path": "src/main.rs",
  "offset": 101,
  "limit": 30
}
```

### When to Use Full File Read

Only read entire files if:
- The file is **small** (< 50 lines)
- You need to see the **complete context** (e.g., small config files)
- You're doing a **quick scan** of structure

### Pro Tip: Combine with curly_glance

For large code files, use `file_curly_glance` to find sections of interest, then use `read_file` with line ranges to read specific parts:

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
read_file {
  "file_path": "src/main.rs",
  "offset": 42,
  "limit": 25
}
```

This iterative approach is **far more efficient** than reading entire files!
