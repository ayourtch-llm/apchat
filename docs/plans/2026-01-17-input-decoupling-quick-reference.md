# 🚀 Input Decoupling - Quick Reference Guide for Subagents

## 📋 At a Glance

**Project**: Input Decoupling Implementation
**Goal**: Decouple terminal I/O from LLM loop using MSPC channels
**Duration**: 20-25 hours
**Tasks**: 18 atomic tasks
**Documentation**: 4 comprehensive documents

---

## 🎯 Key Concepts

### MSPC Channels
- **Multi-producer, Single-consumer** pattern
- **Purpose**: Route input from multiple sources (terminal, web, bots)
- **Implementation**: `tokio::sync::mpsc`
- **Key Types**:
  - `InputMessage`: content + metadata
  - `InputChannelConfig`: buffer size settings
  - `InputChannel`: sender/receiver endpoints

### Interruption Handling
- **Trigger**: Messages starting with `!`
- **Behavior**: Stop current LLM response, clean up history
- **Marker**: `== interrupted ==` in message history
- **Priority**: Immediate processing over deferred input

### Message History Rules
1. After system messages → first content must be "user"
2. Normal operation → must end with "agent" (no incomplete tool calls)
3. After interruption → can end with "user" message

---

## 📁 File Structure

### New Files to Create
```
apchat-main/src/chat/input_channel.rs              (TASK-01)
apchat-main/src/terminal/input_listener.rs          (TASK-05)
apchat-main/src/chat/input_channel_tests.rs        (TASK-13)
apchat-main/src/app/repl_tests.rs                  (TASK-14)
apchat-main/examples/test_input_decoupling.rs    (TASK-15)
docs/architecture/INPUT_DECOUPLING.md            (TASK-18)
```

### Files to Modify
```
apchat-main/src/chat/mod.rs                       (TASK-02)
apchat-main/src/main.rs                            (TASK-03, TASK-04, TASK-16)
apchat-main/src/terminal/mod.rs                   (TASK-06)
apchat-main/src/app/repl.rs                       (TASK-07, TASK-08)
apchat-main/src/chat/history.rs                   (TASK-09, TASK-10)
apchat-main/src/chat/session.rs                   (TASK-11, TASK-12)
apchat-main/src/app/mod.rs                       (TASK-14)
apchat-main/src/chat/mod.rs                      (TASK-13)
```

---

## 🔧 Common Patterns

### Creating MSPC Channel
```rust
use tokio::sync::mpsc;

let (sender, receiver) = mpsc::channel(config.buffer_size);
let channel = InputChannel { sender, receiver };
```

### Sending Messages
```rust
let msg = InputMessage {
    content: line,
    is_interrupt: line.trim().starts_with('!'),
    timestamp: std::time::Instant::now(),
};

sender.send(msg).await.unwrap();
```

### Receiving Messages (Non-blocking)
```rust
if chat.has_pending_input().await {
    if let Some(input_msg) = chat.try_recv_input().await {
        // Process message
    }
}
```

### Receiving Messages (Timeout)
```rust
use tokio::time;

match time::timeout(Duration::from_millis(50), receiver.recv()).await {
    Ok(Some(msg)) => process_message(msg),
    Ok(None) => channel_closed(),
    Err(_) => timeout(),
}
```

### Checking for Interrupts
```rust
if input_msg.is_interrupt {
    cleanup_interrupted_messages(chat);
    // Process interrupt immediately
}
```

---

## ⚙️ Development Setup

### Prerequisites
```bash
# Install Rust (stable)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Update dependencies
cd apchat-main
cargo update
```

### Build and Test
```bash
# Check compilation
cargo check --package apchat

# Run unit tests
cargo test

# Run specific test module
cargo test input_channel_tests

# Build release
cargo build --release
```

### PTY Testing
```bash
# Run example
cargo run --example test_input_decoupling

# Test manually
cargo run --release
```

---

## 📊 Task Quick Reference

