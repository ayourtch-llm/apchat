# Issue 127: Create stateless non-streaming API function

## Summary
Create `call_api_stateless()` that takes `ApiCallParams` instead of `&APChat`.

## Location
- File: `apchat-main/src/api/client.rs`

## Current Behavior
`call_api()` takes `&APChat` and extracts parameters from it.

## Expected Behavior
New `call_api_stateless()` takes `&ApiCallParams`. Existing `call_api()` creates params and delegates to it.

## Impact
Enables the LLM task to make non-streaming API calls without APChat access.

## Suggested Implementation

In `apchat-main/src/api/client.rs`:

1. Add import: `use super::ApiCallParams;`

2. Rename existing `call_api` to `call_api_stateless` and change signature:
```rust
pub(crate) async fn call_api_stateless(
    params: &ApiCallParams,
) -> Result<(Message, Option<Usage>, ModelColor)> {
    // Same logic but use params.current_model, params.client_config, etc.
    // Replace chat.X with params.X throughout
}
```

3. Create new `call_api` that delegates:
```rust
pub(crate) async fn call_api(
    chat: &APChat,
    orig_messages: &[Message],
) -> Result<(Message, Option<Usage>, ModelColor)> {
    let mut params = ApiCallParams::from_apchat(chat);
    params.messages = orig_messages.to_vec();
    call_api_stateless(&params).await
}
```

4. Similarly update `call_api_with_llm_client` to `call_api_with_llm_client_stateless`.

## Verification
```bash
cargo build -p apchat
cargo test -p apchat
```

---
*Created: 2025-02-02*
