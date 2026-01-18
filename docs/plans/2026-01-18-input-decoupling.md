# Decoupling the Input Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Decouple terminal input/output from the LLM interaction loop using MSPC channels to enable flexible input sources

**Architecture:** Implement an MSPC (Multi-Stream Processing Channel) system where inputs from various sources (terminal, webex bot, etc.) are routed through a central channel. The LLM interaction loop checks this channel frequently:
- Inputs starting with "!" interrupt the current loop immediately
- Regular inputs wait until the end of the current turn
- Message history is maintained with proper user/agent pairing
- Confirmation prompts are handled through the same channel

**Tech Stack:** Rust, MSPC channels, async/await, message history management

---

## Current State Analysis

Before implementation, we need to understand the existing codebase structure:
1. Current input/output handling in the terminal
2. LLM interaction loop implementation
3. Message history management
4. Confirmation prompt handling
5. Existing MSPC or channel implementations

---

### Task 0: Repository Analysis

**Files:**
- Explore: `src/**/*.rs` (all source files)
- Explore: `docs/**/*.md` (documentation)
- Explore: `tests/**/*.rs` (existing tests)

**Step 1: Map current architecture**
- Identify main entry point
- Find LLM interaction loop
- Locate input/output handling
- Find message history management
- Identify confirmation prompt logic

**Step 2: Identify existing MSPC or channel usage**
- Search for channel implementations
- Find any existing multi-source input handling
- Note previous implementation attempts

**Step 3: Document findings**
- Create architecture diagram
- List key components and their interactions
- Identify pain points in current implementation

**Step 4: Commit**
```bash
git add docs/
git commit -m "docs: architecture analysis for input decoupling"
```

---

### Task 1: Design MSPC Channel System

**Files:**
- Create: `src/mspc/mod.rs`
- Create: `src/mspc/channel.rs`
- Create: `src/mspc/message.rs`
- Modify: `src/lib.rs` (add module)

**Step 1: Define Message types**
```rust
pub enum MspcMessage {
    UserInput(String),
    SystemPrompt(String),
    ConfirmationRequest(String),
    ConfirmationResponse(bool),
    InterruptSignal(String),
    Command(String),
}
```

**Step 2: Implement channel structure**
```rust
pub struct MspcChannel {
    sender: Sender<MspcMessage>,
    receiver: Receiver<MspcMessage>,
    message_history: Vec<MessagePair>,
}

impl MspcChannel {
    pub fn new() -> Self {
        // Create bounded channel
        // Initialize message history
    }
    
    pub fn send(&self, message: MspcMessage) -> Result<(), ChannelError> {
        // Send message through channel
    }
    
    pub fn try_recv(&self) -> Result<Option<MspcMessage>, ChannelError> {
        // Non-blocking receive
    }
    
    pub fn recv(&self) -> Result<MspcMessage, ChannelError> {
        // Blocking receive
    }
}
```

**Step 3: Implement message history management**
```rust
pub struct MessagePair {
    pub user: String,
    pub agent: String,
}

impl MspcChannel {
    pub fn add_user_message(&mut self, content: String) {
        // Add to history, ensure proper pairing
    }
    
    pub fn add_agent_message(&mut self, content: String) {
        // Add to history, ensure proper pairing
    }
    
    pub fn handle_interruption(&mut self) {
        // Clean up interrupted agent message
        // Insert interruption marker if needed
    }
    
    pub fn get_history_for_prompt(&self) -> Vec<MessagePair> {
        // Return history formatted for LLM prompt
    }
}
```

**Step 4: Commit**
```bash
git add src/mspc/
git commit -m "feat: implement MSPC channel system"
```

---

### Task 2: Implement Input Routers

**Files:**
- Create: `src/input_router/mod.rs`
- Create: `src/input_router/terminal.rs`
- Create: `src/input_router/webex.rs` (stub for future)

