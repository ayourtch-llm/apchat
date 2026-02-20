## 📋 QUICK REFERENCE: read_file Tool Best Practices

### ⚠️ CRITICAL RULE
**Do NOT read entire large files!** Use 20-line chunks instead.

### 🔧 Recommended Usage
```rust
// FOR LARGE FILES (>50 lines)
read_file {
  "file_path": "src/main.rs",
  "limit": 20,
  "offset": 100  // Optional
}

// FOR SMALL FILES (<50 lines)
read_file {
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
read_file {
  "file_path": "src/main.rs",
  "offset": 42,
  "limit": 25
}
```

### 📚 Full Documentation
- `docs/tools/read_file_usage.md` - Detailed guide
- `docs/tools/file_reading_quick_reference.md` - Quick cheat sheet
- `docs/tools/README.md` - All tools documentation
