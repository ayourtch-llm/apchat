# Input Decoupling Implementation Plan

## Executive Summary

This plan outlines the implementation of the "Decoupling the input" feature, which will decouple terminal input/output from the LLM interaction loop, enabling flexible input sources and responsive interruption handling.

## 1. Current State Analysis

### Existing Infrastructure
- **Input Channel**: `apchat-main/src/chat/input_channel.rs` - Basic MSPC channel implementation
- **Terminal Listener**: `apchat-main/src/terminal/input_listener.rs` - Terminal input handling
- **REPL Loop**: `apchat-main/src/app/repl.rs` - Main interaction loop
- **Message History**: `apchat-main/src/chat/history.rs` - Conversation management
- **APChat State**: `apchat-main/src/main.rs` - Application state

### Key Components Identified
1. Input channel infrastructure (MSPC pattern)
2. Terminal input listener using rustyline
3. Main REPL interaction loop
4. Message history with validation logic
5. Chat session management

## 2. Implementation Strategy

### Core Requirements
1. **Input Decoupling**: Separate terminal I/O from LLM loop
2. **Interrupt Handling**: "!" prefix for immediate interruptions
3. **Deferred Input**: Normal input waits until turn completion
4. **Message Validation**: History must start with "user" and end with "agent"
5. **Interruption Recovery**: Clean up interrupted tool_use messages

### Architecture Design

#### Input Flow
```
Terminal Input → Input Channel (MSPC) → Input Processor → REPL Loop
                                      ↑
                                  Interrupts
```

#### Message Validation Flow
```
1. Check last message role
2. If interrupted tool_use: remove it
3. Validate history structure
4. Insert bogus message if needed
```

## 3. Task Breakdown

### Phase 1: Infrastructure Enhancement

**Task 1.1: Enhance Input Channel Types**
- File: `apchat-main/src/chat/input_channel.rs`
- Add interrupt flag to InputMessage
- Add priority level (high/normal)
- Add timestamp and source tracking

**Task 1.2: Update Chat Module Exports**
- File: `apchat-main/src/chat/mod.rs`
- Export enhanced input channel types
- Add validation helper functions

**Task 1.3: Add Input Processor**
- File: `apchat-main/src/app/input_processor.rs` (NEW)
- Process incoming messages
- Handle interruption logic
- Validate message structure

**Task 1.4: Create History Validation**
- File: `apchat-main/src/chat/history.rs`
- Add `validate_history()` function
- Add `fix_interrupted_history()` function
- Add `insert_bogus_message()` helper

### Phase 2: Terminal Integration

**Task 2.1: Enhance Terminal Listener**
- File: `apchat-main/src/terminal/input_listener.rs`
- Add interrupt detection ("!")
- Set priority flags
- Maintain readline history

**Task 2.2: Update Terminal Module**
- File: `apchat-main/src/terminal/mod.rs`
- Export enhanced listener
- Add configuration options

**Task 2.3: Refactor REPL Loop**
- File: `apchat-main/src/app/repl.rs`
- Add input channel checking
- Implement interrupt handling
- Add deferred input queue

**Task 2.4: Add Interruption State**
- File: `apchat-main/src/main.rs`
- Add interruption flag to APChat
- Add deferred input queue
- Add turn completion tracking

### Phase 3: Message Management

**Task 3.1: Implement History Validation**
- File: `apchat-main/src/chat/history.rs`
- Validate after each message
- Fix interrupted sequences
- Maintain user/agent pattern

**Task 3.2: Add Interruption Recovery**
- File: `apchat-main/src/chat/history.rs`
- Detect interrupted tool_use
- Remove problematic messages
- Insert recovery markers

**Task 3.3: Update Chat Session**
- File: `apchat-main/src/chat/session.rs`
- Add interruption hooks
- Update turn tracking
- Maintain history integrity

### Phase 4: Testing

**Task 4.1: Unit Tests for Input Channel**
- File: `apchat-main/src/chat/input_channel_tests.rs`
- Test interrupt handling
- Test message prioritization
- Test channel reliability

**Task 4.2: Integration Tests for REPL**
- File: `apchat-main/src/app/repl_tests.rs`
- Test input decoupling
- Test interruption timing
- Test history validation

**Task 4.3: PTY Testing Framework**
- File: `apchat-main/examples/test_input_decoupling.rs`
- Test normal input
- Test immediate interruptions
- Test deferred input
- Test history integrity

### Phase 5: Final Integration

**Task 5.1: Update Main Entry Point**
- File: `apchat-main/src/main.rs`
- Initialize enhanced infrastructure
- Wire up all components

**Task 5.2: Full System Testing**
- Verify compilation
- Run all tests
- Manual testing
- Performance validation

