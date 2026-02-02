# Issue 128: Create stateless streaming API function

## Summary
Create `call_api_streaming_stateless()` that takes `ApiCallParams` and an optional channel for streaming output.

## Location
- File: `apchat-main/src/api/streaming.rs`

## Current Behavior
`call_api_streaming()` takes `&APChat` and prints streaming output directly to stdout.

## Expected Behavior
New `call_api_streaming_stateless()` takes `&ApiCallParams` and optionally sends chunks via a channel instead of printing directly.

## Impact
Enables the LLM task to receive streaming chunks and forward them to the REPL for display.

## Suggested Implementation

In `apchat-main/src/api/streaming.rs`:

1. Add to imports:
```rust
use super::ApiCallParams;
use tokio::sync::mpsc;
```

2. Define a simple chunk type (or reuse from llm_task):
```rust
pub struct OutputChunk {
    pub text: String,
    pub is_final: bool,
}
```

3. Create stateless version:
```rust
pub(crate) async fn call_api_streaming_stateless(
    params: &ApiCallParams,
    output_tx: Option<mpsc::Sender<OutputChunk>>,
) -> Result<(Message, Option<Usage>, ModelColor, StreamingMetrics)> {
    // Same logic but:
    // - Use params.X instead of chat.X
    // - When printing chunks, also send to output_tx if Some
    // - If output_tx is None, print directly (backwards compat)
}
```

4. Update existing `call_api_streaming` to delegate:
```rust
pub(crate) async fn call_api_streaming(
    chat: &APChat,
    orig_messages: &[Message],
) -> Result<(Message, Option<Usage>, ModelColor, StreamingMetrics)> {
    let mut params = ApiCallParams::from_apchat(chat);
    params.messages = orig_messages.to_vec();
    call_api_streaming_stateless(&params, None).await
}
```

5. Similarly for `call_api_streaming_with_llm_client`.

## Verification
```bash
cargo build -p apchat
cargo run -- -i --stream  # Test streaming still works
```

---
*Created: 2025-02-02*
