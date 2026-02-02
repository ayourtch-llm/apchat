# REPL Refactor Design

## Overview

Refactor the REPL into two separate tasks with clean separation of concerns:
- REPL task handles MSPC messaging, state, and orchestration
- LLM task processes LLM requests independently and is interruptible without races

## Architecture

### REPL Task (Orchestrator)

**Responsibilities:**
- Runs the main message loop on MSPC channel
- Owns APChat state (messages, history, tools registry, everything)
- Handles tool-calling loop after receiving LLM responses
- Executes tools with current messages/turns context
- Adds/removes messages
- Manages conversation flow
- Sends LLM requests to dedicated LLM service
- Receives Ctrl-C interrupts and forwards to LLM task

### LLM Task (Stateless Service)

**Responsibilities:**
- Runs as a single long-lived tokio task
- Continuously monitors request queue
- Processes each request by making ONE LLM API call
- Returns raw response (content, usage, tool_calls)
- Can be cleanly interrupted (Ctrl-C terminates API call)
- NO access to APChat state - completely stateless

## Channels

### REPL → LLM Task
```rust
struct LLMRequest {
    messages: Vec<Message>,
    model: ModelColor,
    client_config: ClientConfig,
    api_key: String,
    // Cancel token REPL sets before sending request
    cancel_token: tokio_util::sync::CancellationToken,
    // Optional: for streaming mode, sends incremental chunks
    stream_sender: Option<tokio::sync::mpsc::Sender<StreamChunk>>,
}

// Buffer of 1: only one request in flight at a time
let (llm_request_sender, llm_request_receiver) = tokio::sync::mpsc::channel::<LLMRequest>(1);
```

### LLM Task → REPL
```rust
enum LLMResponse {
    Success {
        content: String,
        usage: Option<Usage>,
        tool_calls: Option<Vec<ToolCall>>,
    },
    Interrupted,
    Error(String),
}

// Buffer of 1: one response per request
let (llm_response_sender, llm_response_receiver) = tokio::sync::mpsc::channel::<LLMResponse>(1);
```

## Data Flow

### Normal Conversation
1. User input → REPL (MSPC `UserInput`)
2. REPL adds user message to APChat.messages
3. REPL creates `LLMRequest` with fresh cancel_token
4. REPL sends to LLM task via `llm_request_sender`
5. LLM task makes ONE API call (interruptible via token)
6. LLM task returns `LLMResponse`
7. REPL matches on `LLMResponse`:
   - `Success { tool_calls: Some(_), .. }`: execute tools, add results to messages, loop back to step 3
   - `Success { tool_calls: None, .. }`: add assistant response, done
   - `Interrupted`: add "[Interrupted]" to history, done
   - `Error(e)`: display error, may retry or prompt user
8. REPL displays response, broadcasts to Webex

### Interrupt Flow
1. Terminal Ctrl-C → Input Router → MSPC `InterruptSignal`
2. REPL receives interrupt signal
3. REPL calls `cancel_token.cancel()`
4. LLM task detects token cancelled, aborts API call, returns `LLMResponse::Interrupted`
5. REPL adds "[Interrupted]" message to history
6. REPL continues waiting for next input

## Key Benefits

- **No races:** LLM API call runs independently in LLM task, not raced in tokio::select
- **Clean interrupt:** Ctrl-C cancels at task level via token, not by racing control flow
- **Stateless LLM:** LLM task has no access to APChat, simpler and more testable
- **Clear responsibilities:** REPL owns all state; LLM task is a dumb service
- **Independent processing:** MSPC messages of any kind don't interfere with LLM calls

## Streaming Support

Streaming requires incremental delivery of tokens to the user while the LLM is generating.

### Streaming Channel
```rust
// Optional channel for streaming chunks (created per-request when streaming enabled)
struct StreamChunk {
    delta: String,      // Incremental text
    is_final: bool,     // True when stream complete
}

let (stream_sender, stream_receiver) = tokio::sync::mpsc::channel::<StreamChunk>(32);
```

### Streaming Flow
1. REPL creates `stream_receiver`, passes `stream_sender` in `LLMRequest`
2. LLM task sends `StreamChunk` for each token received from API
3. REPL renders chunks to terminal as they arrive
4. On `is_final: true`, REPL stops listening and processes complete response
5. Final `LLMResponse::Success` contains the full assembled content

### Non-Streaming Mode
- `stream_sender` field is `None` in `LLMRequest`
- LLM task waits for complete response, sends single `LLMResponse`

## Timeout vs Interrupt

These are distinct failure modes with different semantics:

| Scenario | Trigger | LLMResponse | REPL Action |
|----------|---------|-------------|-------------|
| **Interrupt** | User Ctrl-C | `Interrupted` | Add "[Interrupted]" to history, ready for next input |
| **Timeout** | No response within deadline | `Error("timeout")` | Display error, may retry or prompt user |
| **API Error** | Network/rate limit/etc | `Error(details)` | Display error, may retry with backoff |

The REPL sets a timeout when waiting on `llm_response_receiver`. This is separate from the `CancellationToken` which is only triggered by user interrupt.

## Implementation Notes

- Extract LLM API call logic from `chat::session::chat()` into separate function
- LLM task runs one `tokio::select!` on request channel and cancellation token
- REPL sends request, then waits on response channel with timeout
- Tool-calling loop stays in REPL (moved out of `chat()`)
- Streaming chunks flow through separate channel to avoid blocking response channel