**Step 1: Terminal input router**
```rust
pub struct TerminalInputRouter {
    channel: Arc<MspcChannel>,
}

impl TerminalInputRouter {
    pub fn new(channel: Arc<MspcChannel>) -> Self {
        Self { channel }
    }
    
    pub async fn run(&self) {
        // Read from stdin
        // Parse input
        // Route to MSPC channel
        // Handle confirmation prompts
    }
    
    fn parse_input(&self, input: &str) -> MspcMessage {
        // Check for "!" prefix for interrupt
        // Check for "/" commands
        // Return appropriate message type
    }
}
```

**Step 2: Webex stub (for future expansion)**
```rust
pub struct WebexInputRouter {
    channel: Arc<MspcChannel>,
}

impl WebexInputRouter {
    pub fn new(channel: Arc<MspcChannel>) -> Self {
        Self { channel }
    }
    
    pub async fn run(&self) {
        // Stub implementation
        // Will connect to Webex API in future
    }
}
```

**Step 3: Commit**
```bash
git add src/input_router/
git commit -m "feat: implement input routers"
```

---

### Task 3: Modify LLM Interaction Loop

**Files:**
- Modify: `src/llm_loop.rs` (or wherever the main loop is)

**Step 1: Integrate MSPC channel**
```rust
pub struct LlmInteractionLoop {
    channel: Arc<MspcChannel>,
    llm_client: Arc<LlmClient>,
    // other dependencies
}

impl LlmInteractionLoop {
    pub fn new(channel: Arc<MspcChannel>, llm_client: Arc<LlmClient>) -> Self {
        Self { channel, llm_client }
    }
    
    pub async fn run(&self) -> Result<(), LoopError> {
        loop {
            self.process_turn().await?;
        }
    }
    
    async fn process_turn(&self) -> Result<(), LoopError> {
        // Check for interrupts
        // Process regular inputs at turn end
        // Generate agent response
        // Handle message history
    }
}
```

**Step 2: Implement interrupt handling**
```rust
impl LlmInteractionLoop {
    async fn check_for_interrupt(&self) -> Result<Option<String>, LoopError> {
        // Non-blocking check for interrupt messages
        // Return interrupt content if found
    }
    
    async fn check_for_new_input(&self) -> Result<Option<String>, LoopError> {
        // Non-blocking check for regular input
        // Return input content if found
    }
    
    async fn process_interrupt(&self, interrupt: String) -> Result<(), LoopError> {
        // Clean up message history
        // Handle the interrupt
        // Generate immediate response
    }
}
```

**Step 3: Implement turn processing**
```rust
impl LlmInteractionLoop {
    async fn process_turn(&self) -> Result<(), LoopError> {
        // Generate agent response
        let response = self.generate_response().await?;
        
        // Output response
        self.output_response(&response).await?;
        
        // Check for interrupts during output
        if let Some(interrupt) = self.check_for_interrupt().await? {
            self.process_interrupt(interrupt).await?;
            return Ok(());
        }
        
        // Check for new input at turn end
        if let Some(input) = self.check_for_new_input().await? {
            self.handle_new_input(input).await?;
        }
        
        Ok(())
    }
    
    async fn handle_new_input(&self, input: String) -> Result<(), LoopError> {
        // Add to message history
        // Generate response to input
        // Output response
    }
}
```

**Step 4: Commit**
```bash
git add src/llm_loop.rs
git commit -m "feat: integrate MSPC into LLM interaction loop"
```

---

### Task 4: Message History Management

**Files:**
- Modify: `src/mspc/message.rs`

**Step 1: Implement history validation**
```rust
impl MspcChannel {
    pub fn validate_history(&self) -> bool {
        // Ensure history starts with user message
        // Ensure last message is agent message
        // Ensure proper pairing
    }
    
    pub fn fix_history(&mut self) {
        // Remove incomplete pairs
        // Add interruption markers if needed
        // Ensure proper structure
    }
}
```

**Step 2: Implement history formatting for prompts**
```rust
impl MspcChannel {
    pub fn format_history_for_prompt(&self) -> String {
        // Format message pairs for LLM
        // Handle system messages
        // Ensure proper ordering
    }
}
```

**Step 3: Commit**
```bash
git add src/mspc/message.rs
git commit -m "feat: enhance message history management"
```

---

### Task 5: Confirmation Prompt Handling

