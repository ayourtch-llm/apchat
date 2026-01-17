# Decoupling the Input Feature Implementation Plan

## Executive Summary

This plan outlines the implementation of a decoupled input/output system for APChat that separates terminal I/O from the LLM interaction loop. The new architecture will use MSPC (multi-producer, single-consumer) channels for input routing, support immediate interruption with "!" prefix, and ensure proper message history validation.

## Current Architecture Analysis

### Key Components Identified:

1. **repl.rs (Main REPL Loop)**
   - Current: Uses blocking `rl.read_line()` in a synchronous loop
   - Location: Line ~262 has the main loop, line ~587 has the input reading loop
   - Problem: Input is tightly coupled with output generation

2. **input_channel.rs**
   - Existing MSPC channel infrastructure with `try_recv()` and `recv_with_timeout()`
   - Already used for asynchronous input handling
   - Needs: Integration with interruption handling

3. **history.rs**
   - Message validation logic needed
   - Ensures history starts with "user" and ends with "agent" after system messages
   - Handles tool_use cleanup for interrupted operations

4. **input_listener.rs**
   - Terminal input handling with crossterm
   - Currently handles "!" prefix detection for interruptions
   - Uses std::sync::mpsc (not tokio)

5. **main.rs (APChat State)**
   - Contains `messages: Vec<Message>` for conversation history
   - Needs to track interruption state and pending tool calls

### Message Structure (Inferred):
- `role`: String field containing "system", "user", "assistant", or "tool"
- Tool calls are represented as assistant messages with `tool_calls` field
- Tool results are represented as "tool" role messages with `tool_call_id`

## Implementation Plan

### Phase 1: Architecture Changes

#### Task 1.1: Create MSPC Input Channel Structure
**Dependencies**: None
**Owner**: Subagent

**Implementation**:
1. Create `InputChannelConfig` and `InputChannel` in `src/chat/input_channel.rs`
2. Use `tokio::sync::mpsc` for async channel
3. Add methods:
   - `new()` - Create channel with configurable buffer size
   - `sender()` - Get cloneable sender
   - `try_recv()` - Non-blocking receive
   - `recv_with_timeout()` - Receive with timeout
   - `has_pending_interrupt()` - Check for "!" prefixed messages

#### Task 1.2: Refactor Terminal Input Listener
**Dependencies**: Task 1.1
**Owner**: Subagent

**Implementation**:
1. Update `src/terminal/input_listener.rs` to:
   - Use `tokio::sync::mpsc` instead of `std::sync::mpsc`
   - Detect "!" prefix and mark messages accordingly
   - Support both blocking and non-blocking modes
2. Add `InterruptionMessage` struct:
   ```rust
   pub struct InterruptionMessage {
       pub content: String,
       pub is_interrupt: bool,
       pub timestamp: SystemTime,
   }
   ```

#### Task 1.3: Update APChat State Structure
**Dependencies**: Task 1.1
**Owner**: Subagent

**Implementation**:
1. Add to `APChat` struct in `src/main.rs`:
   ```rust
   pub input_channel: Option<InputChannel<InterruptionMessage>>,
   pub pending_tool_call: Option<ToolCallInfo>,
   pub interruption_flag: AtomicBool,
   ```
2. Create `ToolCallInfo` struct to track active tool calls

### Phase 2: MSPC Channel Integration

#### Task 2.1: Integrate Input Channel with Main Loop
**Dependencies**: Tasks 1.1, 1.2, 1.3
**Owner**: Subagent

**Implementation**:
1. Modify `run_repl_mode()` in `src/app/repl.rs`:
   - Create input channel at startup
   - Spawn terminal input listener in separate task
   - Modify main loop to check channel frequently:
     ```rust
     loop {
         // Check for pending interrupts first
         if let Some(interrupt_msg) = input_channel.try_recv().await {
             if interrupt_msg.is_interrupt {
                 handle_interruption(&mut chat, interrupt_msg.content).await;
             }
         }
         
         // Normal processing...
     }
     ```

#### Task 2.2: Implement Input Polling Mechanism
**Dependencies**: Task 2.1
**Owner**: Subagent

**Implementation**:
1. Add polling logic to check channel during LLM response streaming:
   ```rust
   while let Some(chunk) = stream.next().await {
       print_chunk(&chunk);
       // Check for interrupts every N chunks or timeout
       if should_check_for_interrupts() {
           if let Some(interrupt) = input_channel.try_recv().await {
               handle_interruption(interrupt).await;
           }
       }
   }
   ```

### Phase 3: Interruption Handling

#### Task 3.1: Implement Interruption Detection
**Dependencies**: Task 2.1
**Owner**: Subagent

**Implementation**:
1. In terminal input listener:
   - Detect "!" at start of input
   - Set `is_interrupt: true` on message
