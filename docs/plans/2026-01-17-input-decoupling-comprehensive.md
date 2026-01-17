# Comprehensive Input Decoupling Implementation Plan

## 1. Repository Structure Analysis

### Current Structure
The repository is a Rust-based project with a monorepo structure:
- **apchat-main**: Main application crate
- **crates/**: Modular crates for different functionalities
- **docs/**: Documentation and plans
- **skills/**: Skill definitions
- **web/**: Web frontend

### Key Components Identified
- **Input/Output**: Currently in `apchat-main/src/app/repl.rs`
- **Chat State**: In `apchat-main/src/main.rs` (APChat struct)
- **Terminal Handling**: In `apchat-main/src/terminal/`
- **Message History**: In `apchat-main/src/chat/history.rs`
- **Chat Session**: In `apchat-main/src/chat/session.rs`

### Existing Architecture
- Uses tokio for async I/O
- Rustyline for terminal input
- MSPC (multi-producer, single-consumer) pattern already used in other parts
- Message-based architecture with `apchat-models` crate

---

## 2. Detailed Task Breakdown with Dependencies

### Phase 1: Infrastructure Setup (Prerequisites)

#### Task 1.1: Create MSPC Channel Types
- **File**: `apchat-main/src/chat/input_channel.rs` (NEW)
- **Dependencies**: None
- **Duration**: 1 hour
- **Verification**: Compilation succeeds

#### Task 1.2: Update Chat Module Exports
- **File**: `apchat-main/src/chat/mod.rs`
- **Dependencies**: Task 1.1
- **Duration**: 15 minutes
- **Verification**: Compilation succeeds

#### Task 1.3: Add Input Channel to APChat State
- **File**: `apchat-main/src/main.rs`
- **Dependencies**: Task 1.2
- **Duration**: 1 hour
- **Verification**: Compilation succeeds

#### Task 1.4: Create Helper Methods for Input Channel
- **File**: `apchat-main/src/main.rs`
- **Dependencies**: Task 1.3
- **Duration**: 1 hour
- **Verification**: Compilation succeeds

### Phase 2: Terminal Input Integration

#### Task 2.1: Create Terminal Input Listener
- **File**: `apchat-main/src/terminal/input_listener.rs` (NEW)
- **Dependencies**: Task 1.1
- **Duration**: 2 hours
- **Verification**: Compilation succeeds

#### Task 2.2: Update Terminal Module Exports
- **File**: `apchat-main/src/terminal/mod.rs`
- **Dependencies**: Task 2.1
- **Duration**: 15 minutes
- **Verification**: Compilation succeeds

#### Task 2.3: Refactor REPL to Use Input Channel
- **File**: `apchat-main/src/app/repl.rs`
- **Dependencies**: Task 1.3, Task 2.1
- **Duration**: 3 hours
- **Verification**: Compilation succeeds

#### Task 2.4: Create Message Processing Helpers
- **File**: `apchat-main/src/app/repl.rs`
- **Dependencies**: Task 2.3
- **Duration**: 2 hours
- **Verification**: Compilation succeeds

### Phase 3: Message History Management

#### Task 3.1: Add History Validation Functions
- **File**: `apchat-main/src/chat/history.rs`
- **Dependencies**: None
- **Duration**: 2 hours
- **Verification**: Unit tests pass

#### Task 3.2: Add Interruption Handling Logic
- **File**: `apchat-main/src/chat/history.rs`
- **Dependencies**: Task 3.1
- **Duration**: 1.5 hours
- **Verification**: Unit tests pass

### Phase 4: Chat Session Integration

#### Task 4.1: Update Chat Session for Interruptions
- **File**: `apchat-main/src/chat/session.rs`
- **Dependencies**: Task 3.2
- **Duration**: 2 hours
- **Verification**: Compilation succeeds

#### Task 4.2: Add Loop Continuation Logic
- **File**: `apchat-main/src/chat/session.rs`
- **Dependencies**: Task 4.1
- **Duration**: 1.5 hours
- **Verification**: Compilation succeeds

### Phase 5: Testing

#### Task 5.1: Create Unit Tests for Input Channel
- **File**: `apchat-main/src/chat/input_channel_tests.rs` (NEW)
- **Dependencies**: Task 1.1
- **Duration**: 2 hours
- **Verification**: Tests pass

#### Task 5.2: Create Integration Tests for REPL
- **File**: `apchat-main/src/app/repl_tests.rs` (NEW)
- **Dependencies**: Task 2.3
- **Duration**: 2 hours
- **Verification**: Tests pass

#### Task 5.3: Create PTY Testing Example
- **File**: `apchat-main/examples/test_input_decoupling.rs` (NEW)
- **Dependencies**: Task 2.3
- **Duration**: 2 hours
- **Verification**: Example runs successfully

### Phase 6: Final Integration

#### Task 6.1: Update Main Entry Point
- **File**: `apchat-main/src/main.rs`
- **Dependencies**: All previous tasks
- **Duration**: 1 hour
- **Verification**: Full build succeeds

#### Task 6.2: Full Build and Testing
- **File**: None (build process)
- **Dependencies**: Task 6.1
- **Duration**: 1 hour
- **Verification**: Release build succeeds

### Phase 7: Documentation

#### Task 7.1: Create Architecture Documentation
- **File**: `docs/architecture/INPUT_DECOUPLING.md` (NEW)
- **Dependencies**: All implementation tasks
- **Duration**: 2 hours
- **Verification**: Documentation complete and accurate

---

## 3. Risk Assessment and Mitigation Strategies

### Risk 1: Breaking Existing Functionality
- **Impact**: High - users expect current behavior to work
- **Mitigation**: 
  - Incremental refactoring with compilation checks after each task
  - Maintain backward compatibility in APIs
  - Comprehensive testing before final integration
- **Contingency**: Rollback plan with git stashing

### Risk 2: Performance Degradation
- **Impact**: Medium - async channels add overhead
- **Mitigation**:
  - Benchmark before and after
  - Optimize buffer sizes
  - Profile with `flamegraph`
- **Contingency**: Revert to synchronous input if needed

### Risk 3: Message History Corruption
- **Impact**: Critical - can lose conversation context
- **Mitigation**:
  - Robust validation functions
  - Automatic repair mechanisms
  - Comprehensive unit tests
- **Contingency**: Fallback to simple history structure

### Risk 4: Interruption Handling Issues
- **Impact**: High - users need responsive interruptions
- **Mitigation**:
  - Clear specification of interruption behavior
  - Manual testing with various scenarios
  - PTY-based automated tests
- **Contingency**: Implement simpler interruption mechanism

### Risk 5: Resource Leaks
- **Impact**: Medium - memory/handles not properly cleaned up
- **Mitigation**:
  - Proper drop implementations
  - Async task cleanup
  - Valgrind/heap tracking
- **Contingency**: Implement leak detection in tests

### Risk 6: Terminal I/O Issues
- **Impact**: High - basic user interaction broken
- **Mitigation**:
  - Preserve existing rustyline configuration
  - Test on multiple terminal types
  - PTY testing
- **Contingency**: Revert to original terminal handling

---

## 4. Testing Strategy

### Unit Testing
- **Location**: In-file test modules (`*_tests.rs`)
- **Coverage**: 
  - Input channel creation and messaging
  - Interrupt detection
  - Message validation
  - History structure enforcement
- **Framework**: tokio-test for async code

### Integration Testing
- **Location**: `apchat-main/src/app/repl_tests.rs`
- **Coverage**:
  - REPL initialization
  - Input channel integration
  - Message flow end-to-end
  - Interruption handling
- **Framework**: tokio-test with temp directories

### E2E Testing
- **Location**: `apchat-main/examples/test_input_decoupling.rs`
- **Coverage**:
  - Full PTY-based testing
  - Multiple input scenarios
  - Interruption timing
  - History persistence
- **Framework**: Custom async test harness

### Manual Testing Checklist
1. Normal input/output
2. Interruption with "!" prefix
3. Multiple rapid inputs
4. History validation after interruptions
5. Exit commands
6. Ctrl+C handling
7. Ctrl+D handling

### Testing Tools
- `cargo test`: Unit and integration tests
- PTY sessions: For E2E testing
- `flamegraph`: Performance profiling
- `valgrind`: Memory leak detection

---

## 5. Integration Plan

### Development Phases
1. **Phase 1**: Infrastructure (Tasks 1.1-1.4)
   - Focus: Channel implementation
   - Verification: Compilation and basic unit tests

2. **Phase 2**: Terminal Integration (Tasks 2.1-2.4)
   - Focus: Input collection and REPL refactoring
   - Verification: Compilation and manual testing

3. **Phase 3**: Message Management (Tasks 3.1-3.2)
   - Focus: History validation and interruption handling
   - Verification: Unit tests and integration tests

4. **Phase 4**: Session Integration (Tasks 4.1-4.2)
   - Focus: Chat loop modifications
   - Verification: Integration tests

5. **Phase 5**: Testing (Tasks 5.1-5.3)
   - Focus: Comprehensive test coverage
   - Verification: All tests passing

6. **Phase 6**: Final Integration (Tasks 6.1-6.2)
   - Focus: Complete system validation
   - Verification: Full build and manual testing

7. **Phase 7**: Documentation (Task 7.1)
   - Focus: Architecture documentation
   - Verification: Documentation review

### Commit Strategy
- **Atomic Commits**: Each task results in a single commit
- **Commit Messages**: Follow conventional commits
  - `feat:`, `refactor:`, `test:`, `docs:` prefixes
- **Branching**: Feature branch with PR to main
- **Review**: Code review before merging

### Rollback Plan
- **Git**: Use git revert or git reset
- **Backup**: Tag releases before major changes
- **Monitoring**: Watch for regression in tests

### Stakeholder Communication
- **Progress Updates**: After each phase completion
- **Blocking Issues**: Immediate notification
- **Testing Results**: Shared with team
- **Documentation**: Updated continuously

---

## 6. Subagent Execution Plan

Each task can be executed independently by worker subagents:

### Task Format for Subagents
```json
{
  "task": "Task description with specific requirements",
  "files": ["list of files to modify"],
  "dependencies": ["list of dependent tasks"],
  "verification": "how to verify completion",
  "estimated_time": "time estimate"
}
```

### Example Subagent Task
```json
{
  "task": "Create MSPC Channel Types in apchat-main/src/chat/input_channel.rs. Implement InputMessage struct, InputChannelConfig struct, and InputChannel struct with methods: new(), has_pending_messages(), try_recv(), recv_with_timeout().",
  "files": ["apchat-main/src/chat/input_channel.rs"],
  "dependencies": [],
  "verification": "cargo check --package apchat should succeed",
  "estimated_time": "1 hour"
}
```

### Parallelization Opportunities
1. **Phase 1**: All tasks can run in parallel
2. **Phase 2**: Tasks 2.1 and 2.2 can run in parallel
3. **Phase 3**: Tasks 3.1 and 3.2 can run in parallel
4. **Phase 5**: All testing tasks can run in parallel

---

## 7. Success Criteria

### Technical Success
- [ ] All compilation checks pass
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] E2E tests pass
- [ ] Manual testing checklist completed
- [ ] Performance metrics meet or exceed baseline
- [ ] No memory leaks detected
- [ ] Documentation complete

### Functional Success
- [ ] Input decoupling working as specified
- [ ] Interruption handling functional
- [ ] Message history maintained correctly
- [ ] Backward compatibility preserved
- [ ] Error handling robust

### Quality Success
- [ ] Code review completed
- [ ] Code coverage targets met
- [ ] Documentation is clear and accurate
- [ ] Testing is comprehensive
- [ ] Rollback plan verified

---

## 8. Timeline Estimate

### Total Estimated Duration: 20-25 hours

### Breakdown
- **Infrastructure Setup**: 4 hours
- **Terminal Integration**: 7 hours
- **Message Management**: 4 hours
- **Session Integration**: 3.5 hours
- **Testing**: 6 hours
- **Final Integration**: 2 hours
- **Documentation**: 2 hours

### Critical Path
1. Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5 → Phase 6 → Phase 7

---

## 9. References

### Existing Plans
- `docs/plans/2026-01-17-input-decoupling.md`: Original implementation plan

### Related Documentation
- `docs/architecture/REFACTORING_MAP.md`: Refactoring guidelines
- `docs/dev/CODE_REVIEW_CHECKLIST.md`: Code review standards

### Technical References
- Rust Async Book: https://rust-lang.github.io/async-book/
- Tokio Documentation: https://tokio.rs/
- Rustyline Documentation: https://docs.rs/rustyline/

---

## Appendix A: Task Dependency Graph

```
Task 1.1 → Task 1.2 → Task 1.3 → Task 1.4
                                    └→ Task 2.3
Task 1.1 → Task 2.1 → Task 2.2 → Task 2.3 → Task 2.4
Task 3.1 → Task 3.2 → Task 4.1 → Task 4.2
Task 1.1 → Task 5.1
Task 2.3 → Task 5.2
Task 2.3 → Task 5.3
Task 1.3 → Task 6.1
Task 1.4 → Task 6.1
Task 2.3 → Task 6.1
Task 3.2 → Task 6.1
Task 4.2 → Task 6.1
Task 6.1 → Task 6.2 → Task 7.1
```

---

## Appendix B: Testing Checklist

### Before Implementation
- [ ] Baseline performance metrics captured
- [ ] Baseline memory usage captured
- [ ] Existing tests passing

### During Implementation
- [ ] Compilation check after each task
- [ ] Unit tests added for new code
- [ ] Integration tests updated

### After Implementation
- [ ] All tests passing
- [ ] Performance regression testing
- [ ] Memory leak testing
- [ ] Manual testing checklist
- [ ] Documentation review

---

*Implementation Plan Version: 1.0*
*Date: 2026-01-17*
