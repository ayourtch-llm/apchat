# Readline Lifecycle Fix Plan

## Current Issues Summary

1. **CRITICAL**: Readline editor not saved on exit (line 364, 366, and other break points)
2. **MAJOR**: Dual history system without synchronization
3. **MINOR**: Incomplete error handling for readline operations

## Proposed Solution

### Option 1: Simplified Approach (Recommended)

Use only the custom JSONL history system and remove the editor's built-in history:

**Pros**:
- Single source of truth
- Better control over persistence
- Simpler code

**Cons**:
- Lose some rustyline features
- Need to manage history manually

### Option 2: Hybrid Approach

Keep both systems but ensure proper synchronization:

**Pros**:
- Full rustyline functionality
- Persistent storage

**Cons**:
- More complex
- Potential for duplication
- Harder to maintain

## Implementation Plan (Option 1 - Recommended)

### Step 1: Remove editor history operations

**File**: `apchat-main/src/app/repl.rs`

**Changes**:
1. Remove line 709: `rl.add_history_entry(line)?;`
2. Keep the JSONL save at line 710

### Step 2: Add proper cleanup before exit

**Location**: Add cleanup code before all exit points

**Implementation**:
```rust
// Add this function at the end of run_repl_mode before Ok(())
// Ensure readline history is saved on all exit paths
if let Err(e) = crate::chat::readline_history::save_to_file(
    &crate::chat::readline_history::ReadlineEntry::with_session(
        line,
        format!("session_{}", chat.process_id)
    )
) {
    if chat.debug_level > 0 {
        eprintln!("{} Failed to save readline history on exit: {}", "⚠️".yellow(), e);
    }
}
```

### Step 3: Add cleanup guard

**Implementation**: Use a RAII pattern to ensure cleanup happens even on unexpected exits

```rust
struct ReadlineGuard {
    rl: DefaultEditor,
    last_line: Option<String>,
    session_id: String,
}

impl Drop for ReadlineGuard {
    fn drop(&mut self) {
        if let Some(ref line) = self.last_line {
            if let Err(e) = crate::chat::readline_history::save_to_file(
                &crate::chat::readline_history::ReadlineEntry::with_session(
                    line,
                    self.session_id.clone()
                )
            ) {
                eprintln!("{} Failed to save readline history: {}", "⚠️".yellow(), e);
            }
        }
    }
}
```

### Step 4: Update readline_history module

**File**: `apchat-main/src/chat/readline_history.rs`

**Changes**:
1. Add function to save current line on exit
2. Add error handling improvements
3. Add logging for critical operations

## Testing Strategy

1. **Unit Tests**: Add tests for the ReadlineGuard
2. **Integration Tests**: Test all exit scenarios (Ctrl+D, Ctrl+C, exit command)
3. **Manual Testing**: Verify history persists across sessions
4. **Stress Testing**: Test with many commands to ensure no performance issues

## Rollout Plan

1. **Phase 1**: Implement Option 1 with cleanup guard
2. **Phase 2**: Add comprehensive error handling
3. **Phase 3**: Add monitoring/logging for history operations
4. **Phase 4**: Consider adding metrics for performance tracking

## Risk Mitigation

1. **Backup history**: Before changes, document current history location
2. **Feature flag**: Consider making history changes opt-in initially
3. **Migration script**: If changing format, provide migration tool
4. **Rollback plan**: Keep old code available for quick revert if issues arise

## Files to Modify

1. `apchat-main/src/app/repl.rs` - Main fix location
2. `apchat-main/src/chat/readline_history.rs` - Enhancements
3. Possibly `apchat-main/src/main.rs` - If adding cleanup at higher level

## Estimated Effort

- **Low complexity**: 2-4 hours
- **Medium complexity**: 4-8 hours
- **High complexity**: 8-16 hours

**Estimate**: Medium complexity, 4-6 hours total
