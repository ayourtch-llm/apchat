# Input Decoupling Implementation Plan

## Overview

This plan provides a detailed, step-by-step implementation strategy for decoupling input/output from the LLM interaction loop using an MSPC (Multi-Stream Processing Channel) system. The plan is based on the validation analysis of the current codebase.

## Implementation Strategy

### Phase 1: Foundation - MSPC Channel System

**Goal:** Create the core channel infrastructure for message routing

#### Task 1.1: Create MSPC Module Structure

```bash
mkdir -p src/mspc
```

**Files to create:**
- `src/mspc/mod.rs` - Main module
- `src/mspc/message.rs` - Message types
- `src/mspc/channel.rs` - Channel implementation

**Implementation:**

```rust
// src/mspc/mod.rs
pub mod message;
pub mod channel;

pub use message::MspcMessage;
pub use channel::MspcChannel;
```

#### Task 1.2: Define Message Types

```rust
// src/mspc/message.rs
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum MspcMessage {
    /// Regular user input from any source
    UserInput {
        content: String,
        source: InputSource,
        metadata: Option<serde_json::Value>,
    },
    
    /// System-generated prompt or notification
    SystemPrompt(String),
    
    /// Request for user confirmation
    ConfirmationRequest {
        prompt: String,
        default: bool,
        callback: Option<tokio::sync::oneshot::Sender<bool>>,
    },
    
    /// Response to confirmation request
    ConfirmationResponse(bool),
    
    /// Interrupt signal (Ctrl+C, "!")
    InterruptSignal(String),
    
    /// Slash command
    Command {
        name: String,
        args: Vec<String>,
    },
    
    /// WebSocket-specific input
    WebSocketInput {
        session_id: String,
        content: String,
        client_id: String,
    },
    
    /// Agent system messages
    AgentMessage {
        content: String,
        agent_id: String,
    },
    
    /// Response from LLM
    LlmResponse {
        content: String,
        model: apchat_models::ModelColor,
        usage: Option<apchat_models::TokenUsage>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputSource {
    Terminal,
    WebSocket(String), // session_id
    Api,
    Agent(String),     // agent_id
}

impl MspcMessage {
    pub fn is_interrupt(&self) -> bool {
        matches!(self, MspcMessage::InterruptSignal(_))
    }
    
    pub fn is_user_input(&self) -> bool {
        matches!(self, MspcMessage::UserInput { .. })
    }
    
    pub fn is_command(&self) -> bool {
        matches!(self, MspcMessage::Command { .. })
    }
}
```

#### Task 1.3: Implement Channel System

```rust
// src/mspc/channel.rs
use tokio::sync::mpsc;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::mspc::message::{MspcMessage, InputSource};
use apchat_models::Message as ChatMessage;

pub struct MspcChannel {
    sender: mpsc::Sender<MspcMessage>,
    receiver: Mutex<mpsc::Receiver<MspcMessage>>,
    message_history: Arc<Mutex<Vec<ChatMessage>>>,
}

impl MspcChannel {
    pub fn new(capacity: usize) -> (Arc<Self>, Arc<Self>) {
        let (sender, receiver) = mpsc::channel(capacity);
        let channel = Arc::new(Self {
            sender: sender.clone(),
            receiver: Mutex::new(receiver),
            message_history: Arc::new(Mutex::new(Vec::new())),
        });
        
        (channel.clone(), channel)
    }
    
    pub async fn send(&self, message: MspcMessage) -> Result<(), ChannelError> {
        self.sender.send(message).await
            .map_err(|e| ChannelError::SendError(e.to_string()))
    }
    
    pub async fn recv(&self) -> Result<MspcMessage, ChannelError> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await
            .ok_or(ChannelError::ChannelClosed)
    }
    
    pub async fn try_recv(&self) -> Result<Option<MspcMessage>, ChannelError> {
        let mut receiver = self.receiver.lock().await;
        match receiver.try_recv() {
            Ok(msg) => Ok(Some(msg)),
            Err(mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(mpsc::error::TryRecvError::Disconnected) => Err(ChannelError::ChannelClosed),
        }
    }
    
    pub async fn recv_interrupt(&self) -> Result<Option<String>, ChannelError> {
        match self.try_recv().await? {
            Some(MspcMessage::InterruptSignal(content)) => Ok(Some(content)),
            Some(_) => Ok(None),
            None => Ok(None),
        }
    }
    
    pub async fn recv_user_input(&self) -> Result<Option<String>, ChannelError> {
        match self.try_recv().await? {
            Some(MspcMessage::UserInput { content, .. }) => Ok(Some(content)),
            Some(_) => Ok(None),
            None => Ok(None),
        }
    }
    
    pub async fn add_to_history(&self, message: ChatMessage) {
        let mut history = self.message_history.lock().await;
        history.push(message);
    }
    
    pub async fn get_history(&self) -> Vec<ChatMessage> {
        let history = self.message_history.lock().await;
        history.clone()
    }
    
    pub async fn clear_history(&self) {
        let mut history = self.message_history.lock().await;
        history.clear();
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ChannelError {
    #[error("Channel send error: {0}")]
    SendError(String),
    #[error("Channel closed")]
    ChannelClosed,
    #[error("Channel operation timeout")]
    Timeout,
}
```

