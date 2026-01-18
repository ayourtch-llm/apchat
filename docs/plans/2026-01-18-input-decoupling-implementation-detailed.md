# Decoupling the Input - Technical Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a fully decoupled input system using MSPC channels that allows multiple input sources (terminal, webex, etc.) to interact with the LLM loop, with proper interrupt handling, message history management, and confirmation prompt routing.

**Architecture:** The system will use an MSPC (Multi-Stream Processing Channel) architecture where:
1. Input routers (terminal, webex) send messages to a central channel
2. The LLM interaction loop checks the channel frequently for messages
3. Interrupts (starting with "!") are processed immediately
4. Regular inputs wait until the end of the current turn
5. Message history maintains proper user/agent pairing
6. Confirmation prompts are routed through the same channel

**Tech Stack:** Rust, Tokio async runtime, MPSC channels, Arc/Mutex for thread safety

---

## Current State Analysis

### Key Components Identified

1. **MSPC Channel System** (`src/mspc/`)
   - ✅ Already implemented with `MspcChannel` structure
   - ✅ Supports message types: UserInput, InterruptSignal, Command, ConfirmationRequest/Response
   - ✅ Message history management with `MessagePair` structure
   - ✅ Interrupt handling methods exist
   - ⚠️ **Issue**: Message history doesn't enforce "always start with user, end with agent" invariant

2. **Input Routers** (`src/input_router/`)
   - ✅ TerminalInputRouter implemented
   - ✅ WebexInputRouter stub exists
   - ✅ Input parsing logic for "!", "/", and regular input
   - ⚠️ **Issue**: Terminal input reading is blocking, needs async handling

3. **LLM Interaction Loop** (`src/chat/mspc_session.rs`)
   - ✅ New `chat_with_mspc` function implemented
   - ✅ Interrupt handling logic exists
   - ✅ Command handling integrated
   - ✅ Confirmation request/response handling
   - ⚠️ **Issue**: Doesn't fully integrate with existing chat logic
   - ⚠️ **Issue**: Message history management needs improvement

4. **Message History** (`src/mspc/channel.rs`)
   - ✅ Basic history storage implemented
   - ✅ Methods to add user/agent messages
   - ⚠️ **Issue**: No validation of history invariants
   - ⚠️ **Issue**: Interruption cleanup incomplete

5. **Confirmation System** (`src/chat/mspc_session.rs`)
   - ✅ Confirmation request/response message types
   - ✅ Basic confirmation handling in loop
   - ⚠️ **Issue**: Terminal router doesn't prompt for confirmations
   - ⚠️ **Issue**: No integration with policy manager

### Existing Implementation Status

**Good:**
- MSPC channel infrastructure is in place
- Input routers exist (terminal and webex stub)
- Interrupt signal handling works
- Command parsing functional
- Confirmation message types defined

**Needs Work:**
- Message history validation and invariant enforcement
- Full integration with existing chat/LLM logic
- Terminal input reading needs async handling
- Confirmation prompt system incomplete
- Input protection against clobbering
- Turn-based input processing (wait until turn end)

---

## Implementation Plan

### Task 0: Verify Current Implementation

**Files:**
- Test: `src/input_router/tests.rs`
- Test: `src/chat/tests.rs`

**Steps:**

1. **Run existing tests**
   ```bash
   cd apchat-main
   cargo test input_router
   cargo test mspc
   ```

2. **Verify MSPC channel functionality**
   - Test message sending/receiving
   - Test interrupt signal routing
   - Test command parsing
   - Test message history operations

3. **Document current behavior**
   - Create test scenarios for expected vs actual behavior
   - Identify gaps in current implementation

4. **Commit**
   ```bash
   git add tests/
   git commit -m "test: verify current MSPC implementation"
   ```

---

### Task 1: Enhance Message History Management

**Files:**
- Modify: `src/mspc/channel.rs`

**Steps:**

1. **Implement history validation**
   ```rust
   impl MspcChannel {
       /// Validate history invariants:
       /// - After system messages, first message must be user
       /// - Last message must be agent (unless interrupted)
       /// - All user messages must have agent responses (except last)
       pub async fn validate_history(&self) -> Result<(), HistoryError> {
           // Implementation
       }
   }
   ```

