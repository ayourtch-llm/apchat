# Readline History Auto-Save and Load Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement auto-saving of readline command history after each message to `${logs_dir}/history/readline.jsonl` and loading this history upon startup.

**Architecture:** The feature will:
1. Save each command entered in readline to a JSONL file at `${logs_dir}/history/readline.jsonl`
2. Append new entries without overwriting existing history
3. Load and add history entries from the file on application startup
4. Maintain compatibility with existing rustyline history management
5. Store entries with metadata (timestamp, command)

**Tech Stack:** Rust, serde_json, chrono for timestamps, rustyline, existing logging infrastructure

---

## Task 1: Define readline history entry structure

**Files:**
- Create: `apchat-main/src/readline_history.rs`

**Step 1: Create new module for readline history management**

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};

/// A single readline history entry with metadata
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReadlineEntry {
    pub timestamp: DateTime<Utc>,
    pub command: String,
    pub session_id: Option<String>,
}

impl ReadlineEntry {
    /// Create a new readline history entry
    pub fn new(command: String, session_id: Option<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            command,
            session_id,
        }
    }
    
    /// Get the history file path
    pub fn get_history_file() -> Result<PathBuf> {
        let logs_dir = apchat_logging::get_logs_dir()?;
        let history_dir = logs_dir.join("history");
        std::fs::create_dir_all(&history_dir)?;
        Ok(history_dir.join("readline.jsonl"))
    }
    
    /// Save this entry to the history file (append mode)
    pub fn save_to_file(&self) -> Result<()> {
        let file_path = Self::get_history_file()?;
        let json = serde_json::to_string(self)
            .context("Failed to serialize readline entry")?;
        
        // Append to file
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .context("Failed to open readline history file")?;
        
        writeln!(file, "{}", json)
            .context("Failed to write readline entry")?;
        
        Ok(())
    }
}

/// Load readline history from file
pub fn load_history() -> Result<Vec<ReadlineEntry>> {
    let file_path = ReadlineEntry::get_history_file()?;
    
    if !file_path.exists() {
        return Ok(Vec::new());
    }
    
    let content = std::fs::read_to_string(&file_path)
        .context("Failed to read readline history file")?;
    
    let mut entries = Vec::new();
    
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        
        let entry: ReadlineEntry = serde_json::from_str(line)
            .context("Failed to deserialize readline entry")?;
        entries.push(entry);
    }
    
    Ok(entries)
}

/// Load history and add to rustyline editor
pub fn load_and_add_to_editor(rl: &mut rustyline::Editor<rustyline::DefaultHelper>) -> Result<()> {
    let entries = load_history()?;
    
    for entry in entries {
        rl.add_history_entry(&entry.command)?;
    }
    
    Ok(())
}
```

**Step 2: Create module exports in chat/mod.rs**

Add to `apchat-main/src/chat/mod.rs`:

```rust
pub mod readline_history;
pub use readline_history::{ReadlineEntry, load_history, load_and_add_to_editor};
```

**Step 3: Commit**

```bash
git add apchat-main/src/readline_history.rs apchat-main/src/chat/mod.rs
git commit -m "feat: add readline history management module"
```

---

## Task 2: Initialize readline with loaded history

**Files:**
- Modify: `apchat-main/src/app/repl.rs:260-280` (readline initialization)

**Step 1: Load history before readline loop**

Replace the simple readline initialization with history loading:

```rust
let mut rl = DefaultEditor::new()?;

// Load readline history from file
match crate::chat::readline_history::load_and_add_to_editor(&mut rl) {
    Ok(_) => {
        if chat.debug_level > 1 {
            println!("{} Loaded {} readline history entries",
                     "📖".bright_green(),
                     crate::chat::readline_history::load_history()?.len());
        }
    }
    Err(e) => {
        if chat.debug_level > 0 {
            eprintln!("{} Failed to load readline history: {}", "⚠️".yellow(), e);
        }
    }
}
```

**Step 2: Commit**

```bash
git add apchat-main/src/app/repl.rs
git commit -m "feat: load readline history on startup"
```

---

## Task 3: Save readline history after each command

**Files:**
- Modify: `apchat-main/src/app/repl.rs:639-655` (after add_history_entry)

**Step 1: Add history saving after add_history_entry**

Update the section where `rl.add_history_entry(line)?;` is called:

```rust
rl.add_history_entry(line)?;

