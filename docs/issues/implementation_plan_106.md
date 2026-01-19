# Implementation Plan for Issue 106: Update REPL Loop to Use MSPC Channel

## Current State Analysis

The code currently:
1. ✅ Creates MSPC channel
2. ✅ Spawns terminal input router
3. ❌ Reads directly from readline instead of MSPC channel
4. ❌ Doesn't use output destinations
5. ❌ No MSPC chat loop integration

## Required Changes

### 1. Create TerminalOutputDestination
- Implement OutputDestination trait for terminal output
- Handle different message types (user, assistant, tool calls, errors)
- Colorize output appropriately

### 2. Refactor REPL Loop
- Replace direct readline calls with MSPC channel reception
- Use output destinations for all messages
- Maintain backward compatibility for commands

### 3. Create MSPC Chat Loop Integration
- Process messages from MSPC channel
- Route to appropriate handlers
- Handle interruptions and commands

## Implementation Steps

### Step 1: Create TerminalOutputDestination
**File: `src/mspc/output.rs`**
- Add TerminalOutputDestination struct
- Implement OutputDestination trait
- Handle formatting and display

### Step 2: Update REPL Main Loop
**File: `src/app/repl.rs`**
- Replace readline loop with MSPC receiver loop
- Process messages from channel
- Route to appropriate handlers

### Step 3: Test Integration
- Ensure all existing functionality works
- Test command parsing
- Test interruption handling
- Test output formatting

## Files to Modify
1. `src/mspc/output.rs` - Add TerminalOutputDestination
2. `src/app/repl.rs` - Refactor main loop to use MSPC

## Testing Strategy
1. Unit tests for TerminalOutputDestination
2. Integration tests for REPL loop
3. Manual testing of all commands
4. Verify output formatting
