# Issue 129: Update llm_task.rs to use stateless API functions

## Summary
Modify `llm_task.rs` to use the new stateless API functions instead of calling `LlmClient` directly.

## Location
- File: `apchat-main/src/app/repl/llm_task.rs`

## Current Behavior
`llm_task.rs` calls `llm_client.chat()` and `llm_client.chat_streaming()` directly, missing all the API-level logic (message conversion, XML tool parsing, retry logic, logging).

## Expected Behavior
`llm_task.rs` uses `call_api_stateless()` and `call_api_streaming_stateless()` which have all the battle-tested API logic.

## Impact
Tool calls will work correctly, streaming will work correctly, all edge cases handled.

## Suggested Implementation

1. Update `LLMRequest` to carry `ApiCallParams` instead of individual fields:
```rust
pub struct LLMRequest {
    pub params: ApiCallParams,
    pub cancel_token: CancellationToken,
    pub stream_sender: Option<mpsc::Sender<OutputChunk>>,
}
```

2. Update `make_non_streaming_call`:
```rust
async fn make_non_streaming_call(request: &LLMRequest) -> LLMResponse {
    match crate::api::call_api_stateless(&request.params).await {
        Ok((message, usage, model)) => {
            LLMResponse::Success {
                content: message.content,
                usage,
                tool_calls: message.tool_calls,
                model,
            }
        }
        Err(e) => LLMResponse::Error(e.to_string()),
    }
}
```

3. Update `make_streaming_call`:
```rust
async fn make_streaming_call(request: &LLMRequest) -> LLMResponse {
    match crate::api::call_api_streaming_stateless(
        &request.params,
        request.stream_sender.clone(),
    ).await {
        Ok((message, usage, model, _metrics)) => {
            LLMResponse::Success {
                content: message.content,
                usage,
                tool_calls: message.tool_calls,
                model,
            }
        }
        Err(e) => LLMResponse::Error(e.to_string()),
    }
}
```

## Verification
```bash
cargo build -p apchat
```

---
*Created: 2025-02-02*