### Phase 2: Input Routers

**Goal:** Create routers for different input sources

#### Task 2.1: Terminal Input Router

```rust
// src/input_router/mod.rs
pub mod terminal;
pub mod websocket;

pub use terminal::TerminalInputRouter;
pub use websocket::WebSocketInputRouter;
```

```rust
// src/input_router/terminal.rs
use std::sync::Arc;
use tokio::sync::Mutex;
use rustyline::Editor;
use colored::Colorize;

use crate::mspc::{MspcChannel, MspcMessage};

pub struct TerminalInputRouter {
    channel: Arc<MspcChannel>,
    rl: Arc<Mutex<Editor<()>>>,
}

impl TerminalInputRouter {
    pub fn new(channel: Arc<MspcChannel>) -> Self {
        Self {
            channel,
            rl: Arc::new(Mutex::new(Editor::<()>::new().unwrap())),
        }
    }
    
    pub async fn run(&self) {
        loop {
            let prompt = format!("{} {}", 
                "You:".bright_green().bold(),
                "Enter your message:"
            );
            
            let input = {
                let mut rl = self.rl.lock().await;
                match rl.readline(&prompt) {
                    Ok(line) => line,
                    Err(rustyline::error::ReadlineError::Interrupted) => {
                        self.channel.send(MspcMessage::InterruptSignal("Ctrl+C".to_string())).await
                            .unwrap_or(());
                        continue;
                    }
                    Err(rustyline::error::ReadlineError::Eof) => break,
                    Err(_) => continue,
                }
            };
            
            if input.trim().is_empty() {
                continue;
            }
            
            // Parse and route input
            let message = self.parse_input(&input);
            self.channel.send(message).await.unwrap_or(());
            
            // Add to readline history
            {
                let mut rl = self.rl.lock().await;
                rl.add_history_entry(&input).unwrap();
            }
        }
    }
    
    fn parse_input(&self, input: &str) -> MspcMessage {
        let input = input.trim();
        
        if input.starts_with('!') {
            return MspcMessage::InterruptSignal(input[1..].to_string());
        }
        
        if input.starts_with('/') {
            return self.parse_command(input);
        }
        
        MspcMessage::UserInput {
            content: input.to_string(),
            source: crate::mspc::message::InputSource::Terminal,
            metadata: None,
        }
    }
    
    fn parse_command(&self, input: &str) -> MspcMessage {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return MspcMessage::UserInput {
                content: input.to_string(),
                source: crate::mspc::message::InputSource::Terminal,
                metadata: None,
            };
        }
        
        let command = parts[0][1..].to_string(); // Remove leading '/'
        let args = parts[1..].iter().map(|s| s.to_string()).collect();
        
        MspcMessage::Command {
            name: command,
            args,
        }
    }
}
```

#### Task 2.2: WebSocket Input Router

