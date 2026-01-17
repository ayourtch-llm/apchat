# Quick Reference: Reading Files Efficiently

## 🚀 Best Practice Summary

### For Large Files (>50 lines):
```bash
# Always use chunked reading
open_file {
  "file_path": "src/main.rs",
  "read_line_count": 20,
  "start_line": 100  # Optional: start from specific line
}
```

### For Small Files (<50 lines):
```bash
# Full read is fine
open_file {
  "file_path": "src/config.toml"
}
```

### Advanced: Use curly_glance First
```bash
file_curly_glance {"file_path": "src/main.rs"}  # Get structure
file_curly_glance {"file_path": "src/main.rs", "starting_line": 42}  # Drill down
open_file {"file_path": "src/main.rs", "start_line": 42, "read_line_count": 25}  # Read chunk
```

## ⚠️ Why This Matters

- **Saves iterations** - Each full file read costs a tool call
- **Prevents truncation** - Large files won't be cut off
- **Better context management** - Keeps conversation history clean
- **More precise** - Focus on what you actually need

## 💡 Pro Tip

Create aliases in your mind for common patterns:
- `read_chunk(path, start=0, count=20)` → use `open_file` with params
- `search_section(path, keyword)` → use `search_files`, then `read_chunk`
