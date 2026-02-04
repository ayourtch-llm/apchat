# Issue 141: Replace println/eprintln with Heart Emoji Functions

## Summary

Replace all uses of `println!` and `eprintln!` macros with the project's standardized `print_heart_red` and `print_heart_yellow` functions to ensure consistent output formatting with heart emoji prefixes.

## Location

### Files requiring changes:
- `crates/apchat-terminal/src/manager.rs` (3 uses of `eprintln!`)
- `apchat-main/src/app/repl/input_router.rs` (1 use of `println!`)
- `apchat-main/src/app/repl.rs` (1 use of `println!`)
- `apchat-main/src/mspc/router.rs` (1 use of `eprintln!`)

### Functions to use:
- `print_heart_red(text: &str, newline: bool)` - for `println!` (outputs to stdout with ❤️)
- `print_heart_yellow(text: &str, newline: bool)` - for `eprintln!` (outputs to stderr with 💛)

## Current Behavior

Currently, the code uses standard Rust macros:
- `println!` - prints to stdout without any emoji prefix
- `eprintln!` - prints to stderr without any emoji prefix

This creates inconsistent output formatting compared to the rest of the application which uses heart emoji prefixes.

## Expected Behavior

All output should use the standardized functions that:
- Prepend the appropriate heart emoji (❤️ for red, 💛 for yellow) to each line
- Ensure proper routing through the OutputRouter system (Issue 138)
- Maintain consistent visual styling across the application

## Impact

- **Consistency**: Ensures all output follows the same visual pattern
- **User Experience**: Provides clearer visual distinction between different output types
- **System Integration**: Ensures output routes through the proper output router system

## Suggested Implementation

### Changes needed:

1. **crates/apchat-terminal/src/manager.rs**:
   - Add import: `use apchat_vty::print_heart_yellow;`
   - Replace 3 `eprintln!` calls with `print_heart_yellow`:
     - Line 44: `eprintln!("⚠️  Failed to initialize tmux backend: {}", e);`
     - Line 45: `eprintln!("⚠️  Falling back to PTY backend");`
     - Line 52: `eprintln!("Terminal backend: {}", backend.backend_name());`

2. **apchat-main/src/app/repl/input_router.rs**:
   - Add import: `use apchat_vty::print_heart_red;`
   - Replace 1 `println!` call with `print_heart_red`:
     - Line 50: `println!("");` → `print_heart_red("", true);`

3. **apchat-main/src/app/repl.rs**:
   - Add import: `use apchat_vty::print_heart_red;`
   - Replace 1 `println!` call with `print_heart_red`:
     - Line 388: `println!("");` → `print_heart_red("", true);`

4. **apchat-main/src/mspc/router.rs**:
   - Add import: `use apchat_vty::print_heart_yellow;`
   - Replace 1 `eprintln!` call with `print_heart_yellow`:
     - Line 57: `eprintln!("OutputRouter: Failed to send to {}: {}", dest_id, e);`

### Important Caveat:

**DO NOT modify the `TextOutput` printing implementation in `apchat-main/src/mspc/destinations.rs`.** The `TerminalDestination` and `FileDestination` implementations that directly use `write!` and `writeln!` for `TextOutput` must remain unchanged. These implementations use the `TextOutput.emoji` field directly, and changing them would break the intended emoji hierarchy (emoji from `text.emoji` is the primary emoji to display, not the heart emoji).

Only the standalone `println!`, `eprintln!`, `print!`, `eprint!`, `write!`, and `writeln!` calls that do NOT involve `TextOutput` message handling should be replaced.

### Verification:
After changes, run:
```bash
cargo build --release
cargo run --release
```

Verify output shows heart emojis consistently across all print statements.

---
*Created: 2026-02-04*
