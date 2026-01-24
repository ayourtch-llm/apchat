## 📋 QUICK REFERENCE: open_file Tool Best Practices

### ⚠️ CRITICAL RULE
**Do NOT read entire large files!** Use 20-line chunks instead.

### 🔧 Recommended Usage
```rust
// FOR LARGE FILES (>50 lines)
open_file {
  "file_path": "src/main.rs",
  "max_line_count": 20,
  "start_line": 100  // Optional
}

// FOR SMALL FILES (<50 lines)
open_file {
  "file_path": "src/config.toml"
}
```

### 🎯 Why This Matters
- ✅ Saves tool call iterations
- ✅ Prevents output truncation
- ✅ Better context management
- ✅ More efficient workflow

### 💡 Pro Tip: Use curly_glance First
```rust
file_curly_glance {"file_path": "src/main.rs"}
// Find the line you want, then:
open_file {
  "file_path": "src/main.rs",
  "start_line": 42,
  "max_line_count": 25
}
```

### 📚 Full Documentation
- `docs/tools/open_file_usage.md` - Detailed guide
- `docs/tools/file_reading_quick_reference.md` - Quick cheat sheet
- `docs/tools/README.md` - All tools documentation
