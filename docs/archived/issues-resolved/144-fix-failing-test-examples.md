# Issue 144: Fix Failing Test Examples in apchat-vty

## Summary

The example files in `crates/apchat-vty/examples/` failed to compile because they referenced methods that don't exist (`try_take_test_lock()`, `caller()` on `TestLock`).

## Location
- File: `crates/apchat-vty/examples/test_raii_lock.rs`
- File: `crates/apchat-vty/examples/test_lock_pattern.rs`
- File: `crates/apchat-vty/src/instance.rs`

## Current Behavior

Examples failed to compile with errors about missing methods.

## Expected Behavior

Examples should compile and run successfully.

## Impact

Examples could not be built or tested.

## Suggested Implementation

Add missing methods and fix example code.

## Resolution

**Date:** 2026-02-04

**Changes Made:**
- Added `caller()` method to `TestLock` struct
- Added `try_take_test_lock()` method to `ReadlineInstance` struct
- Fixed method name discrepancies in example files

**Files Modified:**
- `crates/apchat-vty/src/instance.rs` - Added methods to structs
- `crates/apchat-vty/examples/test_raii_lock.rs` - Fixed method calls
- `crates/apchat-vty/examples/test_lock_pattern.rs` - Fixed method calls

**Commit:** 99ca56a

---
*Created: 2026-02-04*
*Resolved: 2026-02-04*
