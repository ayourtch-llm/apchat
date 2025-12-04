# APChat LLM Color Refactoring Implementation Plan

## Original Task Requirements

Please prepare and write the implementation plan for refactoring to separate the logical llm "color" (blu/grn/red) from provider/model: each color should map to a tuple: (model, backend, api_url, api_key). The key should be configurable as now, the other three elements should be parseable from attings: model@backend(api_url) - or, model@backend (with api url coming from backend default mapping, or simply model, with backend and api url coming from default mapping. Model type enum therefore should have only three "colors", and not talk about providers. After you wrote and committed plan, use subagents to implement it. Also use subagents as much as you can for everything.

## Overview

This document outlines the implementation plan for refactoring the logical LLM "color" (blu/grn/red) from provider/model configuration. The goal is to:

1. Separate the three logical LLM "colors" (blu/grn/red) from provider/backend configuration
2. Each color should map to a tuple: (model, backend, api_url, api_key)
3. The API key should remain configurable as before
4. The other three elements should be parseable from attings: model@backend(api_url) or similar patterns
5. ModelColor enum should have only three "colors" and not mention providers

## Current Architecture

### ModelColor Enum
Located in `crates/apchat-models/src/types.rs`

Current ModelColor enum has:
- `BluModel` (mapped to "moonshotai/kimi-k2-instruct-0905")
- `GrnModel` (mapped to "openai/gpt-oss-120b")
- `RedModel` (mapped to "meta-llama/llama-3.1-70b-versatile")
- `AnthropicModel` (mapped to "claude-3-5-sonnet-20241022")
- `Custom(String)`

### Configuration System

Configuration is handled through `ClientConfig` in `apchat-main/src/config/mod.rs` which has separate fields for:
- backend_blu_model, backend_grn_model, backend_red_model
- api_url_blu_model, api_url_grn_model, api_url_red_model
- api_key_blu_model, api_key_grn_model, api_key_red_model
- model_blu_model_override, model_grn_model_override, model_red_model_override

## Implementation Steps

### Step 1: Simplify ModelColor Enum

Update `crates/apchat-models/src/types.rs`:
- Remove AnthropicModel and Custom variants from core ModelColor
- Keep only BluModel, GrnModel, RedModel
- Preserve as_str_default, display_name, from_string functions

### Step 2: Create Default Configuration Mappings

Update `crates/apchat-llm-api/src/config/mod.rs`:
- Add constants for default model names
- Add constants for default backend types
- Add constants for default API URLs
- Add helper functions for parsing configuration strings

### Step 3: Update Configuration Helper Functions

Update `apchat-main/src/config/helpers.rs`:
- Update get_model_config_from_env() to work with new parsing approach
- Update create_model_client() to use mapping system
- Update create_client_for_model_color() to use mapping system

### Step 4: Update CLI Interface (if needed)

Update `apchat-main/src/cli.rs`:
- Update help text if necessary
- Ensure backward compatibility

### Step 5: Update Main Application

Update `apchat-main/src/main.rs`:
- Ensure initialization works with new structure
- Maintain backward compatibility

## Detailed Changes

### ModelColor Simplification (crates/apchat-models/src/types.rs)

The ModelColor enum will be simplified to:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelColor {
    BluModel,
    GrnModel,
    RedModel,
}
```

And maintain all existing helper methods.

### Configuration Mapping (crates/apchat-llm-api/src/config/mod.rs)

Add mapping system:

```rust
// Default mappings for each model color
pub const DEFAULT_BLU_MODEL: &str = "moonshotai/kimi-k2-instruct-0905";
pub const DEFAULT_GRN_MODEL: &str = "openai/gpt-oss-120b";
pub const DEFAULT_RED_MODEL: &str = "meta-llama/llama-3.1-70b-versatile";

pub const DEFAULT_BLU_BACKEND: BackendType = BackendType::Groq;
pub const DEFAULT_GRN_BACKEND: BackendType = BackendType::Groq;
pub const DEFAULT_RED_BACKEND: BackendType = BackendType::Groq;

pub const DEFAULT_BLU_API_URL: &str = GROQ_API_URL;
pub const DEFAULT_GRN_API_URL: &str = GROQ_API_URL;
pub const DEFAULT_RED_API_URL: &str = GROQ_API_URL;

// Parser for model@backend(api_url) format
pub fn parse_model_attings(atts: &str) -> (String, Option<BackendType>, Option<String>) {
    // Implementation here
}
```

### New Configuration Parsing Logic

The parsing logic should handle:
- "model@backend(api_url)" format
- "model@backend" format (uses default API URL for backend)
- "model" format (uses default backend and API URL)

### Backward Compatibility

Ensure that existing CLI arguments and configuration files continue to work by maintaining the same interface for the client configuration.

## Testing Approach

1. Test that existing functionality still works
2. Test that new parsing format works correctly
3. Verify that all three model types (blu/grn/red) work with their defaults
4. Test custom configurations still work