2. **Implement history fixing**
   ```rust
   impl MspcChannel {
       /// Fix history to maintain invariants
       /// - Remove incomplete pairs
       /// - Add interruption markers if needed
       /// - Ensure proper structure
       pub async fn fix_history(&mut self) {
           // Implementation
       }
   }
   ```

3. **Implement proper interruption handling**
   ```rust
   impl MspcChannel {
       pub async fn handle_interruption(&self) -> String {
           let mut history = self.message_history.lock().await;
           
           if let Some(last) = history.last_mut() {
               if !last.agent.is_empty() {
                   // Save interrupted agent message
                   let interrupted = std::mem::take(&mut last.agent);
                   
                   // Add interruption marker
                   history.push(MessagePair {
                       user: "== interrupted ==".to_string(),
                       agent: String::new(),
                   });
                   
                   return interrupted;
               }
           }
           
           String::new()
       }
   }
   ```

4. **Add tests**
   ```rust
   #[tokio::test]
   async fn test_history_validation() {
       // Test valid history
       // Test invalid history
       // Test history fixing
   }
   ```

5. **Commit**
   ```bash
   git add src/mspc/channel.rs
   git commit -m "feat: enhance message history management"
   ```

---

### Task 2: Implement Turn-Based Input Processing

**Files:**
- Modify: `src/chat/mspc_session.rs`

**Steps:**

1. **Refactor chat loop to implement turn processing**
   ```rust
   pub(crate) async fn chat_with_mspc(
       chat: &mut APChat,
       mspc_channel: Arc<MspcChannel>,
       cancellation_token: Option<tokio_util::sync::CancellationToken>,
   ) -> Result<()> {
       // Initialize
       
       loop {
           // Check for cancellation
           
           // Check for interrupts (immediate)
           if let Some(interrupt) = check_for_interrupt(&mspc_channel).await? {
               process_interrupt(chat, &mspc_channel, interrupt).await?;
               continue;
           }
           
           // Generate agent response
           let response = generate_agent_response(chat).await?;
           
           // Output response
           output_response(&response).await?;
           
           // Check for interrupts during output
           if let Some(interrupt) = check_for_interrupt(&mspc_channel).await? {
               process_interrupt(chat, &mspc_channel, interrupt).await?;
               continue;
           }
           
           // Check for regular inputs at turn end
           while let Some(input) = check_for_new_input(&mspc_channel).await? {
               process_user_input(chat, &mspc_channel, &input).await?;
           }
           
           // Small delay to prevent busy waiting
           tokio::time::sleep(Duration::from_millis(100)).await;
       }
   }
   ```

2. **Implement interrupt checking**
   ```rust
   async fn check_for_interrupt(channel: &MspcChannel) -> Result<Option<String>> {
       match channel.try_recv().await {
           Ok(Some(MspcMessage::InterruptSignal(content))) => {
               Ok(Some(content))
           }
           Ok(Some(_)) => Ok(None), // Not an interrupt
           Ok(None) => Ok(None),   // No message
           Err(_) => Ok(None),      // Channel error
       }
   }
   ```

3. **Implement regular input checking**
   ```rust
   async fn check_for_new_input(channel: &MspcChannel) -> Result<Option<String>> {
       match channel.try_recv().await {
           Ok(Some(MspcMessage::UserInput(content))) => {
               Ok(Some(content))
           }
           Ok(Some(MspcMessage::Command(content))) => {
               process_command(content).await;
               Ok(None)
           }
           Ok(Some(_)) => Ok(None), // Other message types
           Ok(None) => Ok(None),   // No message
           Err(_) => Ok(None),      // Channel error
       }
   }
   ```

