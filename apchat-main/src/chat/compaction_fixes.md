# Context Compaction Issues - Root Cause Analysis

## Problem 1: System Messages Are Lost

### Current Behavior
The code uses `chat.messages.first().cloned()` to get the system message, which only gets the FIRST message. However, there can be multiple system messages in front.

### Issues Identified

1. **Line ~263 in intelligent_compaction:**
```rust
let system_message = chat.messages.first().cloned();
```
This only gets the first message, but there may be multiple system messages.

2. **Line ~278-280:**
```rust
let to_summarize: Vec<Message> = chat.messages
    .iter()
    .skip(1) // Skip system
    .take(final_cutoff.saturating_sub(1))
    .cloned()
    .collect();
```
This hardcodes `skip(1)` which skips only ONE system message, losing any additional ones.

3. **Rebuild logic (lines ~404-415):**
```rust
let mut new_history = vec![];

if let Some(sys_msg) = system_message {
    new_history.push(sys_msg);
}

// Add recent messages (including recent tool context) - NO system message added!
new_history.extend(recent_messages);
```
Only one system message is added back.

## Problem 2: LLM Loses Purpose/Context After Compaction

### Root Causes

1. **Summary prompt doesn't include system messages**: The summary request only summarizes the middle portion (from index 1 to cutoff), which excludes the system messages entirely.

2. **Summary prompt focuses on tool iteration**: The current prompt asks to summarize for the "current tool iteration" but doesn't emphasize:
   - The overall task purpose
   - What activities/_steps were done
   - What the next steps should be

3. **Summary is discarded**: After getting the summary from the LLM, the code doesn't actually ADD the summary back to the conversation history. The summary is just used to estimate context but isn't preserved.

4. **Recent messages may not include purpose**: The `recent_messages` extracted may not include the system messages or the high-level task context.

## Proposed Fixes

### Fix 1: Handle Multiple System Messages

1. Create a function to identify all system messages at the start:
```rust
fn get_system_messages(messages: &[Message]) -> Vec<Message> {
    messages.iter()
        .take_while(|m| m.role == "system")
        .cloned()
        .collect()
}
```

2. Change `skip(1)` to `skip(system_messages.len())` to skip ALL system messages.

3. In rebuild logic, add ALL system messages, not just one.

### Fix 2: Improve Summary Prompt

The summary prompt should:
1. Include all system messages (preserved at the start)
2. Emphasize task purpose, completed activities, and next steps
3. Preserve the summary in the conversation history (as a user/assistant message)

### Fix 3: Add Summary to History

After getting the summary, add it as a message to preserve context:
```rust
// Add summary as a message to preserve context
new_history.push(Message {
    role: "user".to_string(),
    content: format!("CONVERSATION SUMMARY (previous context compressed):\n{}", summary),
    tool_calls: None,
    tool_call_id: None,
    name: None,
    reasoning: None,
});
```

## Implementation Plan

1. Create helper function `find_system_message_count()` to count leading system messages
2. Update `intelligent_compaction()` to:
   - Extract all system messages
   - Skip all system messages when building `to_summarize`
   - Preserve all system messages in rebuilt history
   - Add summary message to preserve context
3. Update `summarize_and_trim_history()` similarly
4. Update summary prompts to focus on purpose, activities, and next steps
