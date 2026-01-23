# Issue 112: Add crossterm dependency to apchat-vty

## Summary
Add the crossterm 0.28 dependency to the apchat-vty crate as the first step in implementing a custom readline solution to replace rustyline.

## Location
- File: `crates/apchat-vty/Cargo.toml`

## Current Behavior
The apchat-vty crate currently lacks the crossterm dependency needed for terminal input handling.

## Expected Behavior
The apchat-vty crate should have crossterm 0.28 properly configured in its dependencies.

## Impact
This is the foundational step for the entire crossterm readline implementation. Without this dependency, none of the subsequent readline features can be implemented.

## Implementation Plan

### Step 1: Add crossterm dependency
Edit `crates/apchat-vty/Cargo.toml` and add to dependencies section:
```toml
[dependencies]
crossterm = "0.28"
```

### Step 2: Verify dependency resolution
Run: `cargo check -p apchat-vty`
Expected: Successfully resolves crossterm dependency

### Step 3: Commit
```bash
git add crates/apchat-vty/Cargo.toml
git commit -m "feat: add crossterm dependency for readline implementation"
```

## Verification Criteria
- [ ] crossterm = "0.28" is in Cargo.toml dependencies
- [ ] `cargo check -p apchat-vty` succeeds without errors
- [ ] Changes are committed with appropriate message

## References
- Part of larger plan: `docs/plans/2025-01-23-crossterm-readline-implementation.md`
- Task 1 from implementation plan (14 total tasks)

---
*Created: 2025-01-23*
*Resolved: 2025-01-23*

## Resolution
Successfully implemented crossterm dependency addition:

**Changes Made:**
- Added `crossterm = "0.28"` to `[dependencies]` section in `crates/apchat-vty/Cargo.toml`

**Files Modified:**
- `crates/apchat-vty/Cargo.toml` - Added crossterm dependency

**Verification:**
- ✅ `cargo check -p apchat-vty` succeeded with no errors (crossterm 0.28.1 successfully downloaded and compiled)
- ✅ Only 1 pre-existing warning about unused import in lib.rs (unrelated to this change)
- ✅ Changes committed with SHA: 77907d7d0674ecfa7c78ea763a311f020adc21ce

**Commit Message:**
```
feat: add crossterm dependency for readline implementation
```

All verification criteria from the issue have been met.