2. In main loop:
   - Check `is_interrupt` flag immediately
   - If interrupt, break current processing

#### Task 3.2: Implement Immediate Interruption Logic
**Dependencies**: Task 3.1
**Owner**: Subagent

**Implementation**:
1. Create `handle_interruption()` function:
   ```rust
   async fn handle_interruption(chat: &mut APChat, input: String) {
       // Clear interruption flag
       chat.interruption_flag.store(false, Ordering::Relaxed);
       
       // Remove any pending tool_use from history
       cleanup_pending_tool_calls(&mut chat.messages);
       
       // Add user message to history
       chat.messages.push(Message {
           role: "user".to_string(),
           content: input,
           ..Default::default()
       });
       
       // Force immediate response
       chat.force_next_response = true;
   }
   ```

#### Task 3.3: Implement Deferred Input Logic
**Dependencies**: Task 3.1
**Owner**: Subagent

**Implementation**:
1. Add queue for non-interrupt inputs
2. Process deferred inputs after turn completion:
   ```rust
   async fn process_deferred_inputs(chat: &mut APChat) {
       while let Some(msg) = input_channel.try_recv().await {
           if !msg.is_interrupt {
               chat.messages.push(Message {
                   role: "user".to_string(),
                   content: msg.content,
                   ..Default::default()
               });
           }
       }
   }
   ```

### Phase 4: Message History Validation

#### Task 4.1: Implement History Validation
**Dependencies**: None
**Owner**: Subagent

**Implementation**:
1. Add to `src/chat/history.rs`:
   ```rust
   pub fn validate_and_fix_history(messages: &mut Vec<Message>) -> Result<()> {
       // Ensure history starts with user after system messages
       let mut system_end_index = 0;
       for (i, msg) in messages.iter().enumerate() {
           if msg.role == "system" {
               system_end_index = i;
           }
       }
       
       // After system messages, must start with user
       if system_end_index + 1 < messages.len() {
           let first_after_system = &messages[system_end_index + 1];
           if first_after_system.role != "user" {
               // Move system messages to end if needed
               // Or insert user message placeholder
           }
       }
       
       // Must end with assistant (not tool_use)
       if let Some(last) = messages.last() {
           if last.role == "tool" {
               // Remove orphaned tool result
               messages.pop();
           }
       }
       
       Ok(())
   }
   ```

#### Task 4.2: Integrate Validation with Main Loop
**Dependencies**: Tasks 4.1, 2.1
**Owner**: Subagent

**Implementation**:
1. Call `validate_and_fix_history()`:
   - After adding user messages
   - After handling interruptions
   - Before sending to LLM

### Phase 5: Interruption Recovery

#### Task 5.1: Implement Tool Call Cleanup
**Dependencies**: Task 4.1
**Owner**: Subagent

**Implementation**:
1. Add to `src/chat/history.rs`:
   ```rust
   pub fn cleanup_pending_tool_calls(messages: &mut Vec<Message>) {
       // Find last assistant message with tool_calls
       let mut last_tool_assistant = None;
       for (i, msg) in messages.iter().enumerate() {
           if msg.role == "assistant" && msg.tool_calls.is_some() {
               last_tool_assistant = Some(i);
           }
       }
       
       if let Some(idx) = last_tool_assistant {
           // Remove the assistant message with tool calls
           messages.remove(idx);
           
           // Remove any subsequent tool results
           let mut to_remove = Vec::new();
           for (i, msg) in messages.iter().enumerate().skip(idx) {
               if msg.role == "tool" {
                   to_remove.push(i);
               } else if msg.role == "assistant" || msg.role == "user" {
                   break;
               }
           }
           
           // Remove in reverse order to maintain indices
           for &i in to_remove.iter().rev() {
               messages.remove(i);
           }
       }
   }
   ```

#### Task 5.2: Test Interruption Scenarios
**Dependencies**: Tasks 5.1, 6.1
**Owner**: Subagent

**Implementation**:
1. Create test cases for:
   - Interruption during tool execution
   - Interruption during LLM response
   - Multiple consecutive interruptions
   - Interruption with no pending tool call

### Phase 6: Testing Strategy

#### Task 6.1: Create PTY-Based Test Infrastructure
**Dependencies**: None
**Owner**: Subagent

**Implementation**:
1. Create `tests/pty_tests.rs`:
   ```rust
   use std::process::{Command, Stdio};
   use std::io::{Write, Read};
   use std::time::Duration;
   
   #[test]
   fn test_interrupt_during_response() {
       let mut child = Command::new("./target/debug/apchat")
           .stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .spawn()
           .expect("Failed to spawn apchat");
       
       let mut stdin = child.stdin.take().expect("Failed to open stdin");
       let mut stdout = child.stdout.take().expect("Failed to open stdout");
       
       // Send initial prompt
       stdin.write_all(b"test prompt\n").expect("Failed to write");
       
       // Wait for LLM to start responding
       std::thread::sleep(Duration::from_millis(500));
       
       // Send interrupt
       stdin.write_all(b"!stop\n").expect("Failed to write interrupt");
       
       // Verify interruption occurred
       // ...
   }
   ```