4. **Implement interrupt processing**
   ```rust
   async fn process_interrupt(
       chat: &mut APChat,
       channel: &MspcChannel,
       interrupt: String,
   ) -> Result<()> {
       // Clean up interrupted message
       let interrupted = channel.handle_interruption().await;
       
       // Add to chat messages
       chat.messages.push(Message {
           role: "user".to_string(),
           content: format!("[INTERRUPTED] {}", interrupt),
           tool_calls: None,
           tool_call_id: None,
           name: None,
           reasoning: None,
       });
       
       // Add to MSPC history
       channel.add_user_message(format!("[INTERRUPTED] {}", interrupt)).await;
       
       Ok(())
   }
   ```

5. **Commit**
   ```bash
   git add src/chat/mspc_session.rs
   git commit -m "feat: implement turn-based input processing"
   ```

---

### Task 3: Complete Confirmation Prompt System

**Files:**
- Modify: `src/input_router/terminal.rs`
- Modify: `src/chat/mspc_session.rs`

**Steps:**

1. **Implement confirmation prompt in terminal router**
   ```rust
   impl TerminalInputRouter {
       pub async fn handle_confirmation_prompt(&self, prompt: &str) -> bool {
           // Send confirmation request to channel
           self.channel.send(MspcMessage::ConfirmationRequest(prompt.to_string())).await?;
           
           // Wait for response (blocking is OK here as we're in interactive mode)
           match self.channel.recv().await {
               Some(MspcMessage::ConfirmationResponse(response)) => response,
               _ => false,
           }
       }
   }
   ```

2. **Update confirmation handling in chat loop**
   ```rust
   // In chat_with_mspc loop:
   } else if mspc_channel.is_confirmation_request(&message) {
       if let MspcMessage::ConfirmationRequest(content) = message {
           eprintln!("{} {}", "❓".yellow(), content);
           eprintln!("{} Type 'yes' or 'no': ", "👉".bright_black());
           
           // Read from stdin directly (not through channel to avoid recursion)
           let response = read_confirmation_response().await;
           
           // Send response back through channel
           mspc_channel.send(MspcMessage::ConfirmationResponse(response)).await;
           
           continue;
       }
   }
   ```

3. **Integrate with policy manager**
   ```rust
   // Modify existing policy check to use MSPC for confirmations
   impl PolicyManager {
       pub async fn should_execute(&self, action: &str, mspc_channel: &MspcChannel) -> bool {
           if self.auto_confirm {
               return true;
           }
           
           // Use MSPC channel for confirmation prompt
           let prompt = format!("Execute '{}'? (yes/no)", action);
           let router = TerminalInputRouter::new(mspc_channel.clone());
           router.handle_confirmation_prompt(&prompt).await
       }
   }
   ```

4. **Add tests**
   ```rust
   #[tokio::test]
   async fn test_confirmation_flow() {
       // Test confirmation request/response cycle
   }
   ```

5. **Commit**
   ```bash
   git add src/input_router/terminal.rs src/chat/mspc_session.rs
   git commit -m "feat: complete confirmation prompt system"
   ```

---

### Task 4: Implement Input Protection

**Files:**
- Modify: `src/input_router/terminal.rs`
- Modify: `src/chat/mspc_session.rs`

**Steps:**

1. **Add input lock mechanism**
   ```rust
   // In MspcChannel
   pub struct MspcChannel {
       // ... existing fields
       input_lock: Arc<Mutex<bool>>, // True when input is being processed
   }
   
   impl MspcChannel {
       pub async fn acquire_input_lock(&self) -> bool {
           let mut lock = self.input_lock.lock().await;
           if *lock {
               return false; // Input already being processed
           }
           *lock = true;
           true
       }
       
       pub async fn release_input_lock(&self) {
           let mut lock = self.input_lock.lock().await;
           *lock = false;
       }
   }
   ```

2. **Protect terminal input reading**
   ```rust
   async fn read_terminal_input(router: TerminalInputRouter) {
       let stdin = tokio::io::stdin();
       let reader = BufReader::new(stdin);
       let mut lines = reader.lines();
       
       while let Ok(Some(line)) = lines.next_line().await {
           // Check if we should accept input
           if router.channel.acquire_input_lock().await {
               let message = router.parse_input(&line);
               router.send_to_channel(message).await;
               router.channel.release_input_lock().await;
           }
       }
   }
   ```

