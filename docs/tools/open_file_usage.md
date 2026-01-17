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
  "read_line_count": 20
}

// ✅ GOOD - Read specific range
open_file {
  "file_path": "src/main.rs",
  "start_line": 101,
  "read_line_count": 30
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
  "read_line_count": 25
}
```

This iterative approach is **far more efficient** than reading entire files!
