# Refactor Hardcoded Model Identifiers Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace hardcoded model identifiers (like "moonshotai/kimi-k2-instruct-0905") with configurable models from the current config system, enabling proper model overrides.

**Architecture:** 
- Create a new method `get_model_identifier()` that uses the current chat config instead of hardcoded values
- Add a parameter to `as_str()` methods to access current config context
- Use compilation errors as a guide to find all locations needing updates
- Maintain backward compatibility for uses that don't have config context

**Tech Stack:** Rust, kimichat project structure, model configuration system

---

## Analysis Phase

### Current Hardcoded Models Found:
- `ModelType::BluModel.as_str()` → "moonshotai/kimi-k2-instruct-0905"
- `ModelType::GrnModel.as_str()` → "openai/gpt-oss-120b"  
- `ModelType::RedModel.as_str()` → "meta-llama/llama-3.1-70b-versatile"
- `ModelType::AnthropicModel.as_str()` → "claude-3-5-sonnet-20241022"

### Key Files Using `as_str()`:
- `kimichat-main/src/api/client.rs` - API requests
- `kimichat-main/src/api/streaming.rs` - Streaming requests  
- `kimichat-main/src/chat/history.rs` - Summarization requests
- `kimichat-main/src/config/mod.rs` - Model registration
- `kimichat-main/src/config/helpers.rs` - Helper functions

---

## Task 1: Rename Current `as_str()` Method

**Files:**
- Modify: `crates/kimichat-models/src/types.rs:14`

**Step 1: Rename the method**

```rust
// Change from:
pub fn as_str(&self) -> String {

// To:
pub fn as_str_default(&self) -> String {
```

**Step 2: Add new method signature for config-aware version**

```rust
pub fn as_str(&self, client_config: Option<&crate::config::ClientConfig>) -> String {
```

**Step 3: Run compilation to find all breakage**

Run: `cargo check`
Expected: Multiple compilation errors where `.as_str()` is called

**Step 4: Commit**

```bash
git add crates/kimichat-models/src/types.rs
git commit -m "refactor: rename as_str to as_str_default and add config-aware signature"
```

---

## Task 2: Implement Config-Aware Model Resolution

**Files:**
- Modify: `crates/kimichat-models/src/types.rs:15-25`

**Step 1: Add imports for config types**

```rust
use crate::config::ClientConfig;  // Add this at top
```

**Step 2: Implement the new config-aware method**

```rust
pub fn as_str(&self, client_config: Option<&crate::config::ClientConfig>) -> String {
    if let Some(config) = client_config {
        match self {
            ModelType::BluModel => config.model_blu_model_override
                .clone()
                .unwrap_or_else(|| self.as_str_default()),
            ModelType::GrnModel => config.model_grn_model_override
                .clone()
                .unwrap_or_else(|| self.as_str_default()),
            ModelType::RedModel => config.model_red_model_override
                .clone()
                .unwrap_or_else(|| self.as_str_default()),
            ModelType::AnthropicModel => config.model_anthropic_model_override
                .clone()
                .unwrap_or_else(|| self.as_str_default()),
            ModelType::Custom(name) => name.clone(),
        }
    } else {
        self.as_str_default()
    }
}
```

**Step 3: Run compilation to verify**

Run: `cargo check`
Expected: Still compilation errors from renamed method calls

**Step 4: Commit**

```bash
git add crates/kimichat-models/src/types.rs
git commit -m "feat: implement config-aware model identifier resolution"
```

---

## Task 3: Fix Chat Request Creation Points

**Files:**
- Modify: `kimichat-main/src/api/client.rs:59`
- Modify: `kimichat-main/src/api/streaming.rs:31`
- Modify: `kimichat-main/src/chat/history.rs:186, 362, 484`

**Step 1: Fix API client request**

```rust
// Change from:
model: current_model.as_str().to_string(),

// To:
model: current_model.as_str(Some(&chat.client_config)).to_string(),
```

**Step 2: Fix streaming request**

```rust
// Change from:
model: current_model.as_str().to_string(),

// To:
model: current_model.as_str(Some(&chat.client_config)).to_string(),
```

**Step 3: Fix history summarization requests**

```rust
// Change from:
model: current_model.as_str().to_string(),

// To:
model: current_model.as_str(Some(&chat.client_config)).to_string(),
```

**Step 4: Run compilation**

Run: `cargo check`
Expected: Fewer compilation errors

**Step 5: Commit**

```bash
git add kimichat-main/src/api/client.rs kimichat-main/src/api/streaming.rs kimichat-main/src/chat/history.rs
git commit -m "fix: use config-aware model resolution in chat requests"
```

---

## Task 4: Fix Config Registration

**Files:**
- Modify: `kimichat-main/src/config/mod.rs:112-116`
- Modify: `kimichat-main/src/config/helpers.rs:251-254`

**Step 1: Fix model registration in mod.rs**

```rust
// Change from:
let blu_model = client_config.model_blu_model_override.clone()
    .unwrap_or_else(|| ModelType::BluModel.as_str());
let grn_model = client_config.model_grn_model_override.clone()
    .unwrap_or_else(|| ModelType::GrnModel.as_str());
let red_model = client_config.model_red_model_override.clone()
    .unwrap_or_else(|| ModelType::RedModel.as_str());

// To:
let blu_model = ModelType::BluModel.as_str(Some(client_config));
let grn_model = ModelType::GrnModel.as_str(Some(client_config));
let red_model = ModelType::RedModel.as_str(Some(client_config));
```

