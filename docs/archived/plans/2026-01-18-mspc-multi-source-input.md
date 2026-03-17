# MSPC Multi-Source Input Architecture

**Date**: 2026-01-18
**Status**: Design
**Goal**: Enable multiple I/O sources (terminal, webex, web) to share a single conversation with queued inputs, broadcast outputs, and interruption support.

## Problem Statement

Currently, APChat only supports terminal input/output. We need to:
- Support multiple input sources: terminal, webex bot, web frontend
- Allow all participants to see the same conversation
- Queue messages from all sources (FIFO)
- Support interruption via "!" prefix (clears queue, cancels current LLM)
- Broadcast LLM outputs to all connected destinations
- Maintain clean terminal UX without input clobbering

## High-Level Architecture

```
┌─────────────────────────────────────────────┐
│         Single Shared Session               │
│  - ONE conversation (shared history)        │
│  - ONE MSPC channel (queued inputs)         │
│  - ONE LLM processing loop                  │
└─────────────────────────────────────────────┘
           ▲                    │
           │ (tagged msgs)      ▼ (broadcast)
    ┌──────┴──────┐      ┌──────────────┐
    │  Inputs     │      │  Outputs     │
    ├─────────────┤      ├──────────────┤
    │ Terminal    │      │ Terminal     │
    │ Webex users │      │ Webex users  │
    │ Web clients │      │ Web clients  │
    └─────────────┘      └──────────────┘
```

### Key Principles

1. **Single Shared Conversation**: One session, one conversation history, shared by all participants
2. **Tagged Messages**: Each input includes sender ID for display/tracking
3. **FIFO Queue**: Messages processed in order received (unless interrupted)
4. **Broadcast Outputs**: All destinations receive all messages
5. **Interruption**: "!" prefix clears queue and cancels current operation
6. **Output Abstraction**: Unified interface supports terminal, webex, web, future TUI

## Component Design

### 1. MSPC Message Types

```rust
pub enum MspcMessage {
    UserInput {
        sender: String,        // "terminal", "webex-alice", "websocket-abc123"
        content: String,
    },
    InterruptSignal {
        sender: String,
        content: String,       // Content after "!" prefix
    },
    Command {
        sender: String,
        content: String,       // "/model", "/help", etc.
    },
    ConfirmationRequest {
        prompt: String,
        callback_id: String,
    },
    ConfirmationResponse {
        callback_id: String,
        approved: bool,
    },
}
```

### 2. Input Source Architecture

**Global Input Source Manager** (singleton, owns reader tasks):

```rust
pub struct InputSourceManager {
    terminal_reader: Option<JoinHandle<()>>,
    webex_reader: Option<JoinHandle<()>>,
    websocket_handlers: HashMap<String, JoinHandle<()>>,
}

impl InputSourceManager {
    /// Spawn global terminal reader task (ONE reader for readline)
    pub fn spawn_terminal_reader(&mut self, channel: Arc<MspcChannel>) {
        let handle = tokio::task::spawn_blocking(move || {
            loop {
                match ReadlineInstance::readline(&prompt) {
                    Ok(Some(line)) => {
                        let msg = MspcMessage::UserInput {
                            sender: "terminal".to_string(),
                            content: line,
                        };
                        tokio::runtime::Handle::current()
                            .block_on(channel.send(msg));
                    }
                    Ok(None) => continue,
                    Err(e) if is_eof(&e) => break,
                    Err(e) if is_interrupted(&e) => continue,
                    Err(e) => {
                        eprintln!("Terminal error: {}", e);
                        break;
                    }
                }
            }
        });
        self.terminal_reader = Some(handle);
    }

    /// Spawn webex polling task
    pub fn spawn_webex_reader(&mut self, channel: Arc<MspcChannel>) {
        // Poll webex API, send messages tagged with user ID
    }

    /// Handle new websocket connection
    pub fn add_websocket(&mut self, ws_id: String, channel: Arc<MspcChannel>) {
        // Spawn task to read from websocket, send tagged messages
    }
}
```

**Key Points**:
- ONE terminal reader task (avoids readline conflicts)
- ONE webex polling task (handles all webex users)
- Multiple websocket handlers (one per connection)
- All send to same MSPC channel with sender tags

### 3. Output Destination Architecture

**OutputDestination Trait**:

