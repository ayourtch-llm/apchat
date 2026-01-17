# Curly Glance Usage Guide

## Key Feature: Drill-Down Pattern

The `file_curly_glance` tool uses a drill-down pattern for iterative exploration.

### How to Use:
1. Call without `starting_line` to see top-level structure
2. Specify `starting_line` at opening curly brace to drill into sections
3. Repeat to go deeper

### Example:
```bash
# See overall structure
file_curly_glance {"file_path": "src/file.rs"}

# Drill into impl block at line 50
file_curly_glance {"file_path": "src/file.rs", "starting_line": 50}

# Drill into nested section at line 291
file_curly_glance {"file_path": "src/file.rs", "starting_line": 291}
```

## Benefits:
- Avoids information overload
- Matches how humans read code
- Handles complex files efficiently
- Automatic brace matching

## Best Practice:
Always start broad, then drill down systematically.