3. **Handle input during busy periods**
   ```rust
   // In chat loop, when input lock is held:
   } else if mspc_channel.is_user_input(&message) {
       if mspc_channel.acquire_input_lock().await {
           // Process input
           mspc_channel.release_input_lock().await;
       } else {
           // Store for later or show "busy" message
           eprintln!("{} System is busy, please wait...", "⚠️".yellow());
       }
   }
   ```

4. **Add tests**
   ```rust
   #[tokio::test]
   async fn test_input_protection() {
       // Test concurrent input attempts
       // Test input lock acquisition/release
   }
   ```

5. **Commit**
   ```bash
   git add src/mspc/channel.rs src/input_router/terminal.rs src/chat/mspc_session.rs
   git commit -m "feat: implement input protection mechanism"
   ```

---

### Task 5: Integrate with Existing Chat Logic

**Files:**
- Modify: `src/chat/mspc_session.rs`
- Modify: `src/app/repl.rs`

**Steps:**

1. **Complete `execute_chat_turn` function**
   ```rust
   async fn execute_chat_turn(chat: &mut APChat) -> Result<String> {
       // Call existing chat logic
       use crate::chat::session::chat_turn;
       
       let response = chat_turn(
           chat,
           chat.current_model,
           chat.client_config.clone(),
           chat.agents_enabled,
       ).await?;
       
       Ok(response)
   }
   ```

2. **Update REPL to use MSPC chat loop**
   ```rust
   // In run_repl_mode:
   
   // Create MSPC channel
   let mspc_channel = Arc::new(MspcChannel::new(100));
   
   // Start MSPC chat loop
   let chat_result = chat_with_mspc(
       &mut chat,
       mspc_channel,
       None,
   ).await;
   ```

3. **Test full integration**
   ```bash
   cd apchat-main
   cargo run -- --no-agents
   ```

4. **Commit**
   ```bash
   git add src/chat/mspc_session.rs src/app/repl.rs
   git commit -m "feat: integrate MSPC with existing chat logic"
   ```

---

### Task 6: Implement Message History Formatting

**Files:**
- Modify: `src/mspc/channel.rs`

**Steps:**

1. **Implement history formatting for LLM prompts**
   ```rust
   impl MspcChannel {
       pub async fn format_history_for_prompt(&self) -> String {
           let history = self.get_history_for_prompt().await;
           let mut result = String::new();
           
           // Add system message if this is the first interaction
           result.push_str("System: You are APChat, a helpful assistant...\n\n");
           
           for pair in history {
               if !pair.user.is_empty() {
                   result.push_str(&format!("User: {}\n", pair.user));
               }
               if !pair.agent.is_empty() {
                   result.push_str(&format!("Assistant: {}\n", pair.agent));
               }
           }
           
           result
       }
   }
   ```

2. **Update chat loop to use formatted history**
   ```rust
   async fn process_user_input(
       chat: &mut APChat,
       mspc_channel: &MspcChannel,
       user_message: &str,
   ) -> Result<String> {
       // ... existing code
       
       // Add user message to chat
       chat.messages.push(Message {
           role: "user".to_string(),
           content: user_message.to_string(),
           // ...
       });
       
       // Summarize and trim history
       crate::chat::history::summarize_and_trim_history(chat).await?;
       
       // Generate response using existing logic
       let response = execute_chat_turn(chat).await?;
       
       // ...
   }
   ```

3. **Add tests**
   ```rust
   #[tokio::test]
   async fn test_history_formatting() {
       // Test formatting with various history states
   }
   ```

4. **Commit**
   ```bash
   git add src/mspc/channel.rs src/chat/mspc_session.rs
   git commit -m "feat: implement message history formatting"
   ```

---

### Task 7: Test Edge Cases

**Files:**
- Create: `tests/integration/mspc_integration.rs`

**Steps:**

1. **Test interrupt during agent response**
   ```rust
   #[tokio::test]
   async fn test_interrupt_handling() {
       // Simulate interrupt during "thinking"
       // Verify interrupted message is cleaned up
       // Verify interruption marker is added
   }
   ```

