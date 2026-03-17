# Issue 142: Replace println/print in apchat-todo with print_heart_red

## Summary

Replace all uses of `println!` and `print!` macros in `crates/apchat-todo/src/lib.rs` with the project's standardized `print_heart_red` function to ensure consistent output formatting with heart emoji prefixes.

## Location

### Files requiring changes:
- `crates/apchat-todo/src/lib.rs` (6 uses of `println!`)

### Functions to use:
- `print_heart_red(text: &str, newline: bool)` - for `println!` (outputs to stdout with ❤️)

## Current Behavior

Currently, the code uses standard Rust macros:
- `println!` - prints to stdout without any emoji prefix

This creates inconsistent output formatting compared to the rest of the application which uses heart emoji prefixes.

## Expected Behavior

All output should use the standardized functions that:
- Prepend the appropriate heart emoji (❤️) to each line
- Ensure proper routing through the OutputRouter system (Issue 138)
- Maintain consistent visual styling across the application

## Impact

- **Consistency**: Ensures all output follows the same visual pattern
- **User Experience**: Provides clearer visual distinction between different output types
- **System Integration**: Ensures output routes through the proper output router system

## Suggested Implementation

### Changes needed:

1. **crates/apchat-todo/src/lib.rs**:
   - Add import: `use apchat_vty::print_heart_red;`
   - Replace 6 `println!` calls with `print_heart_red`:
     - Line 110: `println!("\n{}"...` → `print_heart_red("\n...", true);`
     - Line 111: `println!("{} Tasks: {}"...` → `print_heart_red("...Tasks: {}...", true);`
     - Line 117: `println!("{}", "...` → `print_heart_red("...", true);`
     - Line 133: `println!("  {} {} {}", ...` → `print_heart_red("  {} {} {}", true);`
     - Line 136: `println!("{}", "...` → `print_heart_red("...", true);`
     - Line 154: `println!("{} {} ({}/{} tasks complete)", ...` → `print_heart_red("...({}/{} tasks complete)", true);`
     - Line 161: `println!("{} All tasks completed! ({}/{})", ...` → `print_heart_red("...All tasks completed! ({}/{})", true);`

### Important Note:

The `TodoManager` is a shared utility used across the application. Ensure that after replacing `println!` with `print_heart_red`, the output properly integrates with the OutputRouter system (Issue 138) if the todo module is meant to route through it.

### Verification:
After changes, run:
```bash
cargo build --release
cargo run --release
```

Verify output shows heart emojis consistently across all print statements in the todo module.

---
*Created: 2026-02-04*