```rust
#[async_trait]
pub trait OutputDestination: Send + Sync {
    async fn send_output(&self, message: &OutputMessage) -> Result<()>;
    fn dest_id(&self) -> String;
    fn is_active(&self) -> bool;
}

pub enum OutputMessage {
    UserMessage { sender: String, text: String },
    AssistantResponse(String),
    ToolCall { name: String, args: String },
    ToolResult(String),
    SystemMessage(String),
    Error(String),
}
```

**Implementations**:

```rust
// Phase 1: Simple terminal output
pub struct TerminalOutputDestination;

impl OutputDestination for TerminalOutputDestination {
    async fn send_output(&self, message: &OutputMessage) -> Result<()> {
        match message {
            OutputMessage::UserMessage { sender, text } => {
                println!("[{}] > {}", sender.bright_cyan(), text);
            }
            OutputMessage::AssistantResponse(text) => {
                println!("{}", text.bright_white());
            }
            OutputMessage::SystemMessage(msg) => {
                println!("{} {}", "ℹ️".blue(), msg.bright_black());
            }
            // ... other types
        }
        Ok(())
    }
}

// Phase 2: WebexOutputDestination - sends to webex API
// Phase 3: WebSocketOutputDestination - sends to web client
// Phase 4: TuiOutputDestination - split-screen terminal UI
```

### 4. Main Processing Loop

```rust
pub async fn run_repl_with_mspc(
    chat: &mut APChat,
    mspc_channel: Arc<MspcChannel>,
    output_destinations: Vec<Box<dyn OutputDestination>>,
    cancel_token: Arc<Mutex<Option<CancellationToken>>>,
) -> Result<()> {
    loop {
        match mspc_channel.recv().await {
            Some(MspcMessage::InterruptSignal { sender, content }) => {
                // Cancel current LLM operation
                if let Some(token) = cancel_token.lock().await.as_ref() {
                    token.cancel();
                }

                // Clear pending messages
                while mspc_channel.try_recv().is_ok() { }

                // Broadcast interruption
                broadcast_to_all(&output_destinations,
                    OutputMessage::SystemMessage(
                        format!("⚠️  Interrupted by {}", sender)
                    )
                ).await;

                // Process interrupt message
                process_message(chat, &content, &output_destinations, &cancel_token).await?;
            }

            Some(MspcMessage::UserInput { sender, content }) => {
                // Broadcast user message
                broadcast_to_all(&output_destinations,
                    OutputMessage::UserMessage {
                        sender: sender.clone(),
                        text: content.clone()
                    }
                ).await;

                // Process with LLM
                process_message(chat, &content, &output_destinations, &cancel_token).await?;
            }

            Some(MspcMessage::Command { sender, content }) => {
                // Handle commands (/model, /help, etc.)
                handle_command(&content, &output_destinations).await?;
            }

            None => break, // Channel closed
        }
    }
    Ok(())
}

async fn process_message(
    chat: &mut APChat,
    content: &str,
    outputs: &[Box<dyn OutputDestination>],
    cancel_token: &Arc<Mutex<Option<CancellationToken>>>,
) -> Result<()> {
    // Create new cancellation token
    let token = CancellationToken::new();
    *cancel_token.lock().await = Some(token.clone());

    // Call LLM with cancellation support
    let response = chat.process_with_agents(content, Some(token)).await?;

    // Broadcast response to all outputs
    broadcast_to_all(outputs, OutputMessage::AssistantResponse(response)).await;

    Ok(())
}

async fn broadcast_to_all(
    destinations: &[Box<dyn OutputDestination>],
    message: OutputMessage
) {
    for dest in destinations {
        if dest.is_active() {
            if let Err(e) = dest.send_output(&message).await {
                eprintln!("Failed to send to {}: {}", dest.dest_id(), e);
            }
        }
    }
}
```

## Interruption Flow

**When "!urgent message" arrives:**

1. Message parsed as `InterruptSignal { sender, content: "urgent message" }`
2. Cancel current LLM operation via `cancel_token.cancel()`
3. Clear all pending messages in queue: `while try_recv().is_ok() {}`
4. Broadcast interruption notice to all outputs
5. Process interrupt content as new message
6. Resume normal operation

**Key guarantees:**
- Interrupt is destructive (clears queue)
- Only one message processed at a time
- All participants see the interruption
- No partial responses shown

## Error Handling