2. **Test multiple rapid inputs**
   ```rust
   #[tokio::test]
   async fn test_rapid_inputs() {
       // Send multiple inputs quickly
       // Verify they're processed in order
   }
   ```

3. **Test confirmation during interrupt**
   ```rust
   #[tokio::test]
   async fn test_confirmation_during_interrupt() {
       // Interrupt during confirmation prompt
       // Verify proper handling
   }
   ```

4. **Test message history invariants**
   ```rust
   #[tokio::test]
   async fn test_history_invariants() {
       // Test various scenarios to ensure history always valid
   }
   ```

5. **Commit**
   ```bash
   git add tests/
   git commit -m "test: add edge case tests"
   ```

---

### Task 8: PTY-Based Testing

**Files:**
- Create: `scripts/test_mspc_pty.sh`

**Steps:**

1. **Create PTY test script**
   ```bash
   #!/bin/bash
   
   # Test interrupt handling
   echo "Testing interrupt handling..."
   cargo run -- --no-agents <<< "!stop" &
   sleep 1
   kill $! 2>/dev/null
   
   # Test regular input
   echo "Testing regular input..."
   (echo "Hello" && sleep 1 && echo "World") | cargo run -- --no-agents
   
   # Test command handling
   echo "Testing command handling..."
   cargo run -- --no-agents <<< "/skills"
   ```

2. **Run PTY tests**
   ```bash
   chmod +x scripts/test_mspc_pty.sh
   ./scripts/test_mspc_pty.sh
   ```

3. **Document test results**
   - Create `docs/testing/mspc_pty_results.md`
   - Document pass/fail scenarios
   - Note any issues found

4. **Commit**
   ```bash
   git add scripts/ docs/testing/
   git commit -m "test: add PTY-based testing"
   ```

---

### Task 9: Documentation

**Files:**
- Create: `docs/architecture/mspc-architecture.md`
- Modify: `README.md`
- Modify: `docs/high-level/decouple-input.md`

**Steps:**

1. **Document MSPC architecture**
   ```markdown
   # MSPC Architecture
   
   ## Overview
   The Multi-Stream Processing Channel (MSPC) system decouples input sources from the LLM interaction loop.
   
   ## Components
   - `MspcChannel`: Central message bus
   - `TerminalInputRouter`: Terminal input handler
   - `WebexInputRouter`: Future Webex bot integration
   - `chat_with_mspc`: Main interaction loop
   
   ## Message Flow
   ```
   User Input → Input Router → MSPC Channel → LLM Loop → Response
   ```
   
   ## Interrupt Handling
   - Messages starting with "!" interrupt immediately
   - Interrupted messages are cleaned up
   - Interruption marker added to history
   ```

2. **Update README**
   - Add MSPC section
   - Document new input patterns
   - Note interrupt behavior

3. **Update high-level doc**
   - Mark implementation as complete
   - Document testing results
   - Note any known limitations

4. **Commit**
   ```bash
   git add docs/
   git commit -m "docs: complete MSPC documentation"
   ```

---

## Risk Assessment and Mitigation

### Risks Identified

1. **Message History Corruption**
   - *Impact*: Loss of conversation context
   - *Mitigation*: Implement validation and fixing methods
   - *Status*: Addressed in Task 1

2. **Race Conditions**
   - *Impact*: Input loss or corruption
   - *Mitigation*: Use proper locking (Arc<Mutex<>>)
   - *Status*: Addressed in Task 4

3. **Interrupt Handling Issues**
   - *Impact*: Incomplete or incorrect interruption
   - *Mitigation*: Comprehensive testing of interrupt scenarios
   - *Status*: Addressed in Task 7

4. **Confirmation System Failures**
   - *Impact*: Policy manager bypassed
   - *Mitigation*: Full integration with policy manager
   - *Status*: Addressed in Task 3

5. **Input Clobbering**
   - *Impact*: Concurrent input overwrites
   - *Mitigation*: Input lock mechanism
   - *Status*: Addressed in Task 4

### Technical Debt

