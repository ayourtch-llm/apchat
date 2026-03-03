# Tools Documentation

This directory contains best practice guides for using the available tools efficiently.

## Available Guides

- **[read_file_usage.md](read_file_usage.md)** - Critical guide for reading files efficiently
  - **Must read!** Explains why you should use 20-line chunks
  - Shows how to avoid tool call overhead and truncation issues

- **[curly_glance_usage.md](curly_glance_usage.md)** - Drill-down pattern for exploring large files
  - Learn the iterative exploration technique
  - Find and focus on specific code sections

- **[file_reading_quick_reference.md](file_reading_quick_reference.md)** - Quick cheat sheet
  - Best practices at a glance
  - Common usage patterns

- **[python_sandbox_guide.md](python_sandbox_guide.md)** - Complete guide to the Python sandbox
  - **All agent tools return strings** - learn parsing patterns
  - Examples for file operations, search, subagents, and more
  - Best practices for batching and complex workflows

## Rust Build Best Practices

For efficient Rust development, see the build tips in:
- **[QUICK_REFERENCE.md](QUICK_REFERENCE.md)** - Includes build/check commands and warning suppression

### Quick Build Tips
```bash
# Fast syntax checking
cargo check

# Suppress warnings to reduce context churn
export RUSTFLAGS="-Awarnings"

# See just the last 10 lines
cargo build --release 2>&1 | tail -n 10
```

## Why This Matters

Using tools efficiently is crucial because:
1. **Iteration cost** - Each tool call consumes your available interactions
2. **Context limits** - Large outputs compete with conversation history
3. **Precision** - Focused reading gets you answers faster
4. **Reliability** - Proper usage prevents truncation and errors

## Recommended Reading Order

1. Start with **file_reading_quick_reference.md** for immediate guidance
2. Read **read_file_usage.md** for detailed best practices
3. Review **curly_glance_usage.md** for advanced exploration techniques
4. Check **python_sandbox_guide.md** for batch operations and automation
5. Review **QUICK_REFERENCE.md** for build command tips