**Step 2: Fix helper function model selection**

```rust
// Change from:
"blu" => ModelType::BluModel.as_str(),
"grn" => ModelType::GrnModel.as_str(),
"red" => ModelType::RedModel.as_str(),
_ => ModelType::GrnModel.as_str(),

// To:
"blu" => ModelType::BluModel.as_str(client_config),
"grn" => ModelType::GrnModel.as_str(client_config),
"red" => ModelType::RedModel.as_str(client_config),
_ => ModelType::GrnModel.as_str(client_config),
```

**Step 3: Run compilation**

Run: `cargo check`
Expected: Still some compilation errors from logging/tool validation

**Step 4: Commit**

```bash
git add kimichat-main/src/config/mod.rs kimichat-main/src/config/helpers.rs
git commit -m "fix: use config-aware model resolution in config registration"
```

---

## Task 5: Fix Logging and Validation

**Files:**
- Modify: `crates/kimichat-logging/src/request_logger.rs:65, 73`
- Modify: `crates/kimichat-tools/src/model_management.rs:61`
- Modify: `crates/kimichat-agents/src/agent_config.rs:64`

**Step 1: Fix request logging**

```rust
// For logging, use default method since config may not be available:
let model_name = model.as_str_default().replace('/', "-");
// and:
log_content.push_str(&format!("Model: {}\n\n", model.as_str_default()));
```

**Step 2: Fix model validation (keep using string checks)**

```rust
// Keep these as string comparisons for validation
if !["kimi", "gpt_oss", "blu_model", "grn_model", "anthropic"].contains(&model.as_str()) {
```

**Step 3: Run compilation**

Run: `cargo check`
Expected: Should compile successfully now

**Step 4: Commit**

```bash
git add crates/kimichat-logging/src/request_logger.rs crates/kimichat-tools/src/model_management.rs crates/kimichat-agents/src/agent_config.rs
git commit -m "fix: update logging and validation to work with new model methods"
```

---

## Task 6: Add Tests

**Files:**
- Create: `crates/kimichat-models/src/tests/model_resolution_tests.rs`

**Step 1: Create test file**

```rust
#[cfg(test)]
mod model_resolution_tests {
    use super::*;
    use crate::config::ClientConfig;

    #[test]
    fn test_default_model_resolution() {
        assert_eq!(ModelType::BluModel.as_str_default(), "moonshotai/kimi-k2-instruct-0905");
        assert_eq!(ModelType::GrnModel.as_str_default(), "openai/gpt-oss-120b");
    }

    #[test]
    fn test_config_aware_resolution() {
        let mut config = ClientConfig::default();
        config.model_blu_model_override = Some("custom/blu-model".to_string());

        assert_eq!(
            ModelType::BluModel.as_str(Some(&config)),
            "custom/blu-model"
        );
        assert_eq!(
            ModelType::GrnModel.as_str(Some(&config)),
            "openai/gpt-oss-120b"  // falls back to default
        );
    }

    #[test]
    fn test_no_config_fallback() {
        assert_eq!(
            ModelType::BluModel.as_str(None),
            "moonshotai/kimi-k2-instruct-0905"
        );
    }

    #[test]
    fn test_custom_model() {
        let custom_model = ModelType::Custom("my-custom-model".to_string());
        assert_eq!(custom_model.as_str(None), "my-custom-model");
        assert_eq!(custom_model.as_str(Some(&ClientConfig::default())), "my-custom-model");
    }
}
```

**Step 2: Add mod declaration to lib.rs**

```rust
#[cfg(test)]
mod tests;
```

**Step 3: Run tests**

Run: `cargo test model_resolution_tests`
Expected: All tests pass

**Step 4: Commit**

```bash
git add crates/kimichat-models/src/tests/ crates/kimichat-models/src/lib.rs
git commit -m "test: add model resolution tests"
```

---

## Task 7: Integration Test

**Files:**
- Test: Full integration test with model overrides

**Step 1: Test with actual config**

```bash
# Test with custom model override
KIMICHAT_MODEL_BLU_MODEL_OVERRIDE="custom/test-blu" cargo run --bin kimichat -- --help

# Should not crash and should use custom model when BluModel is selected
```

**Step 2: Verify compilation still works**

Run: `cargo build --release`
Expected: Successful compilation

**Step 3: Test existing functionality**

Run: `cargo test`
Expected: All existing tests still pass

**Step 4: Commit**

```bash
git add .
git commit -m "test: verify integration and backward compatibility"
```

---

## Verification Steps

1. **Compilation**: `cargo check` and `cargo build` should pass
2. **Unit Tests**: All existing tests should continue to pass
3. **New Tests**: Model resolution tests should pass
4. **Functional Test**: Start kimichat with model override flags
5. **Backward Compatibility**: CLI flags and config should still work

## Rollback Plan

If issues arise:
1. Revert `as_str()` method back to original implementation
2. Remove new `as_str_default()` method
3. Restore all call sites to use `.as_str()`
4. All changes are contained within the kimichat-models crate and call sites