**Existing Issues to Avoid:**

1. **Blocking Terminal Input** - The current terminal input reading is blocking. This has been partially addressed with async reading in `mspc_session.rs`, but needs proper integration.

2. **Incomplete Message History** - The history management doesn't enforce all invariants. This is addressed in Task 1.

3. **Fragmented Confirmation Logic** - Confirmation prompts are scattered. Centralized in Task 3.

4. **Lack of Input Protection** - No mechanism to prevent concurrent input clobbering. Addressed in Task 4.

### Migration Strategy

1. **Phase 1**: Verify current implementation (Task 0)
2. **Phase 2**: Enhance core components (Tasks 1-4)
3. **Phase 3**: Integrate with existing logic (Task 5)
4. **Phase 4**: Test and document (Tasks 6-9)

**Rollback Plan**: The MSPC system is isolated enough that if issues arise, we can revert to the original REPL loop while keeping the MSPC infrastructure for future use.

---

## Testing Strategy

### Verification Points

1. **Input Routing**
   - ✅ Messages starting with "!" → InterruptSignal
   - ✅ Messages starting with "/" → Command
   - ✅ Regular messages → UserInput
   - ✅ Empty input → UserInput (empty)

2. **Interrupt Handling**
   - ✅ Interrupts processed immediately
   - ✅ Agent message cleaned up on interrupt
   - ✅ Interruption marker added to history
   - ✅ User can resume after interrupt

3. **Turn-Based Processing**
   - ✅ Regular inputs wait until turn end
   - ✅ Multiple inputs queued properly
   - ✅ Inputs processed in order

4. **Message History**
   - ✅ History starts with user after system messages
   - ✅ History ends with agent (unless interrupted)
   - ✅ Proper user/agent pairing
   - ✅ Interruptions handled correctly

5. **Confirmation Prompts**
   - ✅ Confirmation requests routed through channel
   - ✅ Confirmation responses received through channel
   - ✅ Policy manager integrated
   - ✅ Terminal prompts user for confirmation

6. **Input Protection**
   - ✅ Concurrent inputs don't clobber
   - ✅ Input lock prevents overlap
   - ✅ "Busy" message shown when appropriate

### PTY Test Scenarios

1. **Basic Interrupt Test**
   - Send "!stop" during processing
   - Verify immediate interruption
   - Verify proper cleanup

2. **Regular Input Test**
   - Send multiple regular messages
   - Verify turn-based processing
   - Verify message ordering

3. **Command Test**
   - Send "/skills", "/model", etc.
   - Verify commands processed correctly

4. **Confirmation Test**
   - Trigger confirmation prompt
   - Respond with "yes" and "no"
   - Verify proper handling

5. **Stress Test**
   - Send rapid inputs
   - Verify no data loss
   - Verify proper ordering

### Success Criteria

1. **Functional Requirements**
   - [ ] All input types routed correctly
   - [ ] Interrupts processed immediately
   - [ ] Regular inputs processed at turn end
   - [ ] Message history maintains invariants
   - [ ] Confirmation prompts work correctly
   - [ ] Input protection prevents clobbering

2. **Non-Functional Requirements**
   - [ ] No memory leaks
   - [ ] No deadlocks
   - [ ] Proper error handling
   - [ ] Good performance (no busy waiting)
   - [ ] Clean code structure

3. **Integration Requirements**
   - [ ] Works with existing chat logic
   - [ ] Compatible with policy manager
   - [ ] Compatible with logging system
   - [ ] Compatible with model switching

---

## Execution Plan

**Recommended Approach:** Subagent-Driven (this session)

**Rationale:**
1. The plan is detailed with clear tasks
2. Many tasks are interdependent
3. Need for continuous testing and verification
4. Benefits from incremental review between tasks

**Estimated Duration:** 1-2 days for full implementation

**Resource Requirements:**
1. Rust toolchain
2. Tokio runtime
3. Test environment with PTY access
4. CI/CD for automated testing

---

Plan complete and saved to `docs/plans/2026-01-18-input-decoupling-implementation.md`.

Two execution options:

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

Which approach?