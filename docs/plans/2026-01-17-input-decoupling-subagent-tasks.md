# Input Decoupling - Subagent Task List

This document provides detailed, executable tasks for worker subagents to implement the input decoupling architecture.

## Task Format

Each task is designed to be executed independently by a subagent:

```json
{
  "task_id": "TASK-XX",
  "description": "Detailed task description",
  "files_to_modify": ["file1", "file2"],
  "dependencies": ["TASK-01", "TASK-02"],
  "verification": "How to verify completion",
  "estimated_time": "time estimate",
  "priority": "P0-P3",
  "difficulty": "easy-medium-hard"
}
```

## Phase 1: Infrastructure Setup

### TASK-01: Create MSPC Channel Types
```json
{
  "task_id": "TASK-01",
  "description": "Create apchat-main/src/chat/input_channel.rs with InputMessage, InputChannelConfig, and InputChannel structs. Implement methods: new(), has_pending_messages(), try_recv(), recv_with_timeout(). Use tokio::sync::mpsc for channel implementation.",
  "files_to_modify": ["apchat-main/src/chat/input_channel.rs"],
  "dependencies": [],
  "verification": "Run 'cd apchat-main && cargo check --package apchat' - should compile without errors",
  "estimated_time": "1 hour",
  "priority": "P0",
  "difficulty": "medium"
}
```

### TASK-02: Update Chat Module Exports
```json
{
  "task_id": "TASK-02",
  "description": "Add module declaration and pub use statements for input_channel module in apchat-main/src/chat/mod.rs. Add: pub mod input_channel; and pub use input_channel::{InputMessage, InputChannel, InputChannelConfig};",
  "files_to_modify": ["apchat-main/src/chat/mod.rs"],
  "dependencies": ["TASK-01"],
  "verification": "Run 'cd apchat-main && cargo check --package apchat' - should compile without errors",
  "estimated_time": "15 minutes",
  "priority": "P0",
  "difficulty": "easy"
}
```

### TASK-03: Add Input Channel to APChat State
```json
{
  "task_id": "TASK-03",
  "description": "Add input_channel: Option<InputChannel> field to APChat struct in apchat-main/src/main.rs. Initialize it as None in the new() function.",
  "files_to_modify": ["apchat-main/src/main.rs"],
  "dependencies": ["TASK-02"],
  "verification": "Run 'cd apchat-main && cargo check --package apchat' - should compile without errors",
  "estimated_time": "1 hour",
  "priority": "P0",
  "difficulty": "medium"
}
```

### TASK-04: Create Helper Methods for Input Channel
```json
{
  "task_id": "TASK-04",
  "description": "Implement helper methods in APChat impl block in apchat-main/src/main.rs: initialize_input_channel(), input_channel_receiver(), input_channel_sender(), has_pending_input(), try_recv_input().",
  "files_to_modify": ["apchat-main/src/main.rs"],
  "dependencies": ["TASK-03"],
  "verification": "Run 'cd apchat-main && cargo check --package apchat' - should compile without errors",
  "estimated_time": "1 hour",
  "priority": "P0",
  "difficulty": "medium"
}
```

## Phase 2: Terminal Input Integration

### TASK-05: Create Terminal Input Listener
```json
{
  "task_id": "TASK-05",
  "description": "Create apchat-main/src/terminal/input_listener.rs with TerminalInputListener struct. Implement methods: new(), run(), save_history(). Handle terminal input, detect interruptions (! prefix), and forward to input channel.",
  "files_to_modify": ["apchat-main/src/terminal/input_listener.rs"],
  "dependencies": ["TASK-01"],
  "verification": "Run 'cd apchat-main && cargo check --package apchat' - should compile without errors",
  "estimated_time": "2 hours",
  "priority": "P0",
  "difficulty": "medium"
}
```

### TASK-06: Update Terminal Module Exports
```json
{
  "task_id": "TASK-06",
  "description": "Add module declaration and pub use statements for input_listener module in apchat-main/src/terminal/mod.rs. Add: pub mod input_listener; and pub use input_listener::TerminalInputListener;",
  "files_to_modify": ["apchat-main/src/terminal/mod.rs"],
  "dependencies": ["TASK-05"],
  "verification": "Run 'cd apchat-main && cargo check --package apchat' - should compile without errors",
  "estimated_time": "15 minutes",
  "priority": "P0",
  "difficulty": "easy"
}
```

### TASK-07: Refactor REPL to Use Input Channel
```json
{
  "task_id": "TASK-07",
  "description": "Refactor run_repl_mode function in apchat-main/src/app/repl.rs to use input channel. Initialize input channel, spawn terminal listener as separate task, and modify main loop to check for pending input using has_pending_input() and try_recv_input().",
  "files_to_modify": ["apchat-main/src/app/repl.rs"],
  "dependencies": ["TASK-03", "TASK-05"],
  "verification": "Run 'cd apchat-main && cargo check --package apchat' - should compile without errors",
  "estimated_time": "3 hours",
  "priority": "P0",
  "difficulty": "hard"
}
```