```rust
// src/input_router/websocket.rs
use std::sync::Arc;

use crate::mspc::{MspcChannel, MspcMessage};
use crate::web::protocol::ClientMessage;

pub struct WebSocketInputRouter {
    channel: Arc<MspcChannel>,
}

impl WebSocketInputRouter {
    pub fn new(channel: Arc<MspcChannel>) -> Self {
        Self { channel }
    }
    
    pub async fn handle_message(&self, session_id: &str, client_id: &str, message: ClientMessage) -> Result<(), RouterError> {
        match message {
            ClientMessage::UserMessage { content } => {
                let msg = MspcMessage::WebSocketInput {
                    session_id: session_id.to_string(),
                    content,
                    client_id: client_id.to_string(),
                };
                self.channel.send(msg).await
                    .map_err(|e| RouterError::ChannelError(e.to_string()))?;
            }
            ClientMessage::Interrupt => {
                let msg = MspcMessage::InterruptSignal(format!("WebSocket interrupt from {}", client_id));
                self.channel.send(msg).await
                    .map_err(|e| RouterError::ChannelError(e.to_string()))?;
            }
            ClientMessage::ConfirmationResponse { response } => {
                let msg = MspcMessage::ConfirmationResponse(response);
                self.channel.send(msg).await
                    .map_err(|e| RouterError::ChannelError(e.to_string()))?;
            }
            _ => {
                // Other message types can be handled as needed
                Ok(())
            }
        }
        
        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum RouterError {
    #[error("Channel error: {0}")]
    ChannelError(String),
    #[error("Router error: {0}")]
    RouterError(String),
}
```

### Phase 3: LLM Interaction Loop Refactoring

**Goal:** Convert the chat loop to use MSPC channel

#### Task 3.1: Create Continuous Chat Loop

```rust
// src/chat/session.rs (MODIFIED)

/// New continuous chat loop that uses MSPC channel
pub(crate) async fn chat_loop(
    chat: &mut APChat,
    channel: Arc<MspcChannel>,
    cancellation_token: Option<tokio_util::sync::CancellationToken>,
) -> Result<()> {
    loop {
        // Check for cancellation
        if let Some(ref token) = cancellation_token {
            if token.is_cancelled() {
                return Ok(()); // Graceful exit
            }
        }
        
        // Check for interrupts
        if let Some(interrupt) = channel.recv_interrupt().await? {
            println!("{} {}", "⚠️ Interrupt:".bright_yellow(), interrupt);
            // Handle interrupt - could clear current generation
            continue;
        }
        
        // Check for user input
        if let Some(input) = channel.recv_user_input().await? {
            // Process user input
            process_user_input(chat, &input).await?;
            
            // Generate response
            let response = generate_response(chat).await?;
            
            // Send response back through channel
            let msg = MspcMessage::LlmResponse {
                content: response.clone(),
                model: chat.current_model,
                usage: None, // Could be populated
            };
            channel.send(msg).await?;
        }
        
        // Small sleep to prevent busy waiting
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

async fn process_user_input(chat: &mut APChat, input: &str) -> Result<()> {
    // Add to message history
    chat.messages.push(ChatMessage {
        role: "user".to_string(),
        content: input.to_string(),
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning: None,
    });
    
    // Compact history if needed
    crate::chat::history::summarize_and_trim_history(chat).await?;
    
    Ok(())
}

async fn generate_response(chat: &mut APChat) -> Result<String> {
    // This would call the existing chat logic but adapted for the loop
    // Would need to handle tool calls, etc.
    
    // For now, call the existing chat function with a dummy message
    // This is a temporary bridge until fully refactored
    let dummy_msg = "generate_response".to_string();
    crate::chat::session::chat(chat, &dummy_msg, None).await
}
```

#### Task 3.2: Update Main REPL Loop

```rust
// src/app/repl.rs (MODIFIED)

pub async fn run_repl_mode(
    cli: &Cli,
    client_config: ClientConfig,
    work_dir: PathBuf,
    policy_manager: PolicyManager,
) -> Result<()> {
    // ... existing setup code ...
    
    // Create MSPC channel
    let (mspc_tx, mspc_rx) = crate::mspc::MspcChannel::new(100);
    
    // Create terminal input router
    let terminal_router = TerminalInputRouter::new(mspc_tx.clone());
    
    // Spawn terminal input router
    tokio::spawn(async move {
        terminal_router.run().await;
    });
    
    // Create cancellation token
    let cancel_token = tokio_util::sync::CancellationToken::new();
    
    // Spawn Ctrl-C handler
    let token_for_handler = cancel_token.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            token_for_handler.cancel();
        }
    });
    
    // Run chat loop
    let result = chat_loop(&mut chat, mspc_rx, Some(cancel_token)).await;
    
    // ... handle result and cleanup ...
}
```

### Phase 4: Integration and Testing

**Goal:** Integrate components and ensure everything works together

