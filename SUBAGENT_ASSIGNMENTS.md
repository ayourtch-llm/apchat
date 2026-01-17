# Subagent Task Assignments for Decoupling Input Feature

## Overview

This document assigns specific tasks to subagents for implementing the decoupled input feature. Each subagent will work independently on their assigned tasks and coordinate through the main repository.

## Subagent Assignments

### Subagent Alpha: Input Channel Infrastructure

**Lead**: [Assign Lead]
**Team**: 2 developers
**Duration**: 3 days
**Priority**: Critical Path

#### Tasks:

1. **Task 1.1: Input Channel Implementation**
   - File: `src/chat/input_channel.rs`
   - Implement `InputChannel<T>` with `tokio::sync::mpsc`
   - Add methods: `new()`, `sender()`, `try_recv()`, `recv_with_timeout()`, `has_pending_interrupts()`, `drain_pending_messages()`
   - Create `InterruptionMessage` struct
   - Tests: Unit tests for all methods

2. **Task 1.2: Terminal Input Listener Refactor**
   - File: `src/terminal/input_listener.rs`
   - Convert to use `tokio::sync::mpsc`
   - Implement non-blocking event polling
   - Add interrupt detection logic
   - Preserve all existing functionality (history, navigation, etc.)
   - Tests: Integration tests with PTY

3. **Task 1.3: APChat State Updates**
   - File: `src/main.rs`
   - Add `input_channel` field to `APChat` struct
   - Add `pending_tool_call` field
   - Add `interruption_flag` field
   - Create `ToolCallInfo` struct
   - Tests: State management tests

4. **Task 2.1: Main Loop Integration**
   - File: `src/app/repl.rs`
   - Create input channel at startup
   - Spawn terminal input listener task
   - Modify main loop to check for interrupts
   - Implement deferred input processing
   - Tests: Basic loop functionality tests

#### Deliverables:
- [ ] Updated `input_channel.rs` with new implementation
- [ ] Updated `input_listener.rs` with async support
- [ ] Updated `main.rs` with new fields
- [ ] Updated `repl.rs` with channel integration
- [ ] Unit tests (80% coverage)
- [ ] Integration tests (basic scenarios)

---

### Subagent Beta: Interruption Handling

**Lead**: [Assign Lead]
**Team**: 2 developers
**Duration**: 4 days
**Priority**: Critical Path

#### Tasks:

1. **Task 3.1: Interruption Detection Logic**
   - File: `src/terminal/input_listener.rs`
   - Implement `is_interrupt_input()` function
   - Implement `extract_interrupt_content()` function
   - Tests: Unit tests for detection logic

2. **Task 3.2: Immediate Interruption Handling**
   - File: `src/app/repl.rs`
   - Implement `handle_interruption()` function
   - Integrate with main loop
   - Add `force_next_response` flag
   - Tests: Interruption scenarios

3. **Task 3.3: Deferred Input Processing**
   - File: `src/app/repl.rs`
   - Implement deferred input queue
   - Implement `process_deferred_inputs()` function
   - Integrate with turn completion
   - Tests: Deferred input scenarios

4. **Task 5.1: Tool Call Cleanup**
   - File: `src/chat/history.rs`
   - Implement `cleanup_pending_tool_calls()` function
   - Handle multiple tool calls
   - Preserve tool call/result pairs when appropriate
   - Tests: Cleanup scenarios

#### Deliverables:
- [ ] Interruption detection logic
- [ ] `handle_interruption()` implementation
- [ ] Deferred input queue
- [ ] Tool call cleanup logic
- [ ] Unit tests (90% coverage)
- [ ] Integration tests (interruption scenarios)

---

### Subagent Gamma: History Management

**Lead**: [Assign Lead]
**Team**: 2 developers
**Duration**: 3 days
**Priority**: High

#### Tasks:

1. **Task 4.1: History Validation Implementation**
   - File: `src/chat/history.rs`
   - Implement `validate_and_fix_history()` function
   - Ensure history starts with user after system
   - Ensure history ends with assistant
   - Handle orphaned tool results
   - Tests: Validation scenarios

2. **Task 4.2: Validation Integration**
   - File: `src/app/repl.rs`
   - Call validation after adding user messages
   - Call validation after handling interruptions
   - Call validation before sending to LLM
   - Tests: Integration scenarios

3. **Task 5.2: Interruption Recovery Tests**
   - Create test suite for interruption scenarios
   - Test: Interruption during tool execution
   - Test: Interruption during LLM response
   - Test: Multiple consecutive interruptions
   - Test: Interruption with no pending tool call
   - Tests: Comprehensive recovery scenarios

4. **Task 6.3: History Validation Tests**
   - Test valid histories (pass through)
   - Test invalid histories (fixed)
   - Test edge cases
   - Tests: Comprehensive validation scenarios

#### Deliverables:
- [ ] `validate_and_fix_history()` implementation
- [ ] Validation integrated at all necessary points
- [ ] Interruption recovery test suite
- [ ] History validation test suite
- [ ] Unit tests (95% coverage)

---

### Subagent Delta: Testing & Integration

**Lead**: [Assign Lead]
**Team**: 2 developers
**Duration**: 5 days
**Priority**: High

#### Tasks:

1. **Task 6.1: PTY Test Infrastructure**
   - File: `tests/pty_tests.rs`
   - Implement PTY-based testing framework
   - Create helper functions for spawning apchat
   - Create helper functions for sending input
   - Create helper functions for reading output
   - Tests: Infrastructure validation

2. **Task 6.2: Input Routing Tests**
   - Test: Normal input (deferred until turn end)
   - Test: Interrupt input (immediate processing)
   - Test: Multiple inputs queued
   - Test: Input during tool execution
   - Tests: All input routing scenarios

