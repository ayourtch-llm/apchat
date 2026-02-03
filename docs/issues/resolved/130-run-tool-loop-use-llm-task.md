# Issue 130: Update run_tool_loop to properly use LLM task

## Summary
Restore the full `run_tool_loop()` implementation that uses the LLM task channels, now that the LLM task uses the correct API functions.

## Resolution Status: ✅ RESOLVED

## Implementation Details

### File: `apchat-main/src/app/repl.rs` (Lines 327-574)

**Key Changes:**

1. **Spawn LLM Task**:
   ```rust
   let mut llm_channels = spawn_llm_task();
   ```
   - Creates long-lived tokio task for stateless API calls
   - Returns request/response channels

2. **Tool Loop with LLM Task Integration**:
   - Prepare user message and history summarization (lines 343-352)
   - Main loop with cancellation check (line 357)
   - Validate tool calls before API call (lines 371-376)
   - **Send request to LLM task** (lines 379-393):
     ```rust
     let request = LLMRequest {
         params: ApiCallParams { ... },
         cancel_token: cancel_token.clone(),
         stream_sender: Some(tx), // For streaming output
     };
     llm_channels.request_tx.send(request).await
     ```
   - **Receive response from LLM task** (lines 395-407):
     ```rust
     let llm_response = llm_channels.response_rx.recv().await
     ```

3. **Handle Streaming Chunks** (lines 382-390):
   - Create tokio channel for streaming chunks
   - Spawn task to receive and print chunks in real-time
   - Pass sender to LLM task via `LLMRequest`

4. **Process LLMResponse** (lines 408-574):
   - `Success`: Handle content, usage, model updates, tool execution
   - `Interrupted`: Return interrupted status
   - `Error`: Return error status

5. **Tool Execution** (lines 453-468):
   - Extracts tool calls from response
   - Increments iteration counter
   - Detects infinite loops using signature window
   - Calls `chat.execute_tool()` for each tool
   - Adds assistant and tool response messages to history

## Architectural Benefits

1. **Cleaner Separation**: LLM task handles all API logic, `run_tool_loop` handles orchestration
2. **Cancellability**: Proper integration with ctrl-c via `cancel_token`
3. **Streaming Support**: Real-time token streaming through channel-based design
4. **Loop Detection**: Simple but effective infinite tool loop detection
5. **Statelessness**: LLM task has no access to APChat state, only request params

## Test Coverage

The tool loop functionality is tested through:
- Integration tests with the REPL
- Manual testing with `cargo run -- --model ... --stream -i`

## Verification Commands

```bash
# Build
cargo build

# Run tests (known pre-existing doctest issues in apchat-vty)
cargo test

# Run interactive REPL to verify
cargo run -- --model 'QuantTrio/GLM-4.7-AWQ@llama(http://192.168.0.121:8000)' --stream -i --auto-confirm
```

## Related Issues

- Issue 128: Stateless streaming API (prerequisite)
- Issue 129: LLM task uses stateless API (prerequisite)
- Issue 131: Clean up session.rs after REPL refactor