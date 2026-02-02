# Issue 126: Create ApiCallParams struct

## Summary
Extract the parameters needed for API calls from APChat into a standalone struct that can be passed to stateless API functions.

## Location
- File: `apchat-main/src/api/mod.rs` (new struct)

## Current Behavior
API functions like `call_api()` and `call_api_streaming()` take `&APChat` directly, making them tightly coupled to the APChat state.

## Expected Behavior
A new `ApiCallParams` struct holds all data needed for an API call, enabling stateless API functions.

## Impact
This is the foundation for the REPL refactor - enables the LLM task to be truly stateless.

## Suggested Implementation

Add to `apchat-main/src/api/mod.rs`:

```rust
use std::sync::Arc;
use apchat_models::{ModelColor, Message, Tool};
use apchat_llm_api::client::LlmClient;
use crate::config::ClientConfig;

/// Parameters needed for a stateless API call
#[derive(Clone)]
pub struct ApiCallParams {
    pub messages: Vec<Message>,
    pub current_model: ModelColor,
    pub client_config: ClientConfig,
    pub api_key: String,
    pub tools: Vec<Tool>,
    pub stream_responses: bool,
    pub verbose: bool,
    pub debug_level: u32,
    pub http_client: reqwest::Client,
}

impl ApiCallParams {
    /// Create from APChat (convenience method during migration)
    pub fn from_apchat(chat: &crate::APChat) -> Self {
        Self {
            messages: chat.messages.clone(),
            current_model: chat.current_model.clone(),
            client_config: chat.client_config.clone(),
            api_key: chat.api_key.clone(),
            tools: chat.get_tools(),
            stream_responses: chat.stream_responses,
            verbose: chat.verbose,
            debug_level: chat.debug_level,
            http_client: chat.client.clone(),
        }
    }

    pub fn should_show_debug(&self, level: u32) -> bool {
        self.debug_level & (1 << (level - 1)) != 0
    }
}
```

Also export it: `pub use self::ApiCallParams;`

## Verification
```bash
cargo build -p apchat
```

---
*Created: 2025-02-02*