**Channel Errors:**
- Channel closed → exit REPL loop gracefully
- Send errors → log but continue (non-critical)

**Output Destination Failures:**
- Inactive destinations skipped
- Errors logged but don't stop broadcast
- Dead destinations removed on cleanup

**Readline Errors:**
- EOF (Ctrl-D) → graceful shutdown
- Interrupted (Ctrl-C) → continue with new prompt
- Other errors → log and exit

**Mutex Deadlock Prevention:**
- Always acquire readline guard in tight scopes
- Drop guard before any await points
- Never hold guard across long operations

## Implementation Plan

### Phase 1: Core MSPC with Terminal (Backward Compatible)

**Goal**: Get MSPC working with terminal input, maintain current single-user behavior

1. **Update MSPC Message Types**
   - Add `sender` field to all message variants
   - Test with simple tagged messages

2. **Create InputSourceManager**
   - Spawn single terminal reader task at startup
   - Move readline calls to background task
   - Send to MSPC channel with "terminal" tag

3. **Create Output Abstraction**
   - Define `OutputDestination` trait
   - Implement `TerminalOutputDestination` (simple println)
   - Create `broadcast_to_all()` helper

4. **Update REPL Loop**
   - Read from MSPC channel instead of direct readline
   - Broadcast all outputs instead of println
   - Handle interruption via "!" prefix

5. **Testing**
   - Verify single-user terminal still works
   - Test Ctrl-D, Ctrl-C, "!" interruption
   - Ensure no deadlocks or clobbering

**Success Criteria**: Terminal REPL works exactly as before, but plumbed through MSPC

### Phase 2: Webex Input Source

**Goal**: Add webex bot as second input source

6. **Implement WebexInputSource**
   - Spawn webex polling task
   - Tag messages with webex user ID: "webex-alice"
   - Send to same MSPC channel

7. **Implement WebexOutputDestination**
   - Send messages to webex API
   - Format for webex markdown/cards

8. **Register Both Sources**
   - Terminal + Webex both send to same channel
   - Both receive all outputs

9. **Testing**
   - Verify terminal and webex see same conversation
   - Test interruption from both sources
   - Verify message ordering (FIFO)

**Success Criteria**: Terminal user and webex user collaborate in same conversation

### Phase 3: Web Frontend

**Goal**: Add web interface via WebSocket

10. **Implement WebSocketInputSource**
    - Accept WebSocket connections
    - Tag with connection ID: "websocket-abc123"
    - Send to MSPC channel

11. **Implement WebSocketOutputDestination**
    - Send JSON messages to web clients
    - Handle disconnections gracefully

12. **Update Web Frontend**
    - Display messages with sender tags
    - Support interruption with "!" button
    - Show typing indicators

**Success Criteria**: Terminal, webex, and web all share same conversation

### Phase 4: TUI Enhancement (Optional)

**Goal**: Eliminate output clobbering with split-screen TUI

13. **Add TUI Output**
    - Implement `TuiOutputDestination` using `ratatui`
    - Top region: scrolling output (LLM responses, messages)
    - Bottom region: fixed input line
    - Drop-in replacement for `TerminalOutputDestination`

14. **Configuration**
    - Add `--tui` flag to enable TUI mode
    - Default: simple terminal (backward compatible)

**Success Criteria**: Clean split-screen terminal with no clobbering

## Migration Path

- **Phase 1** maintains current behavior (single terminal user)
- No breaking changes for existing workflows
- Multi-source is transparent to users
- TUI is opt-in enhancement

## Design Decisions

**Why single session instead of multiple?**
- Much simpler architecture
- Collaboration is the default use case
- Subagent system already handles isolation when needed
- Easier to implement and maintain

**Why global input source manager?**
- Prevents multiple readline tasks (deadlock/conflict)
- One reader per physical input source (terminal, webex API, websocket)
- Clean separation: input collection vs message processing

**Why output abstraction?**
- Supports future TUI without refactoring
- Easy to add new destinations (Discord, Slack, etc.)
- Testable without actual I/O

**Why FIFO queue with destructive interrupts?**
- Simple mental model
- Urgent messages truly urgent
- No complex priority logic

## Open Questions

None - design validated with user.

## References

- Original requirement: `docs/high-level/decouple-input.md`
- Existing MSPC code: `src/mspc/channel.rs`
- Input routers: `src/input_router/`
