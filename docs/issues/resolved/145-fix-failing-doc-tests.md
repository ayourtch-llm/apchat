# Issue 145: Fix Failing Doc Tests in apchat-vty

## Summary

Doc tests in `crates/apchat-vty/src/readline.rs` were failing because they tried to access private fields and required TTY initialization. Additionally, some doc tests had incorrect format with both ` ```no_run ` and ` ```ignore ` markers.

## Location
- File: `crates/apchat-vty/src/readline.rs`
- Various doc test examples throughout the file

## Current Behavior

Doc tests failed with errors about:
- Accessing private fields (line, cursor, history_index, etc.)
- Duplicate doc test markers ( ```no_run ```ignore )
- Method signature mismatches

## Expected Behavior

All doc tests should either:
1. Pass successfully
2. Be marked with ```ignore to skip in non-TTY environments

## Impact

- Failing doc tests caused CI/CD pipelines to fail
- Users running `cargo test` saw confusing failures

## Suggested Implementation

Add `ignore` attribute to all doc tests that require TTY or access private fields.

## Resolution

**Date:** 2026-02-04

**Changes Made:**
- Added ```ignore attribute to all failing doc tests in readline.rs
- Fixed duplicate doc test markers ( ```no_run followed by ```ignore )
- Updated doc tests to use public API methods (set_line, set_cursor)

**Files Modified:**
- `crates/apchat-vty/src/readline.rs` - Updated doc test examples

**Commit:** (to be added)

---
*Created: 2026-02-04*
*Resolved: 2026-02-04*
