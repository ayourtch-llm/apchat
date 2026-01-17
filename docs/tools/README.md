# Tools Documentation

This directory contains best practice guides for using the available tools efficiently.

## Available Guides

- **[open_file_usage.md](open_file_usage.md)** - Critical guide for reading files efficiently
  - **Must read!** Explains why you should use 20-line chunks
  - Shows how to avoid tool call overhead and truncation issues

- **[curly_glance_usage.md](curly_glance_usage.md)** - Drill-down pattern for exploring large files
  - Learn the iterative exploration technique
  - Find and focus on specific code sections

- **[file_reading_quick_reference.md](file_reading_quick_reference.md)** - Quick cheat sheet
  - Best practices at a glance
  - Common usage patterns

## Why This Matters

Reading files efficiently is crucial because:
1. **Iteration cost** - Each tool call consumes your available interactions
2. **Context limits** - Large outputs compete with conversation history
3. **Precision** - Focused reading gets you answers faster
4. **Reliability** - Chunked reading prevents truncation

## Recommended Reading Order

1. Start with **file_reading_quick_reference.md** for immediate guidance
2. Read **open_file_usage.md** for detailed best practices
3. Review **curly_glance_usage.md** for advanced exploration techniques

## Need More Help?

Check the main project documentation in `docs/` directory.
