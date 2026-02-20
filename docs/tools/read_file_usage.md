# Tool Usage Best Practices

### IMPORTANT: Avoid large outputs with the tools if you can. Be more specific.

Built-in content limiter will redirect the output to the file if it exceeds 20000 characters.
Be strategic about what you request.

## read_file Tool

### IMPORTANT: Use 40-line chunks for large files!

**Do NOT read entire large files at once.** The `read_file` tool 
WILL refuse to give you the output if it exceeds 20000 bytes. Same thing for all the other tools.

### Recommended Usage Pattern

```rust
// ❌ BAD - Reads entire file (potential issues above)
read_file {
  "file_path": "src/main.rs"
}

// ✅ GOOD - Read in 40-line chunks
read_file {
  "file_path": "src/main.rs",
  "limit": 40
}

// ✅ GOOD - Read specific range
read_file {
  "file_path": "src/main.rs",
  "offset": 101,
  "limit": 20
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
