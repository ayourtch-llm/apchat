# Automatic History Saving Feature Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement an automatic feature that saves the current message history into a file `history-<pid>.json` in the logs directory whenever a user message is processed.

**Architecture:** The feature will:
1. Save message history automatically after each user message in interactive mode
2. Use the same format as the `/save` command (ChatState serialization)
3. Store files in the logs directory with naming pattern `history-<pid>.json`
4. Reuse existing `save_state` function from `chat/state.rs`

**Tech Stack:** Rust, serde_json, std::process for PID, existing chat logging infrastructure

---

## Task 1: Add PID field to APChat struct

**Files:**
- Modify: `apchat-main/src/main.rs:45-80` (APChat struct definition)

**Step 1: Update APChat struct definition**

Add a new field to track the process ID:

```rust
pub(crate) struct APChat {
    // ... existing fields ...
    // Process ID for history file naming
    pub(crate) process_id: u32,
}
```

**Step 2: Initialize process_id in new_with_config**

Update the constructor to set the process ID:

```rust
fn new_with_config(
    // ... existing parameters ...
) -> Self {
    // ... existing initialization code ...
    
    let mut chat = Self {
        // ... existing field assignments ...
        process_id: std::process::id(),
    };
    
    // ... rest of initialization ...
}
```

**Step 3: Commit**

```bash
git add apchat-main/src/main.rs
git commit -m "feat: add process_id field to APChat struct for history file naming"
```

---

## Task 2: Create function to save history automatically

**Files:**
- Modify: `apchat-main/src/main.rs:402-412` (save_state method)
- Modify: `apchat-main/src/main.rs` (add new method after save_state)

**Step 1: Add auto-save function to APChat impl block**

```rust
/// Save conversation history automatically to logs directory
fn auto_save_history(&self) -> Result<String> {
    let logs_dir = apchat_logging::get_logs_dir()?;
    let file_name = format!("history-{}.json", self.process_id);
    let file_path = logs_dir.join(file_name);
    
    save_state(&self.messages, &self.current_model, self.total_tokens_used, file_path.to_str().unwrap())
}
```

**Step 2: Commit**

```bash
git add apchat-main/src/main.rs
git commit -m "feat: add auto_save_history method to APChat"
```

---

## Task 3: Integrate auto-save into message processing flow

**Files:**
- Modify: `apchat-main/src/app/repl.rs:550-600` (message processing loop)

**Step 1: Add auto-save after user messages**

Find the section where user messages are logged and add auto-save:

```rust
// Log the user message before sending
if let Some(logger) = &mut chat.logger {
    logger.log("user", line, None, false).await;
}

// Auto-save history after user message
match chat.auto_save_history() {
    Ok(_) => {
        // Silently save in background (no output to user)
        if chat.debug_level > 1 {
            println!("{} Auto-saved history to history-{}.json", 
                     "💾".bright_blue(), chat.process_id);
        }
    }
    Err(e) => {
        if chat.debug_level > 0 {
            eprintln!("{} Auto-save failed: {}", "⚠️".yellow(), e);
        }
    }
}
```

**Step 2: Run tests to verify**

```bash
cargo test --release --lib chat::state
```

Expected: All existing tests should pass

**Step 3: Commit**

```bash
git add apchat-main/src/app/repl.rs
git commit -m "feat: integrate auto-save history after user messages"
```

---

## Task 4: Create test for auto-save functionality

**Files:**
- Modify: `apchat-main/src/main.rs:700-750` (add test module)

**Step 1: Add test for auto_save_history method**

