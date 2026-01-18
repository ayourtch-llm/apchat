# Issue 103: Message History Validation and Interruption Handling

## Summary
The message history needs validation to ensure proper user/agent sequence after interruptions. When an agent message is interrupted, it should be removed or replaced with a bogus message.

## Location
- File: `src/mspc/channel.rs`
- Function: `process_message`
- File: `src/chat/mspc_session.rs`
- Function: `handle_interrupt`

## Current Behavior
The message history management is basic and doesn't handle interruption scenarios properly. When an interrupt occurs mid-conversation, the incomplete agent message remains in history, which can confuse the LLM.

## Expected Behavior
The system should:
1. Detect incomplete agent messages
2. Remove or replace them with a bogus message (e.g., "== interrupted ==")
3. Ensure history always starts with user message after system messages
4. Ensure history always ends with agent message (or bogus interrupt marker)

## Impact
Without proper history validation, interrupted conversations can result in:
- Confused LLM responses
- Incomplete or incorrect tool calls
- Broken conversation flow
- History corruption

## Suggested Implementation

### Step 1: Add history validation function

```rust
// In src/mspc/channel.rs

pub async fn validate_and_fix_history(
    history: &mut Vec<ChatMessage>,
    is_interrupted: bool,
) {
    if history.is_empty() {
        return;
    }

    // Ensure history starts with user message after system messages
    let mut start_idx = 0;
    for (i, msg) in history.iter().enumerate() {
        if msg.role != "system" {
            start_idx = i;
            break;
        }
    }

    // Check if first non-system message is user
    if start_idx < history.len() {
        if history[start_idx].role != "user" {
            // Insert a user message or move to correct position
            let first_msg = history.remove(start_idx);
            let placeholder = ChatMessage {
                role: "user".to_string(),
                content: "== context started ==".to_string(),
                timestamp: first_msg.timestamp,
                tool_calls: vec![],
            };
            history.insert(start_idx, placeholder);
            history.insert(start_idx + 1, first_msg);
        }
    }

    // Handle interruption
    if is_interrupted {
        // Find and remove incomplete agent message
        for i in (0..history.len()).rev() {
            if history[i].role == "assistant" {
                // Check if this is an incomplete message (has tool_calls but no final response)
                if !history[i].tool_calls.is_empty() {
                    let interrupted_msg = ChatMessage {
                        role: "assistant".to_string(),
                        content: "== interrupted ==".to_string(),
                        timestamp: history[i].timestamp,
                        tool_calls: vec![],
                    };
                    history[i] = interrupted_msg;
                    break;
                }
            }
        }
    }

    // Ensure history ends with agent message
    let last_msg = history.last().map(|m| m.role.as_str());
    if last_msg != Some("assistant") && last_msg != Some("user") {
        // Add a user message if needed
        history.push(ChatMessage {
            role: "user".to_string(),
            content: "".to_string(),
            timestamp: Utc::now(),
            tool_calls: vec![],
        });
    }
}
```

### Step 2: Update message processing

```rust
// In src/mspc/channel.rs - process_message function

pub async fn process_message(
    &self,
    message: MSPCMessage,
    history: &mut Vec<ChatMessage>,
) -> Result<MSPCMessage, Box<dyn std::error::Error>> {
    let is_interrupt = message.is_interrupt();
    
    // Process the message
    let result = match message {
        // ... existing match arms ...
    };
    
    // Validate and fix history after processing
    validate_and_fix_history(history, is_interrupt).await;
    
    Ok(result)
}
```

### Step 3: Update interruption handling

```rust
// In src/chat/mspc_session.rs - handle_interrupt function

pub async fn handle_interrupt(&mut self, interrupt: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Mark that we're in interrupt state
    self.interrupt_pending = true;
    
    // Add interrupt message to history
    self.history.push(ChatMessage {
        role: "user".to_string(),
        content: interrupt.to_string(),
        timestamp: Utc::now(),
        tool_calls: vec![],
    });
    
    // Validate history
    mspc_channel::validate_and_fix_history(&mut self.history, true).await;
    
    // Clear any partial tool execution
    self.tool_execution_state = None;
    
    Ok(())
}
```

### Step 4: Add helper methods to ChatMessage

```rust
// In src/chat/mspc_session.rs or appropriate module

impl ChatMessage {
    pub fn is_tool_call(&self) -> bool {
        !self.tool_calls.is_empty()
    }
    
    pub fn is_incomplete(&self) -> bool {
        self.role == "assistant" && self.is_tool_call() && self.content.is_empty()
    }
}
```

## Resolution

This issue will be resolved by implementing:

1. History validation function
2. Interruption detection and handling
3. Proper sequence enforcement
4. Bogus message insertion

**Files Modified:**
- `src/mspc/channel.rs`
- `src/chat/mspc_session.rs`

**Testing:**
- Test regular conversation flow
- Test interruption mid-tool-call
- Test interruption mid-response
- Test history validation
- Verify LLM receives correct history

---
*Created: 2026-01-18*