**Files:**
- Modify: `src/input_router/terminal.rs`
- Modify: `src/llm_loop.rs`

**Step 1: Implement confirmation request**
```rust
pub enum MspcMessage {
    // ... existing variants
    ConfirmationRequest {
        prompt: String,
        default: bool,
    },
    ConfirmationResponse(bool),
}
```

**Step 2: Handle confirmation in terminal router**
```rust
impl TerminalInputRouter {
    async fn handle_confirmation(&self, prompt: &str, default: bool) -> bool {
        // Display prompt
        // Read response from stdin
        // Parse yes/no response
        // Return boolean result
    }
}
```

**Step 3: Integrate into LLM loop**
```rust
impl LlmInteractionLoop {
    async fn request_confirmation(&self, prompt: &str, default: bool) -> Result<bool, LoopError> {
        // Send confirmation request
        // Wait for response
        // Return result
    }
}
```

**Step 4: Commit**
```bash
git add src/input_router/terminal.rs src/llm_loop.rs
git commit -m "feat: implement confirmation prompt handling"
```

---

### Task 6: Integration and Main Entry Point

**Files:**
- Modify: `src/main.rs`

**Step 1: Create MSPC channel**
```rust
fn main() {
    // Initialize logging
    // Create MSPC channel
    let channel = Arc::new(MspcChannel::new());
    
    // Spawn input routers
    let terminal_router = TerminalInputRouter::new(channel.clone());
    tokio::spawn(async move {
        terminal_router.run().await
    });
    
    // Create LLM client
    let llm_client = Arc::new(LlmClient::new());
    
    // Create and run interaction loop
    let loop_handler = LlmInteractionLoop::new(channel, llm_client);
    
    if let Err(e) = loop_handler.run().await {
        eprintln!("Error in interaction loop: {}", e);
        std::process::exit(1);
    }
}
```

**Step 2: Commit**
```bash
git add src/main.rs
git commit -m "feat: integrate MSPC into main entry point"
```

---

### Task 7: Command Handling Preservation

**Files:**
- Modify: `src/input_router/terminal.rs`

**Step 1: Preserve "/" command handling**
```rust
impl TerminalInputRouter {
    fn parse_input(&self, input: &str) -> MspcMessage {
        if input.starts_with('!') {
            return MspcMessage::InterruptSignal(input[1..].to_string());
        }
        
        if input.starts_with('/') {
            return self.handle_command(input);
        }
        
        MspcMessage::UserInput(input.to_string())
    }
    
    fn handle_command(&self, command: &str) -> MspcMessage {
        // Parse and handle existing commands
        // Return appropriate message type
    }
}
```

**Step 2: Commit**
```bash
git add src/input_router/terminal.rs
git commit -m "feat: preserve command handling"
```

---

### Task 8: Testing

**Files:**
- Create: `tests/test_mspc.rs`
- Create: `tests/test_input_routing.rs`
- Create: `tests/test_interrupts.rs`

**Step 1: Test MSPC channel**
```rust
#[test]
fn test_channel_send_recv() {
    let channel = MspcChannel::new();
    
    channel.send(MspcMessage::UserInput("test".to_string())).unwrap();
    
    let msg = channel.recv().unwrap();
    assert!(matches!(msg, MspcMessage::UserInput(s) if s == "test"));
}

#[test]
fn test_message_history() {
    let mut channel = MspcChannel::new();
    
    channel.add_user_message("hello".to_string());
    channel.add_agent_message("hi there".to_string());
    
    let history = channel.get_history_for_prompt();
    assert_eq!(history.len(), 1);
}
```

**Step 2: Test interrupt handling**
```rust
#[tokio::test]
async fn test_interrupt_handling() {
    let channel = Arc::new(MspcChannel::new());
    let llm_client = Arc::new(MockLlmClient::new());
    
    let loop_handler = LlmInteractionLoop::new(channel.clone(), llm_client);
    
    // Start the loop in background
    let handle = tokio::spawn(async move {
        loop_handler.run().await
    });
    
    // Send interrupt
    channel.send(MspcMessage::InterruptSignal("stop!".to_string())).unwrap();
    
    // Verify interrupt was processed
    // ...
}
```

