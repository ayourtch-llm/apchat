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

---

## 🚀 Rust Build & Check Best Practices

### Build Commands

**For syntax/type checking (fast!):**
```bash
cargo check
```

**For quick builds (development):**
```bash
cargo build
```

**For optimized releases (slower but faster runtime):**
```bash
cargo build --release
```

### Why the Difference?
- **`cargo build`**: Debug mode with optimizations disabled - **much faster builds** (seconds to minutes)
- **`cargo build --release`**: Optimized mode with -O2/-O3 - **slower builds** (minutes to hours) but faster runtime

### When to Use What
- **Quick iteration**: `cargo check` (no compilation, just validation)
- **Development testing**: `cargo build` (fast, with debug symbols)
- **Before production/testing performance**: `cargo build --release` (slower build, faster execution)
- **Final release**: `cargo build --release`

### Suppress Warnings (Recommended for Development)
```bash
export RUSTFLAGS="-Awarnings"
cargo check  # or cargo build --release
```
This prevents warning messages from cluttering your context window!

### Quick Output Review
```bash
# See just the last 10 lines of build output
cargo build --release 2>&1 | tail -n 10
cargo check 2>&1 | tail -n 10
```

### When to Use What
- **Quick iteration**: `cargo check` (no compilation, just validation)
- **Before testing**: `cargo build --release` (creates optimized binary)
- **During development**: Use `RUSTFLAGS="-Awarnings"` to reduce noise
- **Debugging builds**: `cargo build` (no optimization)