**Task 5.3: Documentation Update**
- File: `docs/architecture/INPUT_DECOUPLING.md`
- Architecture overview
- Usage guide
- Troubleshooting

## 4. Risk Assessment

### Risk 1: Breaking Existing Functionality
- **Impact**: High
- **Mitigation**: 
  - Incremental refactoring
  - Comprehensive test coverage
  - Backward compatibility layer
- **Contingency**: Rollback plan

### Risk 2: Performance Degradation
- **Impact**: Medium
- **Mitigation**:
  - Benchmark before/after
  - Optimize channel buffer sizes
  - Profile with flamegraph
- **Contingency**: Revert to synchronous

### Risk 3: Message History Corruption
- **Impact**: Critical
- **Mitigation**:
  - Robust validation
  - Automatic repair
  - Comprehensive tests
- **Contingency**: Fallback structure

### Risk 4: Interruption Handling Issues
- **Impact**: High
- **Mitigation**:
  - Clear specification
  - Manual testing
  - PTY-based tests
- **Contingency**: Simpler mechanism

### Risk 5: Resource Leaks
- **Impact**: Medium
- **Mitigation**:
  - Proper drop implementations
  - Async task cleanup
  - Memory leak detection
- **Contingency**: Implement detection

## 5. Testing Strategy

### Unit Testing
- Location: In-file test modules
- Coverage:
  - Input channel operations
  - Interrupt detection
  - Message validation
  - History structure

### Integration Testing
- Location: `apchat-main/src/app/repl_tests.rs`
- Coverage:
  - REPL initialization
  - Input channel integration
  - Message flow
  - Interruption handling

### E2E Testing
- Location: `apchat-main/examples/test_input_decoupling.rs`
- Coverage:
  - Full PTY-based testing
  - Multiple input scenarios
  - Interruption timing
  - History persistence

### Manual Testing Checklist
1. Normal input/output
2. Interruption with "!" prefix
3. Multiple rapid inputs
4. History validation after interruptions
5. Exit commands
6. Ctrl+C handling
7. Ctrl+D handling

## 6. Integration Plan

### Development Phases
1. **Phase 1**: Infrastructure (Tasks 1.1-1.4)
   - Focus: Enhanced channel implementation
   - Verification: Compilation and unit tests

2. **Phase 2**: Terminal Integration (Tasks 2.1-2.4)
   - Focus: Input collection and REPL refactoring
   - Verification: Compilation and manual testing

3. **Phase 3**: Message Management (Tasks 3.1-3.3)
   - Focus: History validation and interruption handling
   - Verification: Unit and integration tests

4. **Phase 4**: Testing (Tasks 4.1-4.3)
   - Focus: Comprehensive test coverage
   - Verification: All tests passing

5. **Phase 5**: Final Integration (Tasks 5.1-5.3)
   - Focus: Complete system validation
   - Verification: Full build and manual testing

### Commit Strategy
- **Atomic Commits**: One task per commit
- **Commit Messages**: Conventional commits
- **Branching**: Feature branch with PR to main
- **Review**: Code review before merging

### Rollback Plan
- **Git**: Use git revert or git reset
- **Backup**: Tag releases before major changes
- **Monitoring**: Watch for regression in tests

## 7. Success Criteria

### Technical Success
- [ ] All compilation checks pass
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] E2E tests pass
- [ ] Manual testing checklist completed
- [ ] Performance metrics meet baseline
- [ ] No memory leaks detected

### Functional Success
- [ ] Input decoupling working as specified
- [ ] Interruption handling functional
- [ ] Message history maintained correctly
- [ ] Backward compatibility preserved
- [ ] Error handling robust

### Quality Success
- [ ] Code review completed
- [ ] Code coverage targets met
- [ ] Documentation clear and accurate
- [ ] Testing comprehensive

## 8. Timeline Estimate

### Total Estimated Duration: 30-40 hours

### Breakdown
- **Infrastructure**: 6 hours
- **Terminal Integration**: 10 hours
- **Message Management**: 8 hours
- **Testing**: 8 hours
- **Final Integration**: 4 hours
- **Documentation**: 4 hours

## 9. Subagent Execution Plan

Each task should be executed by worker subagents with:
- Clear task description
- Specific files to modify
- Dependencies listed
- Verification criteria
- Estimated time

### Example Task Format
```json
{
  "task": "Task description with specific requirements",
  "files": ["list of files"],
  "dependencies": ["dependent tasks"],
  "verification": "how to verify",
  "estimated_time": "time"
}
```

## 10. References

### Existing Plans
- `docs/plans/2026-01-17-input-decoupling-comprehensive.md`
- `docs/plans/2026-01-17-input-decoupling.md`

### Technical References
- Rust Async Book
- Tokio Documentation
- Rustyline Documentation
- MSPC Pattern Best Practices
