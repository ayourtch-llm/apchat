# Plans Directory Index

This directory contains implementation plans and design documents for various features.

## Recent Plans (2025-01-23)

### Multiline Editor Implementation 🚀
**Status:** Ready to implement
**Primary:** `SESSION-STARTER.md`
**Supporting files:**
- `multiline-editor.md` - Quick reference implementation guide
- `multiline-paste-plan.md` - Original paste problem analysis
- `multiline-paste-implementation.md` - Bracketed paste implementation details
- `test-paste.rs` - Test program for paste event detection

**What it implements:**
- Shift-Enter for newline insertion
- Enter to submit (when at end) or insert newline
- Arrow key navigation between lines
- Paste that preserves newlines
- Up to 10 lines with scrolling
- Full history support for multiline input

**How to use:**
1. Read `SESSION-STARTER.md` for complete context
2. Follow the implementation steps in order
3. Reference `multiline-editor.md` for code snippets
4. Test frequently using the checklist

---

## Archive

### 2024 Plans
- `2024-06-10-llm-tool.md` - LLM tool design
- `2025-01-21-workspace-reorganization.md` - Workspace structure changes

### 2025-01 Crossterm Migration
- `2025-01-23-crossterm-readline-implementation.md` - Original crossterm implementation
- `2025-01-23-crossterm-readline-implementation-SUMMARY.md` - Migration summary
- `2025-01-23-crossterm-readline-migration.md` - Migration plan
- `2025-01-23-CROSSTERM-READLINE-MIGRATION-COMPLETE.md` - Completion report

### 2025-06+ Enhancements
- `2025-05-13-auto-history-saving.md` - History persistence
- `2025-06-18-compact-command.md` - Compact command design
- `2025-06-20-refactor-hardcoded-models.md` - Model refactoring
- `2025-06-23-llm-color-refactoring.md` - Color system refactoring
- `2025-07-15-readline-history-persistence.md` - History persistence details
- `2025-07-25-content-length-limiter.md` - Content length limiting

### 2026-01 Enhancements
- `2026-01-03-fetch-url-tool-design.md` - Fetch URL tool design
- `2026-01-17-curly-glance-fix.md` - Curly glance tool fix
- `2026-01-17-date-injection-system-prompt.md` - Date injection system
- `2026-01-17-file-curly-glance.md` - File curly glance implementation
- `2026-01-17-persistent-memory.md` - Persistent memory design
- `2026-01-18-input-decoupling.md` - Input decoupling design
- `2026-01-18-input-decoupling-implementation.md` - Input decoupling implementation
- `2026-01-18-input-decoupling-implementation-detailed.md` - Detailed implementation
- `2026-01-18-mspc-multi-source-input.md` - MPSC multi-source input
- `2026-01-19-webex-bot-design.md` - WebEx bot design
- `2026-01-19-webex-websocket-design.md` - WebEx WebSocket design
- `2026-01-19-webex-websocket-implementation.md` - WebEx WebSocket implementation
- `2026-01-20-vty-output-heart-emojis-design.md` - Heart emoji output design

---

## File Naming Convention

Plans use the format: `YYYY-MM-DD-description.md`

Where:
- `YYYY` = Year
- `MM` = Month (01-12)
- `DD` = Day (01-31)
- `description` = Brief description in kebab-case

---

## How to Use These Plans

1. **Find the relevant plan** for what you're working on
2. **Read the full plan** to understand the context and approach
3. **Follow the implementation steps** in order
4. **Test as you go** using the provided checklists
5. **Update the plan** if you find issues or improvements

---

## Creating New Plans

When creating a new implementation plan:

1. **Use the date format** in the filename
2. **Include sections:**
   - Problem statement
   - Proposed solution
   - Implementation steps
   - Testing checklist
   - Edge cases to handle

3. **Cross-reference** related plans
4. **Update this index** to include the new plan

---

## Status Legend

- 🚀 Ready to implement
- 🔄 In progress
- ✅ Complete
- 📋 Planning
- ⏸️ On hold
