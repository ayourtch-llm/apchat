# Multiline Editor - Implementation Ready ✨

## Quick Links

**Start Here:** `docs/plans/SESSION-STARTER.md` - Complete session starter guide

**Implementation Plan:** `docs/plans/multiline-editor.md` - Quick reference with code snippets

**Background:** `docs/plans/multiline-paste-implementation.md` - How we got here

**Plans Index:** `docs/plans/README.md` - All available plans

---

## What's Being Implemented

A full multiline REPL editor that allows:

✅ **Shift-Enter** to insert newlines
✅ **Enter** to submit (when at end) or insert newline
✅ **Arrow keys** to navigate between and within lines
✅ **Paste** that preserves newlines (instead of joining with spaces)
✅ **Up to 10 lines** of editable input with scrolling
✅ **Full history support** for multiline input

---

## Quick Start for Fresh Session

```bash
# 1. Read the session starter guide
cat docs/plans/SESSION-STARTER.md

# 2. Create a feature branch
git checkout -b feature/multiline-editor

# 3. Start with Phase 1 (data structure migration)
# See SESSION-STARTER.md for detailed steps
```

---

## What's Already Done

Recent commits (context for implementation):

```
dba90c6 docs: Update readline documentation with new features
b0a80e3 feat: Add multiline paste support with bracketed paste mode
0ba4e37 feat: Add Ctrl-A and Ctrl-E keybindings
9f534bd fix: Handle Ctrl-C and Ctrl-D correctly in readline
```

**Working now:**
- ✅ Bracketed paste mode enabled (terminal sends paste events)
- ✅ Ctrl-A, Ctrl-E move to start/end of line
- ✅ Ctrl-C sends interrupt, Ctrl-D sends EOF
- ✅ Paste events detected (currently join lines with spaces)

**The limitation:**
- Single line buffer only
- Paste replaces newlines with spaces (workaround)
- No way to manually insert newlines

---

## Implementation Overview

### The Core Change

**Before:**
```rust
pub struct Readline {
    line: String,      // Single line
    cursor: usize,     // Position in line
}
```

**After:**
```rust
pub struct Readline {
    lines: Vec<String>,    // Multiple lines
    cursor_line: usize,    // Which line we're on
    cursor_col: usize,     // Position in line
    max_lines: usize,      // Max display (10)
    scroll_offset: usize,  // For scrolling
}
```

### Implementation Phases

1. **Phase 1:** Update data structures (struct, constructor)
2. **Phase 2:** Add multiline operations (newline, backspace, delete)
3. **Phase 3:** Update navigation (arrow keys, home/end)
4. **Phase 4:** Update Enter key behavior (Shift-Enter vs regular)
5. **Phase 5:** Update paste handling (preserve newlines)
6. **Phase 6:** Update history (save/restore multiline)
7. **Phase 7:** Update display (redraw with scrolling)
8. **Phase 8:** Testing and polish

**Estimated effort:** 4-6 hours, ~300-400 lines of code

---

## Key Files

### Implementation
```
crates/apchat-vty/src/readline.rs
```
- Line ~176: Struct definition
- Line ~209: Constructor
- Line ~288: Accessor methods
- Line ~500: History methods
- Line ~620: Character insertion
- Line ~650: Backspace
- Line ~720: Delete
- Line ~950: Redraw
- Line ~1070: Paste handling
- Line ~1190: Key event handler (Enter key)
- Line ~1250: Arrow keys

### Documentation
```
docs/plans/SESSION-STARTER.md          - START HERE
docs/plans/multiline-editor.md         - Implementation guide
docs/plans/multiline-paste-plan.md     - Original analysis
docs/plans/multiline-paste-implementation.md - Paste implementation
docs/plans/README.md                   - Plans index
```

---

## Example: What It Will Look Like

### Before (Current)
```
> function test() {
|   return "hello";
| }
> [User pastes, but gets 3 separate submissions]
```

### After (With Multiline Editor)
```
> function test() {
|   return "hello";
| }
> [User presses Shift-Enter between lines, Enter to submit at end]
```

Or even better - paste will work natively:
```
> [User pastes 3 lines]
> function test() {
|   return "hello";
| }_
> [All 3 lines appear, user can edit, then press Enter to submit]
```

---

## Testing Checklist

When implementation is complete, test:

- [ ] Shift-Enter creates newline
- [ ] Enter submits when at end of last line
- [ ] Enter inserts newline when not at end
- [ ] Up/Down arrows navigate between lines
- [ ] Left/Right arrows navigate within/across lines
- [ ] Home/End go to start/end of current line
- [ ] Backspace joins lines when at line start
- [ ] Delete joins lines when at line end
- [ ] Paste single line inserts at cursor
- [ ] Paste multiline creates multiple lines
- [ ] History saves multiline state
- [ ] History restores multiline state
- [ ] Scroll works when > 10 lines
- [ ] Ctrl-C works with multiline input
- [ ] Ctrl-D works with multiline input

---

## Common Pitfalls to Avoid

### ❌ Don't Do This
```rust
// WRONG: Direct byte indexing
self.line[self.cursor] = 'x';

// WRONG: Assuming ASCII
let byte_pos = self.cursor_col;
```

### ✅ Do This Instead
```rust
// RIGHT: Convert char pos to byte pos
let byte_pos = line.chars().take(cursor_col)
    .map(|c| c.len_utf8()).sum();
line.insert(byte_pos, 'x');
```

---

## Getting Help

If you get stuck:

1. **Check the session starter** - `docs/plans/SESSION-STARTER.md`
2. **Read the implementation guide** - `docs/plans/multiline-editor.md`
3. **Review the paste implementation** - `docs/plans/multiline-paste-implementation.md`
4. **Check recent commits** - See what's already working

---

## Git Workflow

```bash
# Create feature branch
git checkout -b feature/multiline-editor

# Work in phases, commit frequently
git add -A
git commit -m "wip: Phase 1 - data structure migration"

# When done
git commit -m "feat: Implement multiline editor

- Shift-Enter to insert newlines
- Enter to submit when at end
- Arrow keys navigate between lines
- Paste preserves newlines
- History supports multiline input"

# Merge to main
git checkout main
git merge feature/multiline-editor
```

---

## Status

🎯 **Ready to implement** - All planning complete, just needs execution

📚 **Well documented** - Session starter guide has everything needed

✅ **Feasible** - 4-6 hours, straightforward refactoring

🚀 **Go for it!**

---

**Last updated:** 2025-01-23
**Status:** Implementation ready
**Next step:** Read `docs/plans/SESSION-STARTER.md` and start Phase 1