### TASK-08: Create Message Processing Helpers
```json
{
  "task_id": "TASK-08",
  "description": "Create helper functions in apchat-main/src/app/repl.rs before run_repl_mode: process_first_message(), process_user_message(), cleanup_interrupted_messages(), ensure_valid_history_structure(). Handle message history validation and interruption cleanup.",
  "files_to_modify": ["apchat-main/src/app/repl.rs"],
  "dependencies": ["TASK-07"],
  "verification": "Run 'cd apchat-main && cargo check --package apchat' - should compile without errors",
  "estimated_time": "2 hours",
  "priority": "P0",
  "difficulty": "medium"
}
```

## Phase 3: Message History Management

### TASK-09: Add History Validation Functions
```json
{
  "task_id": "TASK-09",
  "description": "Add functions to apchat-main/src/chat/history.rs: validate_and_fix_history(), is_history_valid(). Implement logic to ensure message history starts with user and ends with agent after system messages.",
  "files_to_modify": ["apchat-main/src/chat/history.rs"],
  "dependencies": [],
  "verification": "Run 'cd apchat-main && cargo test' - history-related tests should pass",
  "estimated_time": "2 hours",
  "priority": "P1",
  "difficulty": "medium"
}
```

### TASK-10: Add Interruption Handling Logic
```json
{
  "task_id": "TASK-10",
  "description": "Add cleanup_interrupted_messages() function to apchat-main/src/chat/history.rs. Implement logic to remove incomplete tool-use messages and insert interruption markers. Update exports in mod.rs.",
  "files_to_modify": ["apchat-main/src/chat/history.rs", "apchat-main/src/chat/mod.rs"],
  "dependencies": ["TASK-09"],
  "verification": "Run 'cd apchat-main && cargo test' - tests should pass",
  "estimated_time": "1.5 hours",
  "priority": "P1",
  "difficulty": "medium"
}
```

## Phase 4: Chat Session Integration

### TASK-11: Update Chat Session for Interruptions
```json
{
  "task_id": "TASK-11",
  "description": "Modify the main loop in chat() function in apchat-main/src/chat/session.rs to check for interruptions from input channel. Add cancellation token checking and pending message detection.",
  "files_to_modify": ["apchat-main/src/chat/session.rs"],
  "dependencies": ["TASK-10"],
  "verification": "Run 'cd apchat-main && cargo check --package apchat' - should compile without errors",
  "estimated_time": "2 hours",
  "priority": "P1",
  "difficulty": "medium"
}
```

### TASK-12: Add Loop Continuation Logic
```json
{
  "task_id": "TASK-12",
  "description": "Add should_continue_loop() function to apchat-main/src/chat/session.rs. Implement logic to determine if tool calling loop should continue based on iteration limits, errors, loop detection, and pending inputs.",
  "files_to_modify": ["apchat-main/src/chat/session.rs"],
  "dependencies": ["TASK-11"],
  "verification": "Run 'cd apchat-main && cargo check --package apchat' - should compile without errors",
  "estimated_time": "1.5 hours",
  "priority": "P1",
  "difficulty": "medium"
}
```

## Phase 5: Testing

### TASK-13: Create Unit Tests for Input Channel
```json
{
  "task_id": "TASK-13",
  "description": "Create apchat-main/src/chat/input_channel_tests.rs with unit tests for input channel functionality. Test: channel creation, send/receive, non-blocking receive, interrupt detection. Add module declaration to mod.rs.",
  "files_to_modify": ["apchat-main/src/chat/input_channel_tests.rs", "apchat-main/src/chat/mod.rs"],
  "dependencies": ["TASK-01"],
  "verification": "Run 'cd apchat-main && cargo test input_channel_tests' - all tests should pass",
  "estimated_time": "2 hours",
  "priority": "P1",
  "difficulty": "medium"
}
```

### TASK-14: Create Integration Tests for REPL
```json
{
  "task_id": "TASK-14",
  "description": "Create apchat-main/src/app/repl_tests.rs with integration tests for REPL functionality. Test: REPL initialization, message processing, interruption handling. Add module declaration to mod.rs.",
  "files_to_modify": ["apchat-main/src/app/repl_tests.rs", "apchat-main/src/app/mod.rs"],
  "dependencies": ["TASK-07"],
  "verification": "Run 'cd apchat-main && cargo test repl_tests' - all tests should pass",
  "estimated_time": "2 hours",
  "priority": "P1",
  "difficulty": "medium"
}
```

