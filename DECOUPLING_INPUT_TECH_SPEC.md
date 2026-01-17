# Technical Specification: Decoupling Input Feature

## Overview

This document provides detailed technical specifications for implementing the decoupled input system in APChat. It includes data structures, algorithms, and implementation details needed by development subagents.

## Data Structures

### 1. InterruptionMessage

```rust
#[derive(Debug, Clone)]
pub struct InterruptionMessage {
    /// The actual input content
    pub content: String,
    
    /// Whether this is an interrupt request (starts with "!")
    pub is_interrupt: bool,
    
    /// Timestamp when message was created
    pub timestamp: std::time::SystemTime,
    
    /// Original input (including "!" prefix if present)
    pub original: String,
}
```

### 2. ToolCallInfo

```rust
#[derive(Debug, Clone)]
pub struct ToolCallInfo {
    /// Index of the assistant message with tool calls
    pub assistant_index: usize,
    
    /// IDs of tool calls being executed
    pub tool_call_ids: Vec<String>,
    
    /// Timestamp when tool execution started
    pub start_time: std::time::SystemTime,
}
```

### 3. InputChannel (Enhanced)

```rust
pub struct InputChannel {
    receiver: tokio::sync::mpsc::Receiver<InterruptionMessage>,
    sender: tokio::sync::mpsc::Sender<InterruptionMessage>,
    buffer_size: usize,
}

impl InputChannel {
    /// Create with default buffer size (100)
    pub fn new() -> Self {
        Self::with_buffer_size(100)
    }
    
    /// Create with custom buffer size
    pub fn with_buffer_size(buffer_size: usize) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(buffer_size);
        Self {
            receiver,
            sender,
            buffer_size,
        }
    }
    
    /// Get sender for this channel
    pub fn sender(&self) -> tokio::sync::mpsc::Sender<InterruptionMessage> {
        self.sender.clone()
    }
    
    /// Try to receive a message without blocking
    pub async fn try_recv(&mut self) -> Option<InterruptionMessage> {
        self.receiver.try_recv().ok()
    }
    
    /// Receive with timeout
    pub async fn recv_with_timeout(&mut self, timeout: Duration) -> Option<InterruptionMessage> {
        match tokio::time::timeout(timeout, self.receiver.recv()).await {
            Ok(Some(msg)) => Some(msg),
            Ok(None) => None, // Channel closed
            Err(_) => None,   // Timeout
        }
    }
    
    /// Check if there are pending interrupt messages
    pub async fn has_pending_interrupts(&mut self) -> bool {
        let msg = self.try_recv().await;
        if msg.is_some() && msg.unwrap().is_interrupt {
            // Put it back if we just want to check
            self.sender().send(msg.unwrap()).await.ok();
            true
        } else {
            false
        }
    }
    
    /// Drain all pending messages (for turn completion)
    pub async fn drain_pending_messages(&mut self) -> Vec<InterruptionMessage> {
        let mut messages = Vec::new();
        while let Some(msg) = self.try_recv().await {
            messages.push(msg);
        }
        messages
    }
}
```

## Algorithms

### 1. Interruption Detection Algorithm

```rust
/// Detect if input should be treated as interruption
pub fn is_interrupt_input(input: &str) -> bool {
    input.trim_start().starts_with('!')
}

/// Extract content from interrupt input
pub fn extract_interrupt_content(input: &str) -> String {
    let trimmed = input.trim_start();
    if trimmed.starts_with('!') {
        trimmed[1..].trim().to_string()
    } else {
        input.trim().to_string()
    }
}
```

### 2. History Validation Algorithm

```rust
/// Validate and fix message history
pub fn validate_and_fix_history(messages: &mut Vec<Message>) -> Result<()> {
    // Step 1: Find all system messages
    let system_indices: Vec<usize> = messages
        .iter()
        .enumerate()
        .filter(|(_, msg)| msg.role == "system")
        .map(|(i, _)| i)
        .collect();
    
    // Step 2: Ensure history after system messages starts with user
    if let Some(&last_system_idx) = system_indices.last() {
        if last_system_idx + 1 < messages.len() {
            let first_after_system = &messages[last_system_idx + 1];
            
            if first_after_system.role != "user" {
                // Move system messages to the end if they're in the middle
                if !system_indices.is_empty() && system_indices[0] != 0 {
                    let system_msgs: Vec<Message> = system_indices
                        .iter()
                        .filter_map(|&i| messages.get(i).cloned())
                        .collect();
                    
                    // Remove system messages
                    let mut new_messages = messages.clone();
                    for &idx in system_indices.iter().rev() {
                        new_messages.remove(idx);
                    }
                    
                    // Add them at the end
                    new_messages.extend(system_msgs);
                    *messages = new_messages;
                } else if first_after_system.role == "assistant" {
                    // Insert user placeholder if assistant comes after system
                    messages.insert(last_system_idx + 1, Message {
                        role: "user".to_string(),
                        content: "(continuing conversation)".to_string(),
                        ..Default::default()
                    });
                }
                // Note: tool messages after system are handled by cleanup below
            }
        }
    }
    
    // Step 3: Ensure history ends with assistant (not tool)
    if let Some(last) = messages.last() {
        if last.role == "tool" {
            // Remove orphaned tool result
            messages.pop();
            
            // Also remove the assistant message that called it if it's now orphaned
            if let Some(last_after_remove) = messages.last() {
                if last_after_remove.role == "assistant" {
                    if last_after_remove.tool_calls.is_some() {
                        messages.pop();
                    }
                }
            }
        }
    }
    
    Ok(())
}
```

