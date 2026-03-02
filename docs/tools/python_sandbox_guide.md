# Python Sandbox Tool Guide

The Python sandbox allows you to execute Python code in a secure, sandboxed environment with full access to all agent tools as Python functions.

## Key Characteristics

### Return Types
**All agent tools return strings**, not native Python objects. You'll need to parse the output:

```python
# Example: list_files returns a formatted string
output = list_files(pattern="*.md")
# Output looks like: "Found 277 file(s) matching '*.md':\nfile1.md\nfile2.md\n..."

# Parse it:
lines = output.strip().split('\n')
header = lines[0]  # "Found 277 file(s)..."
files = lines[1:]  # Actual file paths
```

### Common Tools & Parsing Patterns

| Tool | Returns | Parsing Needed |
|------|---------|----------------|
| `list_files(pattern)` | Formatted list | Split by `\n`, skip header |
| `read_file(path, limit)` | File content | Usually ready to use |
| `search_files(query, pattern)` | Search results | Parse formatted output |
| `run_command(cmd)` | Command output | Extract from formatted string |
| `file_curly_glance(path)` | Structure info | Ready to use |
| `launch_subagent_pretty(task)` | JSON-like summary | Ready to use |
| `todo_write(todos)` | Status message | Ready to use |

## Usage Examples

### Basic File Operations
```python
# Read a file with limit
content = read_file(file_path="README.md", limit=20)
print(content[:500])

# List and count files
output = list_files(pattern="**/*.rs")
lines = output.strip().split('\n')
file_count = len(lines) - 1  # Subtract header
print(f"Found {file_count} Rust files")
```

### Search and Analysis
```python
# Search for patterns
results = search_files(query="fn main", pattern="src/**/*.rs", max_results=10)
print(results)

# Analyze code structure
structure = file_curly_glance(file_path="src/main.rs")
print(structure)
```

### Running Commands
```python
# Execute shell commands
result = run_command(command="ls -la")
print(result)
```

### Using Subagents
```python
# Launch a subagent for independent tasks
result = launch_subagent_pretty(
    task="Find all authentication functions in the codebase",
    auto_confirm=True
)
print(result)
```

### Task Management
```python
import json

# Update todo list
todos = json.dumps([
    {"content": "Task 1", "status": "completed", "activeForm": "Doing task 1"},
    {"content": "Task 2", "status": "in_progress", "activeForm": "Doing task 2"}
])
todo_write(todos=todos)

# View todos
todo_list()
```

## Best Practices

### 1. Parse Output Correctly
```python
# ❌ Wrong - iterating over characters
files = list_files(pattern="*.md")
for f in files:  # This iterates over characters!
    print(f)

# ✅ Right - parse the string first
files_output = list_files(pattern="*.md")
lines = files_output.strip().split('\n')
files = lines[1:]  # Skip header
for f in files:
    print(f)
```

### 2. Handle Large Outputs
```python
# Some tools can produce large outputs
# Use limits where available
content = read_file(file_path="large_file.rs", limit=50)

# For list_files, the output may be truncated
# Check for truncation markers
output = list_files(pattern="**/*")
if "TRUNCATED" in output:
    print("Output was truncated, use more specific pattern")
```

### 3. Combine Multiple Tools
```python
# Chain operations efficiently
rust_files_output = list_files(pattern="**/*.rs")
files = rust_files_output.strip().split('\n')[1:]

# Analyze first few files
for f in files[:5]:
    structure = file_curly_glance(file_path=f)
    print(f"=== {f} ===")
    print(structure[:500])
```

### 4. Use Subagents for Complex Tasks
```python
# Instead of complex loops, use subagents
result = launch_subagent_pretty(
    task="""
    Analyze all files in apchat-main/src/chat/
    - Count total files
    - List all module files (mod.rs)
    - Identify the main entry point
    """,
    auto_confirm=True
)
```

## Available Tools

All agent tools are available as Python functions:
- File operations: `read_file`, `write_file`, `edit_file`, `list_files`, `search_files`
- File analysis: `peek_file_top_10_lines`, `file_curly_glance`
- Execution: `run_command`, `python_sandbox` (recursive)
- Terminal: `pty_*` functions for terminal management
- Tasks: `todo_write`, `todo_list`
- Agents: `launch_subagent`, `launch_subagent_pretty`
- And many more...

## Limitations

1. **No direct filesystem access** - Must use agent tools
2. **No network access** - Must use `fetch_url` or `web_search`
3. **No subprocess access** - Must use `run_command`
4. **All outputs are strings** - Parse as needed
5. **Sandboxed environment** - No persistent state between executions

## When to Use

✅ **Use Python sandbox when:**
- You need to batch multiple tool calls
- You want to parse/transform tool outputs
- You're doing complex analysis or data processing
- You need to loop over files or results
- You want to prototype workflows

❌ **Don't use when:**
- You need a simple single tool call
- You want interactive exploration
- The task is better suited for direct tool usage

## Tips

1. **Print intermediate results** to understand what you're working with
2. **Check types** - everything is a string until parsed
3. **Use `json.dumps()`** for tools that expect JSON strings
4. **Test incrementally** - run small chunks first
5. **Leverage subagents** for complex independent tasks