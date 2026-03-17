# File Curly Glance - Implementation Analysis

## Current Issues with Implementation

Based on the requirements and current implementation in `crates/apchat-tools/src/file_curly_glance.rs`, here are the key problems:

### 1. **Output Format Mismatch**
**Required:** `"10..20: fn main() { ... }"` (one line per pair)
**Current:** Multi-line output with detailed bracket information and content

### 2. **Top-Level Bracket Detection Missing**
**Required:** Only find top-level curly brackets (depth = 0)
**Current:** Finds all curly brackets without checking nesting depth

### 3. **Whitespace Line Detection Missing**
**Required:** Search backwards from opening bracket to find preceding empty/whitespace-only line
**Current:** No backward search implemented

### 4. **Starting Line Logic Flawed**
**Required:** When `starting_line` is provided, stop at extraneous closing bracket
**Current:** Simply skips lines before starting_line but continues processing

### 5. **No Range Calculation**
**Required:** Calculate line range from opening to closing bracket
**Current:** Only tracks individual line numbers

## What Needs to Be Fixed

1. **Rewrite `analyze_file_content()`** to produce compact one-line output
2. **Track nesting depth** to only find top-level brackets
3. **Implement backward search** to find preceding whitespace line
4. **Fix starting_line handling** to stop at extraneous closing bracket
5. **Extract opening line content** for display in output

## Implementation Plan

1. Parse file to find all top-level bracket pairs (depth = 0)
2. For each top-level opening bracket:
   - Find matching closing bracket
   - Search backward to find preceding empty/whitespace line
   - Extract the line containing the opening bracket
   - Format output as "start..end: line_content"
3. When starting_line is provided:
   - Only process brackets appearing at or after starting_line
   - Stop processing when encountering closing bracket with no matching opening