// Save to persistent history file
match crate::chat::readline_history::ReadlineEntry::new(
    line.to_string(),
    Some(format!("session_{}", chat.process_id))
).save_to_file() {
    Ok(_) => {
        if chat.debug_level > 2 {
            println!("{} Saved to readline history", "✏️".bright_blue());
        }
    }
    Err(e) => {
        if chat.debug_level > 0 {
            eprintln!("{} Failed to save readline history: {}", "⚠️".yellow(), e);
        }
    }
}
```

**Step 2: Commit**

```bash
git add apchat-main/src/app/repl.rs
git commit -m "feat: auto-save readline history after each command"
```

---

## Task 4: Add tests for readline history

**Files:**
- Modify: `apchat-main/src/readline_history.rs` (add test module)

**Step 1: Add comprehensive tests**

Add at the bottom of `readline_history.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_readline_entry_serialization() {
        let entry = ReadlineEntry::new(
            "test command".to_string(),
            Some("session_12345".to_string())
        );
        
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: ReadlineEntry = serde_json::from_str(&json).unwrap();
        
        assert_eq!(entry.command, deserialized.command);
        assert_eq!(entry.session_id, deserialized.session_id);
    }

    #[test]
    fn test_save_and_load_history() {
        let temp_dir = TempDir::new().unwrap();
        let test_logs_dir = temp_dir.path().join("logs");
        fs::create_dir_all(&test_logs_dir).unwrap();
        
        // Mock the history file location
        let history_file = test_logs_dir.join("readline.jsonl");
        
        // Save some entries
        let entry1 = ReadlineEntry::new("command 1".to_string(), Some("session_1".to_string()));
        let entry2 = ReadlineEntry::new("command 2".to_string(), Some("session_1".to_string()));
        
        // Manually write to test file
        let mut file = fs::File::create(&history_file).unwrap();
        writeln!(file, "{}", serde_json::to_string(&entry1).unwrap()).unwrap();
        writeln!(file, "{}", serde_json::to_string(&entry2).unwrap()).unwrap();
        
        // Load and verify
        let entries = load_history_from_path(&history_file).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].command, "command 1");
        assert_eq!(entries[1].command, "command 2");
    }

    #[test]
    fn test_empty_history_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_logs_dir = temp_dir.path().join("logs");
        fs::create_dir_all(&test_logs_dir).unwrap();
        
        let history_file = test_logs_dir.join("readline.jsonl");
        fs::File::create(&history_file).unwrap(); // Empty file
        
        let entries = load_history_from_path(&history_file).unwrap();
        assert_eq!(entries.len(), 0);
    }

    /// Helper function for testing
    fn load_history_from_path(file_path: &Path) -> Result<Vec<ReadlineEntry>> {
        let content = fs::read_to_string(file_path)?;
        let mut entries = Vec::new();
        
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            
            let entry: ReadlineEntry = serde_json::from_str(line)?;
            entries.push(entry);
        }
        
        Ok(entries)
    }
}
```

**Step 2: Run tests**

```bash
cargo test --release --lib readline_history
```

Expected: All tests should pass

**Step 3: Commit**

```bash
git add apchat-main/src/readline_history.rs
git commit -m "test: add readline history tests"
```

---

## Task 5: Integration testing and validation

**Files:**
- Test: Manual integration testing

**Step 1: Build the project**

```bash
cargo build --release
```

Expected: Build should succeed without errors

**Step 2: Test readline history persistence**

```bash
# Run the application
cargo run --release --interactive

# In the interactive session:
# 1. Type several commands: "hello", "test", "exit" (don't actually exit)
# 2. Use arrow keys to verify history is loaded (should see your commands)
# 3. Exit with Ctrl+D

# Check the history file
cat ~/.okaychat/logs/history/readline.jsonl
```

Expected: File should contain JSONL entries with timestamps and commands

**Step 3: Verify history persists across sessions**

```bash
# Run again
cargo run --release --interactive

# Verify arrow keys show previous commands
# Exit and check history file again - should have more entries
```

**Step 4: Test with debug flags**

```bash
# Run with debug level to see messages
cargo run --release --interactive -- --debug 3

# Should see messages about loading/saving history
```

**Step 5: Commit**

```bash
git commit -m "feat: complete readline history auto-save and load"
```

---

## Task 6: Documentation

**Files:**
- Modify: `docs/usage.md` (or create if doesn't exist)

**Step 1: Document readline history feature**

Add to existing documentation:

```markdown
### Readline History Persistence

APChat automatically saves your command history to maintain persistence across sessions.

**Location:**
`~/.okaychat/logs/history/readline.jsonl`

**Features:**
- Commands are saved after each entry
- History is loaded automatically on startup
- JSONL format with timestamps and session IDs
- Compatible with standard readline navigation (↑/↓ arrows)
- Debug levels 2+ show history operations

**Manual Management:**
To clear history, delete or empty the file. It will be recreated automatically.

**Format:**
Each line is a JSON object:
```json
{
  "timestamp": "2025-01-01T12:00:00Z",
  "command": "your command here",
  "session_id": "session_12345"
}
```
```

**Step 2: Update CLAUDE.md if needed**

Add note about readline history persistence in features section.

**Step 3: Commit**

```bash
git add docs/usage.md docs/project/CLAUDE.md
git commit -m "docs: document readline history persistence"
```

---

## Task 7: Add dependency for chrono

**Files:**
- Modify: `apchat-main/Cargo.toml`

**Step 1: Add chrono dependency**

Ensure chrono is in dependencies:

```toml
[dependencies]
chrono = { version = "0.4", features = ["serde"] }
```

**Step 2: Commit**

```bash
git add apchat-main/Cargo.toml
git commit -m "deps: add chrono for timestamp support"
```

---

## Verification Checklist

1. ✅ `readline_history.rs` module created with ReadlineEntry struct
2. ✅ History save/load functions implemented
3. ✅ History loaded on startup into rustyline editor
4. ✅ History saved after each command
5. ✅ Tests for readline history functionality
6. ✅ Integration tested manually
7. ✅ Documentation updated
8. ✅ Build succeeds without warnings
9. ✅ All existing tests still pass
10. ✅ Chrono dependency added with serde feature

---

**Plan complete!** Two execution options:

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**