**Step 3: Test confirmation prompts**
```rust
#[tokio::test]
async fn test_confirmation_prompt() {
    let channel = Arc::new(MspcChannel::new());
    
    // Send confirmation request
    channel.send(MspcMessage::ConfirmationRequest {
        prompt: "Continue?".to_string(),
        default: true,
    }).unwrap();
    
    // Send confirmation response
    channel.send(MspcMessage::ConfirmationResponse(true)).unwrap();
    
    // Verify handling
    // ...
}
```

**Step 4: Commit**
```bash
git add tests/
git commit -m "test: add MSPC and input routing tests"
```

---

### Task 9: Documentation

**Files:**
- Create: `docs/architecture/mspc-architecture.md`
- Modify: `docs/architecture/overview.md`

**Step 1: Document MSPC architecture**
```markdown
# MSPC Architecture

## Overview

The Multi-Stream Processing Channel (MSPC) system decouples input sources from the LLM interaction loop, enabling flexible integration of multiple input channels.

## Components

### 1. MSPC Channel
- Central message bus
- Handles message routing
- Manages message history
- Ensures proper message pairing

### 2. Input Routers
- TerminalInputRouter: Reads from stdin
- WebexInputRouter: Future Webex bot integration
- Can be extended for other sources

### 3. LLM Interaction Loop
- Consumes messages from MSPC
- Generates responses
- Handles interrupts
- Manages turn processing

## Message Types

- `UserInput`: Regular user message
- `InterruptSignal`: Immediate interrupt (starts with "!")
- `ConfirmationRequest`: Request for user confirmation
- `ConfirmationResponse`: User confirmation response
- `Command`: Slash commands (/, /help, etc.)

## Interrupt Handling

When an interrupt is received:
1. Current agent message is cleaned up
2. Interruption marker may be added
3. Immediate response is generated
4. Normal flow resumes

## Message History

History must always:
- Start with a user message after system messages
- End with an agent message
- Maintain proper user/agent pairing
- Handle interruptions gracefully
```

**Step 2: Update overview documentation**

**Step 3: Commit**
```bash
git add docs/
git commit -m "docs: add MSPC architecture documentation"
```

---

### Task 10: Verification and Testing

**Files:**
- Modify: `tests/integration.rs`

**Step 1: Integration tests**
```rust
#[tokio::test]
async fn test_full_interaction_flow() {
    // Test complete flow:
    // 1. User input
    // 2. Agent response
    // 3. Interrupt handling
    // 4. Confirmation prompts
    // 5. Command handling
}

#[tokio::test]
async fn test_message_history_invariants() {
    // Test that history always maintains proper structure
    // Test interruption handling
    // Test recovery scenarios
}
```

**Step 2: Run all tests**
```bash
cargo test
```

**Step 3: Commit**
```bash
git add tests/
git commit -m "test: add integration tests"
```

---

### Task 11: Cleanup and Finalization

**Files:**
- Clean up: `src/**/*.rs`
- Clean up: `tests/**/*.rs`

**Step 1: Remove dead code**
- Remove unused functions
- Remove commented-out code
- Clean up debug prints

**Step 2: Ensure all tests pass**
```bash
cargo test --all
```

**Step 3: Final commit**
```bash
git add .
git commit -m "feat: complete MSPC input decoupling implementation"
```

---

## Implementation Notes

1. **Thread Safety**: All shared state must be wrapped in `Arc<Mutex<...>>` or use async channels properly
2. **Error Handling**: Implement comprehensive error handling for all operations
3. **Testing**: Write tests for all components before implementation
4. **Documentation**: Keep documentation updated throughout development
5. **Backward Compatibility**: Ensure existing functionality continues to work
6. **Performance**: Consider performance implications of frequent channel checks

## Risk Mitigation

1. **Complexity**: Break into small, testable components
2. **Race Conditions**: Use proper synchronization primitives
3. **Message Loss**: Implement acknowledgment system if needed
4. **History Corruption**: Validate history after each modification

---

Plan complete and saved to `docs/plans/2026-01-18-input-decoupling.md`. Two execution options:

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**