### 3. Tool Call Cleanup Algorithm

```rust
/// Cleanup pending tool calls after interruption
pub fn cleanup_pending_tool_calls(messages: &mut Vec<Message>) {
    // Find all assistant messages with tool calls
    let mut tool_assistant_indices: Vec<usize> = messages
        .iter()
        .enumerate()
        .filter(|(_, msg)| msg.role == "assistant" && msg.tool_calls.is_some())
        .map(|(i, _)| i)
        .collect();
    
    // Start with the last one (most recent)
    if let Some(&last_tool_idx) = tool_assistant_indices.last() {
        // Remove the assistant message with tool calls
        messages.remove(last_tool_idx);
        
        // Remove any subsequent tool results
        let mut to_remove = Vec::new();
        for (i, msg) in messages.iter().enumerate().skip(last_tool_idx) {
            if msg.role == "tool" {
                to_remove.push(i);
            } else if msg.role == "assistant" || msg.role == "user" {
                // Stop at next user or assistant message
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

## Main Loop Architecture

### Before (Current):

```rust
loop {
    // 1. Get input (blocking)
    let line = rl.read_line().await?;
    
    // 2. Process input immediately
    process_user_input(&mut chat, &line).await?;
    
    // 3. Get LLM response (blocking)
    let response = get_llm_response(&chat).await?;
    
    // 4. Display response
    display_response(&response);
}
```

### After (New):

```rust
pub async fn run_repl_mode(mut chat: APChat) -> Result<()> {
    // Create input channel
    let input_channel = InputChannel::with_buffer_size(100);
    chat.input_channel = Some(input_channel);
    
    // Spawn terminal input listener
    tokio::spawn({
        let sender = input_channel.sender().clone();
        async move {
            run_terminal_input_listener(sender).await;
        }
    });
    
    loop {
        // 1. Check for interrupts first (non-blocking)
        if let Some(mut input_channel) = chat.input_channel.take() {
            if let Some(msg) = input_channel.try_recv().await {
                if msg.is_interrupt {
                    handle_interruption(&mut chat, msg.content).await?;
                    input_channel = None; // Will be set back in handle_interruption
                    continue;
                }
            }
            chat.input_channel = Some(input_channel);
        }
        
        // 2. Check for normal input (deferred processing)
        let has_deferred_input = check_for_deferred_input(&mut chat).await?;
        
        // 3. Process deferred inputs at turn end
        if has_deferred_input || chat.force_next_response {
            process_deferred_inputs(&mut chat).await?;
            chat.force_next_response = false;
        }
        
        // 4. Only process LLM if we have new user input
        if chat.pending_user_input {
            // Get LLM response with interrupt checking
            let response = get_llm_response_with_interrupts(&mut chat).await?;
            
            // Display response with interrupt checking
            display_response_with_interrupts(&response, &mut chat).await?;
            
            chat.pending_user_input = false;
        }
        
        // Small delay to prevent busy looping
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
```

## Terminal Input Listener

### Current Implementation Issues:

1. Uses `std::sync::mpsc` instead of `tokio::sync::mpsc`
2. Blocks on `crossterm::event::read()`
3. No proper handling of "!" prefix in all cases

### New Implementation:

```rust
pub async fn run_terminal_input_listener(
    sender: tokio::sync::mpsc::Sender<InterruptionMessage>,
) -> Result<()> {
    // Set up terminal
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(terminal::EnterAlternateScreen)?;
    
    let mut current_input = String::new();
    let mut show_prompt = true;
    
    loop {
        if show_prompt {
            stdout.execute(terminal::Clear(terminal::ClearType::FromCursorDown))?;
            write!(stdout, "apchat> ")?;
            stdout.flush()?;
            show_prompt = false;
        }
        
        // Non-blocking event read
        if crossterm::event::poll(Duration::from_millis(50))? {
            if let Event::Key(key_event) = crossterm::event::read()? {
                match key_event.code {
                    KeyCode::Char(c) => {
                        // Handle ! prefix
                        if c == '!' && current_input.is_empty() {
                            current_input.push(c);
                        } else {
                            current_input.push(c);
                        }
                        write!(stdout, "{}", c)?;
                        stdout.flush()?;
                    },
                    KeyCode::Enter => {
                        if !current_input.is_empty() {
                            // Create interruption message
                            let original = current_input.clone();
                            let is_interrupt = is_interrupt_input(&original);
                            let content = if is_interrupt {
                                extract_interrupt_content(&original)
                            } else {
                                original.clone()
                            };
                            
                            let msg = InterruptionMessage {
                                content,
                                is_interrupt,
                                timestamp: SystemTime::now(),
                                original,
                            };
                            
                            // Send to channel
                            sender.send(msg).await.map_err(|e| {
                                io::Error::new(io::ErrorKind::Other, e)
                            })?;
                            
                            // Reset for next input
                            current_input.clear();
                            show_prompt = true;
                        }
                    },
                    // ... other key handlers
                    _ => {}
                }
            }
        }
        
        // Periodic check for channel closure
        if sender.is_closed() {
            break;
        }
        
        // Small delay
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Cleanup
    stdout.execute(terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
```

## Interruption Handling

### handle_interruption() Function

```rust
pub async fn handle_interruption(chat: &mut APChat, input: String) -> Result<()> {
    // 1. Clear interruption flag
    if let Some(flag) = &chat.interruption_flag {
        flag.store(false, Ordering::Relaxed);
    }
    
    // 2. Cleanup any pending tool calls
    cleanup_pending_tool_calls(&mut chat.messages);
    
    // 3. Add user message to history
    chat.messages.push(Message {
        role: "user".to_string(),
        content: input,
        timestamp: Some(SystemTime::now()),
        ..Default::default()
    });
    
    // 4. Validate history
    validate_and_fix_history(&mut chat.messages)?;
    
    // 5. Mark for immediate response
    chat.force_next_response = true;
    chat.pending_user_input = true;
    
    // 6. Clear any pending tool call tracking
    chat.pending_tool_call = None;
    
    Ok(())
}
```

### LLM Response with Interrupts

```rust
pub async fn get_llm_response_with_interrupts(chat: &mut APChat) -> Result<ChatResponse> {
    let request = create_chat_request(chat)?;
    
    // Get streaming response
    let mut stream = chat.model_client.chat_stream(request).await?;
    
    let mut full_response = ChatResponse::default();
    let mut buffer = String::new();
    let mut chunk_count = 0;
    
    while let Some(chunk) = stream.next().await {
        chunk_count += 1;
        
        // Check for interrupts periodically (every 5 chunks or at end)
        if chunk_count % 5 == 0 || chunk.choices.is_empty() {
            if let Some(input_channel) = &mut chat.input_channel {
                if let Some(msg) = input_channel.try_recv().await {
                    if msg.is_interrupt {
                        // Store partial response if needed
                        if !buffer.is_empty() {
                            chat.partial_response = Some(buffer.clone());
                        }
                        
                        handle_interruption(chat, msg.content).await?;
                        return Err(anyhow::anyhow!("Interrupted by user"));
                    }
                }
            }
        }
        
        // Accumulate response
        if let Some(choice) = chunk.choices.first() {
            if let Some(delta) = &choice.delta.content {
                buffer.push_str(delta);
            }
        }
    }
    
    full_response.content = buffer;
    Ok(full_response)
}
```

## Testing Strategy

### Unit Tests

1. **Input Channel Tests**
   - Test message sending/receiving
   - Test interrupt detection
   - Test buffer overflow handling

2. **History Validation Tests**
   - Test valid histories (pass through)
   - Test invalid histories (fixed)
   - Test edge cases (empty history, single message, etc.)

3. **Tool Call Cleanup Tests**
   - Test cleanup with single tool call
   - Test cleanup with multiple tool calls
   - Test cleanup with nested tool calls
   - Test cleanup with no pending tools

### Integration Tests

1. **PTY-Based Tests**
   - Test interruption during LLM response
   - Test deferred input processing
   - Test multiple consecutive interruptions
   - Test interruption during tool execution

2. **Stress Tests**
   - High input rate testing
   - Long-running conversation testing
   - Network latency simulation

### Test Fixtures

```rust
// Example test fixture for interruption testing
#[tokio::test]
async fn test_immediate_interruption() {
    // Setup
    let mut chat = APChat::default();
    let input_channel = InputChannel::new();
    chat.input_channel = Some(input_channel);
    
    // Start a "long" operation
    chat.messages.push(Message {
        role: "assistant".to_string(),
        content: "Thinking...".to_string(),
        tool_calls: Some(vec![apchat_models::ToolCall {
            id: "test-001".to_string(),
            // ... tool call details
        }]),
        ..Default::default()
    });
    
    // Spawn a task that will send an interrupt
    let sender = input_channel.sender().clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let msg = InterruptionMessage {
            content: "stop".to_string(),
            is_interrupt: true,
            timestamp: SystemTime::now(),
            original: "!stop".to_string(),
        };
        sender.send(msg).await.unwrap();
    });
    
    // Simulate long operation with interrupt checking
    let mut interrupted = false;
    for i in 0..100 {
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        if let Some(mut channel) = chat.input_channel.take() {
            if let Some(msg) = channel.try_recv().await {
                if msg.is_interrupt {
                    handle_interruption(&mut chat, msg.content).await.unwrap();
                    interrupted = true;
                    break;
                }
                chat.input_channel = Some(channel);
            }
        }
    }
    
    // Assertions
    assert!(interrupted, "Interruption should have occurred");
    assert_eq!(chat.force_next_response, true);
    assert!(chat.messages.last().unwrap().content == "stop");
    
    // Verify tool call was cleaned up
    let has_tool_calls = chat.messages.iter().any(|m| {
        m.role == "assistant" && m.tool_calls.is_some()
    });
    assert!(!has_tool_calls, "Tool calls should have been cleaned up");
}
```

## Performance Considerations

### Polling Frequency

- Check for interrupts every 5 chunks during LLM response
- Check every 50ms during idle periods
- Check immediately after sending output

### Buffer Sizes

- Input channel: 100 messages (configurable)
- Deferred input queue: 50 messages (configurable)
- Tool call tracking: No hard limit (dynamic)

### Memory Management

- Partial responses stored in memory during interruption
- Cleanup of partial responses after new response generated
- History compaction still occurs independently

## Error Handling

### Error Cases to Handle

1. **Channel Closed Unexpectedly**
   - Detect: `sender.is_closed()` or `receiver.recv()` returns `None`
   - Action: Log error, shutdown gracefully

2. **History Corruption Detected**
   - Detect: Validation fails
   - Action: Attempt repair, log warning

3. **Interruption During Critical Section**
   - Detect: Interrupt flag set during tool execution
   - Action: Mark for cleanup after critical section completes

4. **Multiple Concurrent Interruptions**
   - Detect: Multiple interrupt messages in channel
   - Action: Process in order, only act on first

### Error Propagation

- Non-fatal errors: Log and continue
- Fatal errors: Return `Err` to break main loop
- Interruption errors: Always attempt cleanup before failing

## Migration Path

### Phase 1: Feature Flag

```rust
#[cfg(feature = "decoupled_input")]
{
    // New implementation
}

#[cfg(not(feature = "decoupled_input"))]
{
    // Old implementation
}
```

### Phase 2: A/B Testing

- 10% of users on new system
- Monitor for issues
- Gradually increase to 50%

### Phase 3: Full Migration

- Remove feature flag
- Update documentation
- Announce to users

## Monitoring and Metrics

### Metrics to Track

1. **Input Latency**
   - Time from input to processing
   - Breakdown: terminal → channel → processing

2. **Interruption Latency**
   - Time from "!" to interruption handling
   - Should be < 500ms

3. **Deferred Input Count**
   - Number of inputs waiting for turn end

4. **History Validation Failures**
   - Count of validation attempts and fixes

5. **Tool Call Cleanup Count**
   - Number of tool calls cleaned up after interruption

### Log Events

1. `input_received`: Input type, timestamp
2. `interruption_handled`: Interruption content, latency
3. `history_validated`: Was fix needed?
4. `tool_calls_cleaned`: Count of calls removed
5. `deferred_inputs_processed`: Count at turn end

## Open Questions

1. Should we support multi-character interrupt prefixes (e.g., "!!")?
2. Should interruption clear the entire deferred input queue or just the current one?
3. Should we add a visual indicator when interruptions are pending?
4. Should tool call cleanup be configurable (e.g., save to history)?
5. Should we support interruption of specific tool calls by ID?

---
**Technical Specification Version**: 1.0
**Created**: 2026-01-17
**Last Updated**: 2026-01-17