### Phase 1: Infrastructure (4 tasks, 4h)
| Task | Description | Verification |
|------|-------------|--------------|
| TASK-01 | Create MSPC channel types | `cargo check` |
| TASK-02 | Update chat module exports | `cargo check` |
| TASK-03 | Add input channel to APChat state | `cargo check` |
| TASK-04 | Create helper methods | `cargo check` |

### Phase 2: Terminal Integration (4 tasks, 7h)
| Task | Description | Verification |
|------|-------------|--------------|
| TASK-05 | Create terminal input listener | `cargo check` |
| TASK-06 | Update terminal module exports | `cargo check` |
| TASK-07 | Refactor REPL to use channel | `cargo check` |
| TASK-08 | Create message processing helpers | `cargo check` |

### Phase 3: Message Management (2 tasks, 4h)
| Task | Description | Verification |
|------|-------------|--------------|
| TASK-09 | Add history validation functions | `cargo test` |
| TASK-10 | Add interruption handling logic | `cargo test` |

### Phase 4: Session Integration (2 tasks, 3.5h)
| Task | Description | Verification |
|------|-------------|--------------|
| TASK-11 | Update chat session | `cargo check` |
| TASK-12 | Add loop continuation logic | `cargo check` |

### Phase 5: Testing (3 tasks, 6h)
| Task | Description | Verification |
|------|-------------|--------------|
| TASK-13 | Create unit tests | `cargo test input_channel_tests` |
| TASK-14 | Create integration tests | `cargo test repl_tests` |
| TASK-15 | Create PTY example | `cargo run --example test_input_decoupling` |

### Phase 6: Final Integration (2 tasks, 2h)
| Task | Description | Verification |
|------|-------------|--------------|
| TASK-16 | Update main entry point | `cargo build --release` |
| TASK-17 | Full build and testing | All tests pass |

### Phase 7: Documentation (1 task, 2h)
| Task | Description | Verification |
|------|-------------|--------------|
| TASK-18 | Create architecture docs | Documentation complete |

---

## 🔍 Common Issues & Fixes

### Compilation Errors

**Error**: `cannot find type InputMessage`
- **Cause**: Module not exported
- **Fix**: Add to `apchat-main/src/chat/mod.rs`:
  ```rust
  pub use input_channel::{InputMessage, InputChannel, InputChannelConfig};
  ```

**Error**: `no method named has_pending_input`
- **Cause**: Helper methods not implemented
- **Fix**: Implement in `APChat` impl block

**Error**: `use of undeclared type InputChannel`
- **Cause**: Module declaration missing
- **Fix**: Add to `mod.rs`:
  ```rust
  pub mod input_channel;
  ```

### Runtime Issues

**Issue**: Channel receiver hangs
- **Cause**: No sender or sender dropped
- **Fix**: Ensure sender is cloned and stored

**Issue**: Messages lost
- **Cause**: Buffer too small
- **Fix**: Increase buffer size in config

**Issue**: Interrupts not detected
- **Cause**: `is_interrupt` not set correctly
- **Fix**: Check `line.trim().starts_with('!')`

**Issue**: History validation fails
- **Cause**: Invalid message sequence
- **Fix**: Use `validate_and_fix_history()` function

---

## 🧪 Testing Quick Reference

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_send_and_receive() {
        let channel = InputChannel::new(InputChannelConfig::default());
        let msg = InputMessage {
            content: "test".to_string(),
            is_interrupt: false,
            timestamp: std::time::Instant::now(),
        };
        
        channel.sender.send(msg).await.unwrap();
        let received = channel.receiver.recv().await.unwrap();
        assert_eq!(received.content, "test");
    }
}
```

### Integration Tests
```rust
#[test]
fn test_input_message_structure() {
    let msg = InputMessage {
        content: "!interrupt".to_string(),
        is_interrupt: true,
        timestamp: std::time::Instant::now(),
    };
    
    assert_eq!(msg.content, "!interrupt");
    assert!(msg.is_interrupt);
}
```

### Manual Testing Checklist
```bash
# Test normal input
cargo run --release
> hello world

