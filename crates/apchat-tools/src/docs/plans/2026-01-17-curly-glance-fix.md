# Plan: 2026-01-17 Curly Glance Fix

## Objectives

1. **Track bracket depth** - Only find top-level brackets at depth 0
2. **Match opening { with closing }** - Correctly pair brackets
3. **Find preceding empty/whitespace-only line** - For each opening bracket
4. **Handle starting_line parameter** - Start scanning from that line and stop at first unmatched closing bracket
5. **Calculate line ranges** - start..end format
6. **Update output formatting** - Single-line format as specified

## Current Implementation Analysis

The current implementation in `file_curly_glance.rs`:
- ✓ Tracks bracket depth
- ✓ Matches opening { with closing }
- ✓ Finds preceding whitespace lines
- ✓ Has starting_line parameter
- ✗ Output format is multi-line (needs to be single-line)
- ✗ May have issues with starting_line logic

## Required Changes

### 1. Output Formatting

Change from multi-line format:
```
3..10: main (preceded by whitespace at line 2)
12..18: another_function (preceded by whitespace at line 11)
```

To single-line format:
```
3..10: main, 12..18: another_function
```

### 2. Algorithm Fixes

1. **Top-level bracket detection**: Ensure we only match at depth 0
2. **starting_line handling**: 
   - Start processing at the specified line
   - Stop at first unmatched closing bracket
   - Handle brackets that open before starting_line but close after
3. **Line range calculation**: opening_line..closing_line

### 3. Edge Cases

- Empty files
- Files with no brackets
- Files with unmatched brackets
- Nested brackets (should be ignored for top-level analysis)
- Multiple top-level brackets

## Implementation Plan

1. **Fix the algorithm** in `analyze_file_content()`:
   - Track depth correctly
   - Handle starting_line properly
   - Stop at unmatched closing bracket

2. **Update the formatter** in `format_bracket_pairs()`:
   - Change to single-line output
   - Format: "start..end: context" for each pair, separated by commas

3. **Update tests** to verify:
   - Single-line output format
   - Correct bracket pairing
   - starting_line behavior
   - Preceding whitespace detection