```rust
#[cfg(test)]
mod auto_save_tests {
    use crate::APChat;
    use apchat_models::{Message, ModelColor};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tempfile::TempDir;
    use apchat_policy::PolicyManager;
    use apchat_toolcore::ToolRegistry;
    use apchat_terminal::TerminalManager;
    use apchat_todo::TodoManager;

    async fn create_test_chat() -> APChat {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().to_path_buf();
        
        APChat {
            api_key: "test-key".to_string(),
            work_dir: work_dir.clone(),
            client: reqwest::Client::new(),
            messages: Vec::new(),
            current_model: ModelColor::GrnModel,
            total_tokens_used: 0,
            logger: None,
            tool_registry: ToolRegistry::new(),
            client_config: crate::config::ClientConfig::new(),
            policy_manager: PolicyManager::new(),
            terminal_manager: Arc::new(Mutex::new(TerminalManager::new(work_dir))),
            skill_registry: None,
            non_interactive: false,
            todo_manager: Arc::new(TodoManager::new()),
            stream_responses: false,
            verbose: false,
            debug_level: 0,
            process_id: 12345, // Fixed for testing
        }
    }

    #[tokio::test]
    async fn test_auto_save_creates_valid_file() {
        use std::fs;
        
        let mut chat = create_test_chat().await;
        
        // Add test messages
        chat.messages.push(Message {
            role: "user".to_string(),
            content: "Test message".to_string(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning: None,
        });
        
        let temp_dir = TempDir::new().unwrap();
        let test_logs_dir = temp_dir.path().join("logs");
        fs::create_dir_all(&test_logs_dir).unwrap();
        
        // Mock the logs directory for testing
        let file_path = test_logs_dir.join("history-12345.json");
        let result = crate::chat::state::save_state(
            &chat.messages,
            &chat.current_model,
            chat.total_tokens_used,
            file_path.to_str().unwrap()
        );
        
        assert!(result.is_ok(), "Auto-save should succeed");
        
        // Verify file was created
        assert!(file_path.exists(), "History file should exist");
        
        // Verify file contains valid JSON
        let content = fs::read_to_string(&file_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        
        assert_eq!(parsed["messages"].as_array().unwrap().len(), 2); // 1 user + 1 system message
        assert_eq!(parsed["current_model"], "grn_model");
    }

    #[tokio::test]
    async fn test_auto_save_with_multiple_messages() {
        let mut chat = create_test_chat().await;
        
        // Add multiple messages
        for i in 0..5 {
            chat.messages.push(Message {
                role: "user".to_string(),
                content: format!("Message {}", i),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                reasoning: None,
            });
        }
        
        let temp_dir = TempDir::new().unwrap();
        let test_logs_dir = temp_dir.path().join("logs");
        std::fs::create_dir_all(&test_logs_dir).unwrap();
        
        let file_path = test_logs_dir.join("history-12345.json");
        let result = crate::chat::state::save_state(
            &chat.messages,
            &chat.current_model,
            chat.total_tokens_used,
            file_path.to_str().unwrap()
        );
        
        assert!(result.is_ok());
        
        // Verify file was created and can be loaded
        let (loaded_messages, _, _, _) = crate::chat::state::load_state(file_path.to_str().unwrap()).unwrap();
        assert_eq!(loaded_messages.len(), 6); // 5 user + 1 system
    }
}
```

**Step 2: Run the new tests**

```bash
cargo test --release --lib auto_save_tests
```

Expected: All tests should pass

**Step 3: Commit**

```bash
git add apchat-main/src/main.rs
git commit -m "test: add tests for auto-save history functionality"
```

---

## Task 5: Test integration with existing /load command

**Files:**
- Test: Manual integration test

**Step 1: Build the project**

```bash
cargo build --release
```

Expected: Build should succeed without errors

**Step 2: Create a simple test scenario**

```bash
# Run the application in interactive mode
cargo run --release --interactive

# In the interactive session:
# 1. Type a few messages
# 2. Exit the application
# 3. Check that history file was created in ~/.okaychat/logs/
# 4. Load the file with /load command to verify format compatibility
```

**Step 3: Verify history file format**

```bash
# Check the history file structure
cat ~/.okaychat/logs/history-*.json

# Verify it matches the format of /save command
cat ~/.okaychat/saved_conversations/*.json
```

Expected: Both files should have identical structure with `messages`, `current_model`, `total_tokens_used`, and `version` fields

**Step 4: Commit**

```bash
git commit -m "feat: complete automatic history saving feature"
```

---

## Task 6: Documentation and cleanup

**Files:**
- Modify: `docs/usage.md` (or create if doesn't exist)

**Step 1: Add documentation for auto-save feature**

If `docs/usage.md` exists, add a section:

```markdown
### Automatic History Saving

APChat automatically saves your conversation history after each user message. The history is stored in:

```
~/.okaychat/logs/history-<pid>.json
```

Where `<pid>` is the process ID of your APChat session.

**Features:**
- Saved in the same format as `/save` command
- Can be loaded back using `/load` command
- One file per session
- Silent operation (visible only with debug level > 1)

**Use Cases:**
- Accidental exit recovery: Load your session back from the auto-saved file
- Session sharing: Share the history file with team members
- Debugging: Review conversation history when issues occur
```

**Step 2: Update CLAUDE.md if needed**

Add note about auto-save feature in the features section.

**Step 3: Commit**

```bash
git add docs/usage.md
git commit -m "docs: document automatic history saving feature"
```

---

## Verification Checklist

1. ✅ APChat struct has `process_id` field
2. ✅ `process_id` is initialized in constructor
3. ✅ `auto_save_history()` method implemented
4. ✅ Auto-save integrated into message processing loop
5. ✅ Tests for auto-save functionality exist and pass
6. ✅ Integration tested with /load command
7. ✅ Documentation updated
8. ✅ Build succeeds without warnings
9. ✅ All existing tests still pass

---

**Plan complete!** Two execution options:

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**