# Test interruption
> !stop

# Test multiple rapid inputs
echo -e "line1\nline2\nline3" | cargo run --release

# Test exit commands
> exit
> quit

# Test Ctrl+C (manual)
# Test Ctrl+D (manual)
```

---

## 📚 Documentation Quick Reference

### Architecture Documentation Template
```markdown
# Input Decoupling Architecture

## Overview
- Goal: Decouple I/O from LLM loop
- Pattern: MSPC channels
- Interruption: `!`-prefixed messages

## Components
1. Input Channel
2. Terminal Input Listener
3. APChat State Extension
4. Message Processing Helpers
5. History Validation
6. Chat Session Integration

## Message Flow
```
User → Listener → Channel → REPL → LLM → Response
```

## Interruption Flow
```
User → Listener → Channel → Cleanup → Process
```

## History Rules
- Starts with "user" after system
- Ends with "agent" normally
- Ends with "user" after interruption
```

---

## ⏱️ Time Estimates

### Per Task
| Task Type | Time Estimate |
|-----------|--------------|
| Simple (exports, mods) | 15-30 min |
| Medium (structs, methods) | 1-2 hours |
| Complex (refactoring, integration) | 2-3 hours |
| Testing | 2 hours |
| Documentation | 2 hours |

### Phase Time
| Phase | Duration |
|-------|----------|
| 1. Infrastructure | 4h |
| 2. Terminal Integration | 7h |
| 3. Message Management | 4h |
| 4. Session Integration | 3.5h |
| 5. Testing | 6h |
| 6. Final Integration | 2h |
| 7. Documentation | 2h |

---

## 💡 Pro Tips

### For Fast Execution
1. **Start with TASK-01**: Creates foundation for other tasks
2. **Use existing patterns**: Copy from other parts of codebase
3. **Test frequently**: Catch issues early
4. **Document as you go**: Add comments to complex code
5. **Verify after each task**: Don't skip verification steps

### For Quality
1. **Add unit tests**: For every new function
2. **Check edge cases**: Empty input, long input, special characters
3. **Test interruptions**: At different points in flow
4. **Validate history**: After every modification
5. **Profile performance**: Before and after changes

### For Debugging
1. **Add logging**: `println!` or proper logging macro
2. **Check channel state**: Sender/receiver not dropped
3. **Validate messages**: Correct structure and content
4. **Test manually**: Before automating
5. **Review diffs**: Before committing

---

## 🎯 Task Execution Checklist

### Before Starting
- [ ] Read task description carefully
- [ ] Check all dependencies complete
- [ ] Verify current code compiles
- [ ] Understand verification criteria
- [ ] Estimate time required

### During Execution
- [ ] Create necessary files
- [ ] Implement required functionality
- [ ] Add appropriate comments
- [ ] Write unit tests if needed
- [ ] Test changes manually

### Before Completion
- [ ] Run verification commands
- [ ] Fix any issues found
- [ ] Update exports if needed
- [ ] Check compilation
- [ ] Run relevant tests

### After Completion
- [ ] Commit changes atomically
- [ ] Update task status
- [ ] Report progress
- [ ] Identify next tasks
- [ ] Document any issues

---

## 📞 Need Help?

### Documentation
- `docs/plans/2026-01-17-input-decoupling-comprehensive.md` - Full plan
- `docs/plans/2026-01-17-input-decoupling-subagent-tasks.md` - Task details
- `docs/plans/2026-01-17-input-decoupling-validation.md` - Validation
- `docs/dev/CODE_REVIEW_CHECKLIST.md` - Coding standards

### Code Examples
- Existing MSPC usage in codebase
- Tokio async patterns
- Rustyline terminal handling

### Testing Guides
- `skills/test-driven-development/SKILL.md`
- `skills/verification-before-completion/SKILL.md`

---

*Quick Reference Guide Version: 1.0*
*Date: 2026-01-17*
*For: Subagent Execution*