#### Task 4.1: WebSocket Integration

```rust
// src/web/routes.rs (MODIFIED)

async fn handle_websocket(socket: WebSocket, state: AppState, session_id: SessionId) {
    let client_id = Uuid::new_v4();
    
    // Get or verify session exists
    let session = match state.session_manager.get_session(&session_id).await {
        Some(s) => s,
        None => {
            eprintln!("WebSocket: Session {} not found", session_id);
            return;
        }
    };
    
    // Create WebSocket input router
    let mspc_channel = Arc::new(MspcChannel::new(100));
    let ws_router = WebSocketInputRouter::new(mspc_channel.clone());
    
    // Add client to session with router
    session.add_client_with_router(client_id, mspc_channel.clone()).await;
    
    // ... rest of WebSocket handling ...
}
```

#### Task 4.2: Command Handling

```rust
// src/input_router/terminal.rs (EXTENDED)

impl TerminalInputRouter {
    // ... existing methods ...
    
    pub async fn handle_command(&self, command: &str, args: Vec<String>) -> Result<(), RouterError> {
        match command.as_str() {
            "model" => self.handle_model_command(args).await,
            "skills" => self.handle_skills_command().await,
            "compact" => self.handle_compact_command().await,
            "confirm" => self.handle_confirm_command().await,
            _ => Ok(()), // Unknown command, pass through as user input
        }
    }
    
    async fn handle_model_command(&self, args: Vec<String>) -> Result<(), RouterError> {
        // Implement model switching logic
        // Could send confirmation request if needed
        Ok(())
    }
    
    async fn handle_skills_command(&self) -> Result<(), RouterError> {
        // Display skills help
        // Could be a system message
        Ok(())
    }
    
    async fn handle_compact_command(&self) -> Result<(), RouterError> {
        // Request compaction
        // Could send confirmation request
        Ok(())
    }
    
    async fn handle_confirm_command(&self) -> Result<(), RouterError> {
        // Toggle confirmation mode
        // Could send system message
        Ok(())
    }
}
```

#### Task 4.3: Testing Strategy

**Test Files to Create:**
- `tests/test_mspc_channel.rs` - Channel functionality
- `tests/test_input_routers.rs` - Terminal and WebSocket routers
- `tests/test_chat_loop.rs` - Continuous chat loop
- `tests/test_interrupts.rs` - Interrupt handling
- `tests/test_confirmations.rs` - Confirmation prompts

**Key Test Scenarios:**
1. Message routing through channel
2. Interrupt handling from terminal
3. WebSocket input processing
4. Command parsing and execution
5. Message history preservation
6. Concurrent input handling
7. Cancellation token integration

## Migration Strategy

### Option 1: Big Bang (Recommended for this project)
- Implement all phases
- Test thoroughly
- Replace old REPL with new one
- **Pros:** Clean architecture, no technical debt
- **Cons:** Riskier, longer downtime

### Option 2: Gradual Migration
- Keep old REPL running
- Add new MSPC-based REPL as alternative
- Slowly migrate features
- **Pros:** Lower risk, can fall back
- **Cons:** More complex, dual code paths

### Option 3: Hybrid Approach
- Implement MSPC infrastructure first
- Gradually move components to use it
- **Pros:** Balanced risk/reward
- **Cons:** More coordination needed

## Timeline Estimate

- **Phase 1 (Foundation):** 2-3 days
- **Phase 2 (Input Routers):** 2 days
- **Phase 3 (LLM Loop):** 3-4 days
- **Phase 4 (Integration):** 3 days
- **Testing:** 3-5 days
- **Total:** 2-3 weeks

## Success Metrics

1. **Functionality:** All existing features work
2. **Performance:** No significant slowdown
3. **Reliability:** No message loss or corruption
4. **Extensibility:** Easy to add new input sources
5. **Test Coverage:** 80%+ of new code tested

## Conclusion

This implementation plan provides a detailed, step-by-step approach to implementing the input decoupling architecture. The plan:

1. **Builds on existing infrastructure** (WebSocket channels, message history)
2. **Preserves all existing functionality** (commands, interrupts, history)
3. **Enables future extensibility** (new input sources, better routing)
4. **Maintains async/await pattern** throughout
5. **Includes comprehensive testing** strategy

The implementation should result in a more maintainable, extensible architecture that supports multiple input sources while maintaining all current functionality.