### TASK-15: Create PTY Testing Example
```json
{
  "task_id": "TASK-15",
  "description": "Create apchat-main/examples/test_input_decoupling.rs as a PTY-based testing example. Implement async main() that tests input decoupling functionality, message flow, and interruption handling.",
  "files_to_modify": ["apchat-main/examples/test_input_decoupling.rs"],
  "dependencies": ["TASK-07"],
  "verification": "Run 'cd apchat-main && cargo run --example test_input_decoupling' - should execute without errors",
  "estimated_time": "2 hours",
  "priority": "P2",
  "difficulty": "medium"
}
```

## Phase 6: Final Integration

### TASK-16: Update Main Entry Point
```json
{
  "task_id": "TASK-16",
  "description": "Ensure input channel is properly initialized in all initialization paths in apchat-main/src/main.rs. Verify that setup_from_cli and similar functions create the input channel with InputChannelConfig::default().",
  "files_to_modify": ["apchat-main/src/main.rs"],
  "dependencies": ["TASK-03", "TASK-04", "TASK-07", "TASK-10", "TASK-12"],
  "verification": "Run 'cd apchat-main && cargo build --release' - should build successfully",
  "estimated_time": "1 hour",
  "priority": "P0",
  "difficulty": "medium"
}
```

### TASK-17: Full Build and Testing
```json
{
  "task_id": "TASK-17",
  "description": "Run full build and test suite. Execute: cargo build --release, cargo test, and manual testing. Verify all functionality works as expected.",
  "files_to_modify": [],
  "dependencies": ["TASK-16"],
  "verification": "All tests pass, release build succeeds, manual testing checklist completed",
  "estimated_time": "1 hour",
  "priority": "P0",
  "difficulty": "easy"
}
```

## Phase 7: Documentation

### TASK-18: Create Architecture Documentation
```json
{
  "task_id": "TASK-18",
  "description": "Create docs/architecture/INPUT_DECOUPLING.md with comprehensive architecture documentation. Include: overview, key components, message flow, interruption handling, message history rules, testing, and future extensions.",
  "files_to_modify": ["docs/architecture/INPUT_DECOUPLING.md"],
  "dependencies": ["TASK-17"],
  "verification": "Documentation is complete, accurate, and reviewed",
  "estimated_time": "2 hours",
  "priority": "P2",
  "difficulty": "easy"
}
```

## Task Execution Guidelines

### For Subagents
1. **Claim a task**: Select one task to execute
2. **Read dependencies**: Ensure all dependencies are complete
3. **Execute**: Follow task description precisely
4. **Verify**: Run verification commands
5. **Test**: Run appropriate tests
6. **Commit**: Atomic commit with descriptive message
7. **Mark complete**: Update task status

### Best Practices
- **Atomic changes**: One task = one commit
- **Verification first**: Always run verification before claiming task is complete
- **Error handling**: If verification fails, investigate and fix
- **Documentation**: Add comments to complex code sections
- **Testing**: Add appropriate unit tests for new functionality

## Dependency Graph

```
TASK-01 → TASK-02 → TASK-03 → TASK-04
                                    └→ TASK-07
TASK-01 → TASK-05 → TASK-06 → TASK-07 → TASK-08
TASK-09 → TASK-10 → TASK-11 → TASK-12
TASK-01 → TASK-13
TASK-07 → TASK-14
TASK-07 → TASK-15
TASK-03 → TASK-16
TASK-04 → TASK-16
TASK-07 → TASK-16
TASK-10 → TASK-16
TASK-12 → TASK-16
TASK-16 → TASK-17 → TASK-18
```

## Parallelization Strategy

### Phase 1 (All parallel)
- TASK-01, TASK-02, TASK-03, TASK-04 can be worked on sequentially

### Phase 2 (Partial parallel)
- TASK-05 and TASK-09 can run in parallel
- TASK-06 depends on TASK-05
- TASK-07 and TASK-08 are sequential

### Phase 3 (Sequential)
- TASK-09 → TASK-10 must be sequential

### Phase 4 (Sequential)
- TASK-11 → TASK-12 must be sequential

### Phase 5 (All parallel)
- TASK-13, TASK-14, TASK-15 can run in parallel

### Phase 6 (Sequential)
- TASK-16 → TASK-17 → TASK-18 must be sequential

## Monitoring and Progress Tracking

### Metrics to Track
- **Tasks Completed**: Count of completed tasks
- **Tests Passing**: Percentage of tests passing
- **Build Status**: Compilation success rate
- **Verification Success**: Tasks passing verification
- **Time Tracking**: Actual vs estimated time

### Progress Reporting
After each task completion, report:
1. Task ID and description
2. Files modified
3. Verification results
4. Time taken
5. Any issues encountered
6. Next tasks recommended

---

*Subagent Task List Version: 1.0*
*Date: 2026-01-17*
