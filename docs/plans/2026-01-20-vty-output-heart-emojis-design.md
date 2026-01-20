# VTY Output with Heart Emojis - Design Document

**Date:** 2026-01-20
**Status:** Approved for implementation

## Overview

Create centralized terminal output functions that prepend heart emojis to every line of output, ensuring consistent formatting across the entire APChat application.

## Module Structure

**File:** `apchat-main/src/app/vty_output.rs`

### Public Functions

1. **`print_heart_red(text: &str, newline: bool)`**
   - Prepends red heart emoji (❤️) to each line
   - Outputs to stdout
   - Conditionally adds trailing newline

2. **`print_heart_yellow(text: &str, newline: bool)`**
   - Prepends yellow heart emoji (💛) to each line
   - Outputs to stderr
   - Conditionally adds trailing newline

### Implementation Details

Both functions will:
1. Split input text by `\n` (handling embedded newlines)
2. For each non-empty line: prepend emoji and print
3. For empty lines: print just the newline (no emoji)
4. Use `print!` or `println!` based on `newline` parameter

**Example behavior:**
```rust
print_heart_red("Hello\nWorld", true);
// Outputs:
// ❤️ Hello
// ❤️ World
// (with trailing newline)

print_heart_yellow("Warning!", false);
// Outputs:
// 💛 Warning! (no trailing newline)
```

## Refactoring Strategy

### Phase 1: Create Module
1. Create `apchat-main/src/app/vty_output.rs` with both functions
2. Export from `apchat-main/src/app/mod.rs`

### Phase 2: Systematic Replacement

**Mapping:**
- All `print!` → `print_heart_red(..., false)`
- All `println!` → `print_heart_red(..., true)`
- All `eprintln!` → `print_heart_yellow(..., true)`

**Process for each file:**
1. Add import: `use crate::app::vty_output::{print_heart_red, print_heart_yellow};`
2. Replace print statements with appropriate heart function
3. Handle multiline strings automatically (function splits on `\n`)

### Files Requiring Changes

**Primary files (~200+ print statements):**
- `crates/apchat-llm-api/src/client/anthropic.rs`
- `crates/apchat-llm-api/src/client/llama_cpp.rs`
- `crates/apchat-logging/src/request_logger.rs`
- `crates/apchat-logging/src/conversation_logger.rs`
- `apchat-main/src/api/streaming.rs`
- `apchat-main/src/app/terminal_output.rs`
- `apchat-main/src/input_router/terminal.rs`
- `crates/apchat-tools/src/*.rs`
- `crates/apchat-toolcore/src/tool_context.rs`
- `crates/apchat-agents/src/visibility.rs`
- And ~10 more files

**Exclusions:**
- Test files: Keep standard macros for clarity
- Comments: No changes needed

## Success Criteria

- All terminal output goes through heart emoji functions
- Every line of output has appropriate heart emoji
- No output bypasses the new functions
- All print/println/eprintln replaced
