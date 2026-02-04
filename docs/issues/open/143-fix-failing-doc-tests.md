# Issue 143: Fix Failing Doc Tests in apchat-vty

## Summary

Several doc tests in `crates/apchat-vty/src/readline.rs` were failing during `cargo test --doc` due to incorrect doc test markers. Specifically:
1. One doc test had a typo `ignoretext` instead of `ignore`
2. Multiple doc tests had duplicate markers (e.g., ` ```ignore ` as both opening and closing)

## Location
- File: `crates/apchat-vty/src/readline.rs`
- Lines: Various doc test examples throughout the file

## Current Behavior

Running `cargo test -p apchat-vty --doc` produced failures due to:
1. Duplicate doc test markers causing compilation errors
2. Incorrect closing markers (` ```ignore ` instead of ` ``` `)

## Expected Behavior

All doc tests should either:
1. Pass successfully (if they can be fixed to use public API)
2. Be marked with ` ```ignore ` as opening and ` ``` ` as closing

## Impact

- Failing doc tests cause CI/CD pipelines to fail
- Users running `cargo test` see confusing failures
- Doc tests don't provide value if they can't run in standard environments

## Suggested Implementation

### Step 1: Fix incorrect doc test markers

For doc tests that use ` ```ignore ` or ` ```no_run ` as opening markers, the closing marker should be just ` ``` ` (backticks only).

### Step 2: Verify all doc tests pass

```bash
cargo test -p apchat-vty --doc
```

All tests should either pass or be ignored.

## Resolution

**Date:** 2026-02-04

**Changes Made:**
- Fixed `ignoretext` typo to `ignore` on line 187
- Fixed duplicate closing markers in multiple doc tests:
  - Lines 218-225: Changed ` ```ignore ` to ` ``` `
  - Lines 330-335: Changed ` ```ignore ` to ` ``` `
  - Lines 344-349: Changed ` ```ignore ` to ` ``` `
  - Lines 417-423: Changed ` ```ignore ` to ` ``` `
  - Lines 456-479: Changed ` ```ignore ` to ` ``` `
  - Lines 535-557: Changed ` ```ignore ` to ` ``` `
  - Lines 592-603: Changed ` ```ignore ` to ` ``` `
  - Lines 615-631: Changed ` ```ignore ` to ` ``` `
  - Lines 787-803: Changed ` ```ignore ` to ` ``` `
  - Lines 833-849: Changed ` ```ignore ` to ` ``` `
  - Lines 897-913: Changed ` ```ignore ` to ` ``` `
  - Lines 957-975: Changed ` ```ignore ` to ` ``` `
  - Lines 1006-1024: Changed ` ```ignore ` to ` ``` `
  - Lines 1057-1071: Changed ` ```ignore ` to ` ``` `
  - Lines 1092-1106: Changed ` ```ignore ` to ` ``` `

**Files Modified:**
- `crates/apchat-vty/src/readline.rs` - Fixed doc test markers

**Commit:** (to be added)

---
*Created: 2026-02-04*
*Resolved: 2026-02-04*