# Issue 130: Update run_tool_loop to properly use LLM task

## Summary
Restore the full `run_tool_loop()` implementation that uses the LLM task channels, now that the LLM task uses the correct API functions.

## Location
- File: `apchat-main/src/app/repl.rs`

## Current Behavior
`run_tool_loop()` is a thin wrapper around `session::chat()`, not using the LLM task.

## Expected Behavior
`run_tool_loop()` sends requests to the LLM task, receives responses, handles streaming output display, and executes tools locally.

## Impact
Clean separation between REPL (owns state, runs tool loop) and LLM task (stateless API calls).

## Suggested Implementation

1. Restore LLM task spawning in `run_repl_mode()`:
```rust
let mut llm_channels = llm_task::spawn_llm_task();
```

2. Update `run_tool_loop` signature:
```rust
pub(crate) async fn run_tool_loop(
    chat: &mut APChat,
    input: &str,
    cancel_token: &tokio_util::sync::CancellationToken,
    llm_channels: &mut llm_task::LLMTaskChannels,
) -> InferenceOutcome
```

3. In the loop:
   a. Build `ApiCallParams::from_apchat(chat)`
   b. Create streaming channel if needed
   c. Send `LLMRequest` with params to LLM task
   d. Receive streaming chunks and display them
   e. Receive final `LLMResponse`
   f. If tool_calls: execute tools, add results to chat.messages, continue
   g. If no tools: return response

4. Include existing logic: loop detection, progress evaluation, compaction

## Verification
```bash
cargo build -p apchat
cargo run -- -i  # Test non-streaming
cargo run -- -i --stream  # Test streaming
# Test tool execution (e.g., ask to read a file)
```

---
*Created: 2025-02-02*