3. **Task 6.4: Performance Testing**
   - Measure input latency
   - Measure interruption latency
   - Measure impact on LLM response time
   - Tests: Performance benchmarks

4. **Task 6.5: Stress Testing**
   - High input rate testing
   - Long-running conversation testing
   - Network latency simulation
   - Tests: Stress test scenarios

5. **Task 6.6: Integration Testing**
   - End-to-end test scenarios
   - Test: Complete user workflow
   - Test: Interruption workflow
   - Test: Tool execution workflow
   - Tests: End-to-end scenarios

#### Deliverables:
- [ ] PTY test infrastructure
- [ ] Input routing test suite
- [ ] Performance test results
- [ ] Stress test results
- [ ] Integration test suite
- [ ] Test coverage reports (90%+ overall)

---

## Coordination Plan

### Daily Standups

**Time**: 10:00 AM daily
**Format**: Slack/Teams channel
**Agenda**:
1. Progress update (what was done yesterday)
2. Blockers (what's preventing progress)
3. Plan (what will be done today)
4. Risks (new risks identified)

### Code Reviews

**Process**:
1. Create draft PR with WIP prefix
2. Assign 2 reviewers from other subagents
3. Address feedback within 24 hours
4. Merge after approval from both reviewers

**Expected Turnaround**: 12 hours for initial review

### Integration Points

**Critical Integration Points**:
1. Input channel → Main loop (Alpha → Beta)
2. Interruption handling → History validation (Beta → Gamma)
3. All components → Testing (Alpha/Beta/Gamma → Delta)

**Integration Strategy**:
- Feature branches merged to `dev` branch
- Daily integration builds
- Automated testing on `dev`

### Risk Management

**Risk Tracking**: Shared spreadsheet/document
**Escalation Path**:
1. Team lead
2. Tech lead
3. Project manager

**Risk Categories**:
- 🟢 Low: Can be handled by team
- 🟡 Medium: Needs coordination
- 🔴 High: Needs immediate attention

---

## Task Breakdown Timeline

### Week 1 (Days 1-3): Architecture Foundation

**Alpha Team**:
- Day 1: Input channel implementation
- Day 2: Terminal listener refactor
- Day 3: State updates + main loop integration

**Beta Team**:
- Day 1: Interruption detection logic
- Day 2: Immediate interruption handling
- Day 3: Deferred input processing

**Gamma Team**:
- Day 1: History validation implementation
- Day 2: Validation integration
- Day 3: Recovery test scenarios

**Delta Team**:
- Day 1: PTY test infrastructure
- Day 2: Input routing tests
- Day 3: Performance testing setup

### Week 2 (Days 4-7): Core Functionality

**Alpha Team**:
- Day 4: Integration testing
- Day 5: Bug fixes from integration
- Day 6: Documentation
- Day 7: Buffer tuning

**Beta Team**:
- Day 4: Tool call cleanup
- Day 5: Interruption scenarios testing
- Day 6: Edge case handling
- Day 7: Bug fixes

**Gamma Team**:
- Day 4: History validation testing
- Day 5: Edge case handling
- Day 6: Documentation
- Day 7: Bug fixes

**Delta Team**:
- Day 4: Stress testing
- Day 5: Integration testing
- Day 6: Test coverage analysis
- Day 7: Final test suite

### Week 3 (Days 8-10): Testing & Polishing

**All Teams**:
- Day 8: Cross-subagent testing
- Day 9: Bug fix marathon
- Day 10: Performance tuning

### Week 4 (Days 11-14): Final Validation

**Delta Team**:
- Day 11: Full regression testing
- Day 12: User acceptance testing
- Day 13: Documentation review
- Day 14: Final sign-off

---

## Communication Channels

### Primary Channels:

1. **#apchat-dev** (Slack/Teams): General discussion
2. **#decoupled-input** (Slack/Teams): Feature-specific discussion
3. **GitHub Issues**: Bug tracking
4. **GitHub PRs**: Code reviews

### Escalation Path:

1. **Technical Issues**: #decoupled-input channel
2. **Blockers**: @tech-lead
3. **Urgent Issues**: @project-manager or #urgent channel

### Documentation Updates:

- **Technical Docs**: Update DECOUPLING_INPUT_TECH_SPEC.md
- **User Docs**: Update USER_GUIDE.md
- **API Docs**: Update in code with /// comments
- **Architecture**: Update ARCHITECTURE.md

---

## Success Metrics

### Per Subagent:

**Alpha Team**:
- [ ] All architecture tasks completed
- [ ] 80%+ unit test coverage
- [ ] No critical bugs in foundation

**Beta Team**:
- [ ] All interruption tasks completed
- [ ] 90%+ unit test coverage
- [ ] Interruption scenarios working

**Gamma Team**:
- [ ] All history tasks completed
- [ ] 95%+ unit test coverage
- [ ] Recovery scenarios working

**Delta Team**:
- [ ] All testing tasks completed
- [ ] 90%+ overall test coverage
- [ ] No critical integration issues

### Overall:

- [ ] Feature complete per specification
- [ ] All success criteria met
- [ ] Documentation complete
- [ ] Ready for production deployment

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation | Owner |
|------|-----------|--------|------------|-------|
| Channel deadlocks | Medium | High | Timeout-based polling | Alpha |
| History corruption | Medium | High | Atomic operations, validation | Gamma |
| Performance degradation | Low | Medium | Benchmarking, tuning | Delta |
| Terminal compatibility | Low | Low | Cross-platform testing | Alpha |
| User confusion | Medium | Medium | Clear documentation, UX testing | All |

---

**Document Version**: 1.0
**Created**: 2026-01-17
**Last Updated**: 2026-01-17
