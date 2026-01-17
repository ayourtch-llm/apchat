# Date Injection in System Prompt - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Inject the current date into the system prompt of the LLM to provide temporal context to AI assistants

**Architecture:** Modify the `get_system_prompt` function to append the current date (format: "YYYY-MM-DD") to the base system prompt. This change will automatically propagate through the agent system since the prompt is built dynamically each time an agent is created.

**Tech Stack:** Rust, Chrono crate (already in dependencies)

---

### Task 1: Add chrono dependency to apchat-main

**Files:**
- Modify: `apchat-main/Cargo.toml`

**Step 1: Add chrono dependency**

Edit the `Cargo.toml` file to add the chrono crate:

```toml
[dependencies]
# ... existing dependencies ...
chrono = "0.4"
```

**Step 2: Verify dependency exists**

Run: `cd apchat-main && cargo check`
Expected: Build succeeds with chrono crate available

**Step 3: Commit**

```bash
cd apchat-main
git add Cargo.toml
git commit -m "feat: add chrono dependency for date handling"
```

---

### Task 2: Modify get_system_prompt to include current date

**Files:**
- Modify: `apchat-main/src/config/helpers.rs:13-90`

**Step 1: Update imports**

Add chrono import at the top of the file:

```rust
use chrono::Local;
```

**Step 2: Modify get_system_prompt function**

Update the function signature and add date injection:

```rust
pub fn get_system_prompt(
    client_config: &crate::config::ClientConfig,
    skill_registry: Option<&std::sync::Arc<apchat_skills::SkillRegistry>>,
    early_superpowers: bool,
) -> String {
    // Get current date in YYYY-MM-DD format
    let current_date = Local::now().format("%Y-%m-%d").to_string();

    // Helper function to get model name for a color
    fn get_model_name_for_color(color: ModelColor, config: &crate::config::ClientConfig) -> String {
        if let Some(override_model) = config.get_model_override(color) {
            override_model.to_string()
        } else {
            let provider = config.get_provider(color);
            provider.model_name.clone()
        }
    }

    let grn_model_name = get_model_name_for_color(ModelColor::GrnModel, client_config);
    let blu_model_name = get_model_name_for_color(ModelColor::BluModel, client_config);
    let red_model_name = get_model_name_for_color(ModelColor::RedModel, client_config);

    let mut base_prompt = format!("You are an AI assistant with access to file operations and model switching capabilities. \
    The system supports multiple models that can be switched during the conversation:\n\
    - GrnModel ({}): **Preferred for cost efficiency** - significantly cheaper than BluModel while providing good performance for most tasks\n\
    - BluModel ({}): Use when GrnModel struggles or when you need faster responses\n\
    - RedModel ({}): Use for specialized tasks requiring different capabilities\n\n\
    IMPORTANT: You have been provided with a set of tools (functions) that you can use. \
    Only use the tools that are provided to you - do not make up tool names or attempt to use tools that are not available. \
    When making multiple file edits, use plan_edits to create a complete plan, then apply_edit_plan to execute all changes atomically. \
    This prevents issues where you lose track of file state between sequential edits.\n\n\
    Model switches may happen automatically during the conversation based on tool usage and errors. \
    The currently active model will be indicated in system messages as the conversation progresses.\n\
    
    📅 Current Date: {}",
    grn_model_name, blu_model_name, red_model_name, current_date);
```

**Step 3: Run tests to verify changes**

Run: `cd apchat-main && cargo test --lib`
Expected: All tests pass, chrono is properly integrated

**Step 4: Commit**

```bash
cd apchat-main
git add src/config/helpers.rs
git commit -m "feat: inject current date into system prompt"
```

---

### Task 3: Create unit test for date injection

**Files:**
- Create: `apchat-main/src/config/system_prompt_tests.rs`

**Step 1: Create new test file**

Create a new file `apchat-main/src/config/system_prompt_tests.rs`:

```rust
#[cfg(test)]
mod system_prompt_tests {
    use super::super::config::helpers::get_system_prompt;
    use super::super::config::ClientConfig;
    use std::sync::Arc;

    #[test]
    fn test_system_prompt_contains_current_date() {
        let client_config = ClientConfig::default();
        let system_prompt = get_system_prompt(&client_config, None, false);
        
        // Check that the prompt contains a date in YYYY-MM-DD format
        assert!(system_prompt.contains("📅 Current Date: "));
        
        // Extract the date portion and verify format
        let date_start = system_prompt.find("📅 Current Date: ").unwrap();
        let date_str = &system_prompt[date_start + "📅 Current Date: ".len()..];
        let end_of_line = date_str.find('\n').unwrap_or(date_str.len());
        let date_only = &date_str[..end_of_line];
        
        // Verify it matches YYYY-MM-DD format
        assert!(date_only.len() == 10);
        assert!(date_only.chars().nth(4) == Some('-'));
        assert!(date_only.chars().nth(7) == Some('-'));
    }

    #[test]
    fn test_system_prompt_date_updates() {
        let client_config = ClientConfig::default();
        
        // Get prompt twice with small delay to ensure date could potentially change
        let prompt1 = get_system_prompt(&client_config, None, false);
        
        // Extract first date
        let date1_start = prompt1.find("📅 Current Date: ").unwrap();
        let date1_str = &prompt1[date1_start + "📅 Current Date: ".len..];
        let date1_end = date1_str.find('\n').unwrap_or(date1_str.len());
        let date1 = &date1_str[..date1_end];
        
        // Get second prompt
        let prompt2 = get_system_prompt(&client_config, None, false);
        
        // Extract second date
        let date2_start = prompt2.find("📅 Current Date: ").unwrap();
        let date2_str = &prompt2[date2_start + "📅 Current Date: ".len..];
        let date2_end = date2_str.find('\n').unwrap_or(date2_str.len());
        let date2 = &date2_str[..date2_end];
        
        // Both should be valid dates (they might be the same if called quickly)
        assert!(date1.len() == 10);
        assert!(date2.len() == 10);
    }
}
```

**Step 2: Register test module**

Update `apchat-main/src/config/mod.rs` to include the test module:

```rust
#[cfg(test)]
mod system_prompt_tests;
```

**Step 3: Run new tests**

Run: `cd apchat-main && cargo test system_prompt_tests`
Expected: Both tests pass

**Step 4: Commit**

```bash
cd apchat-main
git add src/config/system_prompt_tests.rs src/config/mod.rs
git commit -m "test: add unit tests for date injection in system prompt"
```

---

### Task 4: Integration test - verify date appears in agent system prompt

**Files:**
- Modify: `crates/apchat-agents/src/agent_tests.rs`

**Step 1: Add test to verify date in agent prompt**

Add this test to the existing agent tests:

```rust
#[tokio::test]
async fn test_agent_system_prompt_contains_date() {
    use apchat_config::ClientConfig;
    
    let config = ClientConfig::default();
    let system_prompt = apchat_config::get_system_prompt(&config, None, false);
    
    // Verify date is present
    assert!(system_prompt.contains("📅 Current Date: "));
    
    // Extract and validate date format
    let date_start = system_prompt.find("📅 Current Date: ").unwrap();
    let date_str = &system_prompt[date_start + "📅 Current Date: ".len()..];
    let end_of_line = date_str.find('\n').unwrap_or(date_str.len());
    let date_only = &date_str[..end_of_line];
    
    // Verify YYYY-MM-DD format
    let parts: Vec<&str> = date_only.split('-').collect();
    assert_eq!(parts.len(), 3);
    
    // Verify each part is numeric
    assert!(parts[0].parse::<u32>().is_ok());
    assert!(parts[1].parse::<u32>().is_ok());
    assert!(parts[2].parse::<u32>().is_ok());
}
```

**Step 2: Run integration test**

Run: `cd crates/apchat-agents && cargo test test_agent_system_prompt_contains_date`
Expected: Test passes

**Step 3: Commit**

```bash
cd crates/apchat-agents
git add src/agent_tests.rs
git commit -m "test: add integration test for date in agent system prompt"
```

---

### Task 5: End-to-end verification

**Files:**
- Run: Full build and basic functionality test

**Step 1: Build entire project**

Run: `cargo build --release`
Expected: Build succeeds without errors

**Step 2: Run application in preview mode**

Run: `cargo run --release -- --preview`
Expected: Application starts, no errors about chrono or date formatting

**Step 3: Verify date appears in actual system prompt**

Create a simple test script or manually verify by checking the system prompt content in the output

**Step 4: Commit final changes**

```bash
git add .
git commit -m "feat: complete date injection implementation with tests"
```

---

## Verification Checklist

- [ ] Chrono dependency added to apchat-main/Cargo.toml
- [ ] Import added to helpers.rs
- [ ] Date injection logic implemented in get_system_prompt
- [ ] Unit tests created and passing
- [ ] Integration tests created and passing
- [ ] Full build succeeds
- [ ] Application runs without errors
- [ ] Date appears in system prompt output

## Expected Behavior

When an agent is created, the system prompt will include a line like:
```
📅 Current Date: 2026-01-17
```

This provides the LLM with temporal context about when the conversation is taking place, which can be useful for:
- Date-sensitive operations
- Temporal reasoning
- Context-aware responses
- Debugging and logging

The date updates automatically each time the system prompt is generated, ensuring agents always have the current date.