#### Task 6.2: Test Input Routing
**Dependencies**: Task 6.1
**Owner**: Subagent

**Implementation**:
1. Test scenarios:
   - Normal input (deferred until turn end)
   - Interrupt input (immediate processing)
   - Multiple inputs queued
   - Input during tool execution

#### Task 6.3: Test History Validation
**Dependencies**: Task 6.1
**Owner**: Subagent

**Implementation**:
1. Test scenarios:
   - History starting with user after system
   - History ending with assistant
   - Orphaned tool results cleaned up
   - Interrupted tool calls removed

## Risk Assessment

### High Risk Items:

1. **Deadlocks in Channel Communication**
   - Mitigation: Use timeouts on channel operations
   - Testing: Stress test with high input rates

2. **History Corruption During Interruption**
   - Mitigation: Atomic operations on message vector
   - Testing: Comprehensive interruption tests

3. **Performance Impact of Frequent Polling**
   - Mitigation: Configure polling frequency
   - Testing: Benchmark with different settings

4. **Terminal State Management**
   - Mitigation: Proper cleanup in input listener
   - Testing: Test across different terminal types

### Low Risk Items:

1. **Backward Compatibility** - New feature, not breaking change
2. **Channel Buffer Sizing** - Configurable, can be tuned
3. **Message Validation Logic** - Pure function, easy to test

## Integration Plan

### Integration Order:

1. **Phase 1**: Architecture changes (isolated, low risk)
2. **Phase 2**: MSPC channel integration (test in isolation)
3. **Phase 3**: Interruption handling (critical path)
4. **Phase 4**: History validation (canary in dev)
5. **Phase 5**: Interruption recovery (critical path)
6. **Phase 6**: Testing (parallel with other phases)

### Rollback Strategy:

1. Feature flags for new components
2. Maintain old input path as backup
3. Database snapshots before integration
4. A/B testing in staging

### Success Criteria:

1. **Functional**:
   - [ ] Inputs starting with "!" interrupt immediately (100% of test cases)
   - [ ] Other inputs defer until turn end (100% of test cases)
   - [ ] History always valid after operations (100% of test cases)
   - [ ] Interrupted tool calls cleaned up properly (100% of test cases)

2. **Performance**:
   - [ ] Input latency < 100ms for 95% of inputs
   - [ ] Interruption latency < 500ms for 95% of cases
   - [ ] No measurable impact on LLM response time

3. **Reliability**:
   - [ ] No deadlocks in 24-hour stress test
   - [ ] No history corruption in 1000 interruption test cycles
   - [ ] Graceful handling of edge cases (network issues, etc.)

4. **User Experience**:
   - [ ] Clear visual feedback for interruptions
   - [ ] Smooth transition between interrupted and new context
   - [ ] No unexpected behavior in normal operation

## Task Breakdown for Worker Subagents

### Subagent 1: Input Channel Infrastructure
- Tasks: 1.1, 1.2, 1.3, 2.1
- Duration: 3 days
- Dependencies: None

### Subagent 2: Interruption Handling
- Tasks: 3.1, 3.2, 3.3, 5.1
- Duration: 4 days
- Dependencies: Subagent 1 completion

### Subagent 3: History Management
- Tasks: 4.1, 4.2, 5.2, 6.3
- Duration: 3 days
- Dependencies: Subagent 1 completion

### Subagent 4: Testing & Integration
- Tasks: 6.1, 6.2
- Duration: 5 days
- Dependencies: All other subagents

## Timeline

- **Week 1**: Architecture changes and input channel infrastructure
- **Week 2**: MSPC integration and interruption handling
- **Week 3**: History validation and recovery logic
- **Week 4**: Comprehensive testing and bug fixing
- **Week 5**: Integration, performance tuning, and final validation

## Communication Plan

1. **Daily Standups**: Each subagent reports progress
2. **Code Reviews**: All PRs reviewed by at least 2 team members
3. **Integration Demos**: Weekly integration testing sessions
4. **Risk Board**: Track and mitigate risks in real-time

## Documentation Requirements

1. **Architecture Diagram**: Visual representation of new flow
2. **API Documentation**: Public interfaces for new components
3. **User Guide**: How to use interruption feature
4. **Troubleshooting Guide**: Common issues and resolutions
5. **Test Results**: Comprehensive test coverage reports

---
**Plan Version**: 1.0
**Created**: 2026-01-17
**Last Updated**: 2026-01-17
