# MSPC Code Cleanup Plan

## Goal
Clean up unused MSPC code and documentation in small, verifiable increments.

## Analysis

### Unused Code Identified:
1. **crates/apchat-mspc/src/output.rs** - Complete module not used anywhere
2. **Unused MSPC message variants** - Some variants in MspcMessage enum may not be used
3. **Unused functions** - Some functions in channel.rs may not be used

### Used Code (Must Keep):
1. **crates/apchat-mspc/src/channel.rs** - MspcChannel struct and most functions
2. **MspcMessage variants** - UserInput, InterruptSignal, Command, ConfirmationRequest, ConfirmationResponse, ToolResult, Error, SystemPrompt
3. **MessagePair, HistoryError** - Used for message history

## Implementation Plan

### Phase 1: Remove Unused Output Module

**Task 1: Remove output.rs module**
- Delete crates/apchat-mspc/src/output.rs
- Remove output module from lib.rs exports
- Remove unused imports from apchat-main/src/app/repl.rs

**Verification:**
- Code compiles successfully
- No functional changes to MSPC behavior

### Phase 2: Clean Up Documentation

**Task 2: Remove or update MSPC-related documentation**
- Review docs/plans/2026-01-18-mspc-multi-source-input.md
- Remove outdated sections about OutputDestination trait
- Keep relevant sections about core MSPC functionality

**Verification:**
- Documentation matches actual implementation
- No references to removed code

### Phase 3: Verify No Unused Code Remains

**Task 3: Final verification**
- Run cargo check --all
- Ensure all tests pass
- Verify no warnings about unused code

## Success Criteria
- All code compiles without errors
- No unused code remains in MSPC module
- Documentation matches implementation
- All existing functionality preserved
