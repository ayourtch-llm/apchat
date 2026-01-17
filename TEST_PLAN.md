# Comprehensive Test Plan: Decoupling Input Feature

## Test Strategy

### Testing Approach

1. **Unit Testing**: Test individual components in isolation
2. **Integration Testing**: Test component interactions
3. **System Testing**: Test complete user workflows
4. **Performance Testing**: Measure latency and throughput
5. **Stress Testing**: Test under heavy load
6. **Regression Testing**: Ensure no existing functionality broken

### Test Environment

**Development**: Local development machines
**Staging**: Dedicated test server
**Production**: Canary deployment (10% of users)

### Test Coverage Goals

- **Unit Tests**: 90%+ coverage
- **Integration Tests**: 85%+ coverage
- **End-to-End Tests**: 70%+ coverage
- **Overall**: 90%+ total test coverage

## Test Classification

### 1. Unit Tests

#### Input Channel Tests

**File**: `tests/unit/input_channel.rs`

```rust
#[tokio::test]
async fn test_input_channel_creation() {
    let channel = InputChannel::new();
    assert!(channel.sender().is_some());
}

#[tokio::test]
async fn test_send_and_receive() {
    let channel = InputChannel::new();
    let sender = channel.sender();
    
    let msg = InterruptionMessage {
        content: "test".to_string(),
        is_interrupt: false,
        timestamp: SystemTime::now(),
        original: "test".to_string(),
    };
    
    sender.send(msg.clone()).await.unwrap();
    
    let mut receiver = channel.into_receiver();
    let received = receiver.recv().await.unwrap();
    
    assert_eq!(received.content, msg.content);
    assert_eq!(received.is_interrupt, msg.is_interrupt);
}

#[tokio::test]
async fn test_try_recv_empty() {
    let channel = InputChannel::new();
    let mut receiver = channel.into_receiver();
    
    let result = receiver.try_recv().await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_interrupt_detection() {
    assert!(is_interrupt_input("!test"));
    assert!(is_interrupt_input("! "));
    assert!(!is_interrupt_input("test"));
    assert!(!is_interrupt_input("!")); // Edge case: just "!"
}

#[tokio::test]
async fn test_extract_interrupt_content() {
    assert_eq!(extract_interrupt_content("!test"), "test");
    assert_eq!(extract_interrupt_content("!  test"), "test");
    assert_eq!(extract_interrupt_content("test"), "test");
}
```

#### History Validation Tests

**File**: `tests/unit/history_validation.rs`

```rust
#[test]
fn test_validate_valid_history() {
    let mut messages = vec![
        Message {
            role: "system".to_string(),
            content: "System prompt".to_string(),
            ..Default::default()
        },
        Message {
            role: "user".to_string(),
            content: "User question".to_string(),
            ..Default::default()
        },
        Message {
            role: "assistant".to_string(),
            content: "Assistant answer".to_string(),
            ..Default::default()
        },
    ];
    
    let result = validate_and_fix_history(&mut messages);
    assert!(result.is_ok());
    assert_eq!(messages.len(), 3);
}

#[test]
fn test_validate_history_with_orphaned_tool() {
    let mut messages = vec![
        Message {
            role: "system".to_string(),
            content: "System prompt".to_string(),
            ..Default::default()
        },
        Message {
            role: "user".to_string(),
            content: "User question".to_string(),
            ..Default::default()
        },
        Message {
            role: "assistant".to_string(),
            content: "Assistant answer".to_string(),
            ..Default::default()
        },
        Message {
            role: "tool".to_string(),
            content: "Tool result".to_string(),
            ..Default::default()
        },
    ];
    
    let result = validate_and_fix_history(&mut messages);
    assert!(result.is_ok());
    assert_eq!(messages.len(), 3); // Orphaned tool result removed
}

#[test]
fn test_cleanup_pending_tool_calls() {
    let mut messages = vec![
        Message {
            role: "user".to_string(),
            content: "User question".to_string(),
            ..Default::default()
        },
        Message {
            role: "assistant".to_string(),
            content: "Assistant answer".to_string(),
            tool_calls: Some(vec![ToolCall {
                id: "call-001".to_string(),
                name: "test_tool".to_string(),
                arguments: "{}".to_string(),
            }]),
            ..Default::default()
        },
        Message {
            role: "tool".to_string(),
            content: "Tool result".to_string(),
            tool_call_id: Some("call-001".to_string()),
            ..Default::default()
        },
    ];
    
    cleanup_pending_tool_calls(&mut messages);
    
    assert_eq!(messages.len(), 1); // Both assistant and tool removed
    assert_eq!(messages[0].role, "user");
}
```

### 2. Integration Tests

#### Input Routing Tests

**File**: `tests/integration/input_routing.rs`

```rust
#[tokio::test]
async fn test_normal_input_deferred() {
    // Setup
    let mut chat = APChat::default();
    let input_channel = InputChannel::new();
    chat.input_channel = Some(input_channel);
    
    // Send normal input
    let sender = input_channel.sender().clone();
    sender.send(InterruptionMessage {
        content: "normal input".to_string(),
        is_interrupt: false,
        timestamp: SystemTime::now(),
        original: "normal input".to_string(),
    }).await.unwrap();
    
    // Process deferred inputs
    process_deferred_inputs(&mut chat).await.unwrap();
    
    // Verify
    assert_eq!(chat.messages.len(), 1);
    assert_eq!(chat.messages[0].role, "user");
    assert_eq!(chat.messages[0].content, "normal input");
}

#[tokio::test]
async fn test_interrupt_immediate() {
    // Setup
    let mut chat = APChat::default();
    let input_channel = InputChannel::new();
    chat.input_channel = Some(input_channel);
    
    // Send interrupt
    let sender = input_channel.sender().clone();
    sender.send(InterruptionMessage {
        content: "stop".to_string(),
        is_interrupt: true,
        timestamp: SystemTime::now(),
        original: "!stop".to_string(),
    }).await.unwrap();
    
    // Handle interruption
    let mut channel = chat.input_channel.take().unwrap();
    if let Some(msg) = channel.try_recv().await {
        if msg.is_interrupt {
            handle_interruption(&mut chat, msg.content).await.unwrap();
        }
    }
    chat.input_channel = Some(channel);
    
    // Verify
    assert_eq!(chat.messages.len(), 1);
    assert_eq!(chat.messages[0].role, "user");
    assert_eq!(chat.messages[0].content, "stop");
    assert!(chat.force_next_response);
}

#[tokio::test]
async fn test_multiple_inputs_queued() {
    // Setup
    let mut chat = APChat::default();
    let input_channel = InputChannel::new();
    chat.input_channel = Some(input_channel);
    
    // Send multiple normal inputs
    let sender = input_channel.sender().clone();
    sender.send(InterruptionMessage {
        content: "first".to_string(),
        is_interrupt: false,
        timestamp: SystemTime::now(),
        original: "first".to_string(),
    }).await.unwrap();
    
    sender.send(InterruptionMessage {
        content: "second".to_string(),
        is_interrupt: false,
        timestamp: SystemTime::now(),
        original: "second".to_string(),
    }).await.unwrap();
    
    // Process deferred inputs
    process_deferred_inputs(&mut chat).await.unwrap();
    
    // Verify
    assert_eq!(chat.messages.len(), 2);
    assert_eq!(chat.messages[0].content, "first");
    assert_eq!(chat.messages[1].content, "second");
}
```

### 3. PTY-Based Tests

#### Terminal Input Tests

**File**: `tests/pty/terminal_input.rs`

```rust
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
    
    // Wait for LLM to start responding (simulated)
    std::thread::sleep(Duration::from_millis(500));
    
    // Send interrupt
    stdin.write_all(b"!stop\n").expect("Failed to write interrupt");
    
    // Verify interruption occurred
    let mut output = String::new();
    stdout.read_to_string(&mut output).expect("Failed to read output");
    
    assert!(output.contains("stop"));
    assert!(output.contains("apchat> ")); // Prompt should return
}

#[test]
fn test_deferred_input_processing() {
    let mut child = Command::new("./target/debug/apchat")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn apchat");
    
    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let mut stdout = child.stdout.take().expect("Failed to open stdout");
    
    // Send initial prompt
    stdin.write_all(b"first message\n").expect("Failed to write");
    
    // Wait for turn to complete
    std::thread::sleep(Duration::from_millis(1000));
    
    // Send second message (should be deferred)
    stdin.write_all(b"second message\n").expect("Failed to write");
    
    // Verify both messages processed
    let mut output = String::new();
    stdout.read_to_string(&mut output).expect("Failed to read output");
    
    assert!(output.contains("first message"));
    assert!(output.contains("second message"));
}

#[test]
fn test_multiple_consecutive_interruptions() {
    let mut child = Command::new("./target/debug/apchat")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn apchat");
    
    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let mut stdout = child.stdout.take().expect("Failed to open stdout");
    
    // Send initial prompt
    stdin.write_all(b"start\n").expect("Failed to write");
    
    // Wait for LLM to start responding
    std::thread::sleep(Duration::from_millis(500));
    
    // Send first interrupt
    stdin.write_all(b"!first\n").expect("Failed to write");
    
    // Wait briefly
    std::thread::sleep(Duration::from_millis(200));
    
    // Send second interrupt
    stdin.write_all(b"!second\n").expect("Failed to write");
    
    // Verify second interrupt took effect
    let mut output = String::new();
    stdout.read_to_string(&mut output).expect("Failed to read output");
    
    assert!(output.contains("second"));
    // Should not contain "first" as it was interrupted
}
```

### 4. Performance Tests

#### Latency Tests

**File**: `tests/performance/latency.rs`

```rust
#[tokio::test]
async fn test_input_latency() {
    let mut chat = APChat::default();
    let input_channel = InputChannel::new();
    chat.input_channel = Some(input_channel);
    
    let sender = input_channel.sender().clone();
    
    // Measure 100 input operations
    let mut total_latency = Duration::from_secs(0);
    for i in 0..100 {
        let start = Instant::now();
        
        sender.send(InterruptionMessage {
            content: format!("test {}", i),
            is_interrupt: false,
            timestamp: SystemTime::now(),
            original: format!("test {}", i),
        }).await.unwrap();
        
        let mut channel = chat.input_channel.take().unwrap();
        while channel.try_recv().await.is_none() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        chat.input_channel = Some(channel);
        
        let latency = Instant::now() - start;
        total_latency += latency;
    }
    
    let avg_latency = total_latency / 100;
    assert!(avg_latency < Duration::from_millis(100), 
            "Average input latency should be < 100ms, got {:?}", avg_latency);
}

#[tokio::test]
async fn test_interruption_latency() {
    let mut chat = APChat::default();
    let input_channel = InputChannel::new();
    chat.input_channel = Some(input_channel);
    
    let sender = input_channel.sender().clone();
    
    // Simulate long operation
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        sender.send(InterruptionMessage {
            content: "stop".to_string(),
            is_interrupt: true,
            timestamp: SystemTime::now(),
            original: "!stop".to_string(),
        }).await.unwrap();
    });
    
    let start = Instant::now();
    let mut interrupted = false;
    
    for _ in 0..1000 {
        tokio::time::sleep(Duration::from_millis(1)).await;
        
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
    
    let latency = Instant::now() - start;
    assert!(interrupted, "Should have been interrupted");
    assert!(latency < Duration::from_millis(500),
            "Interruption latency should be < 500ms, got {:?}", latency);
}
```

### 5. Stress Tests

#### Load Tests

**File**: `tests/stress/load.rs`

```rust
#[tokio::test]
async fn test_high_input_rate() {
    let mut chat = APChat::default();
    let input_channel = InputChannel::new();
    chat.input_channel = Some(input_channel);
    
    let sender = input_channel.sender().clone();
    
    // Spawn 10 tasks sending inputs concurrently
    let mut handles = vec![];
    for i in 0..10 {
        let sender = sender.clone();
        let handle = tokio::spawn(async move {
            for j in 0..100 {
                sender.send(InterruptionMessage {
                    content: format!("task {} msg {}", i, j),
                    is_interrupt: false,
                    timestamp: SystemTime::now(),
                    original: format!("task {} msg {}", i, j),
                }).await.unwrap();
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
        handles.push(handle);
    }
    
    // Process all inputs
    let start = Instant::now();
    let mut processed = 0;
    while processed < 1000 {
        if let Some(mut channel) = chat.input_channel.take() {
            while let Some(msg) = channel.try_recv().await {
                if !msg.is_interrupt {
                    processed += 1;
                }
            }
            chat.input_channel = Some(channel);
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    
    let duration = Instant::now() - start;
    assert!(duration < Duration::from_secs(30),
            "Should process 1000 messages in < 30s, took {:?}", duration);
    
    // Wait for all senders to finish
    for handle in handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_long_running_conversation() {
    let mut chat = APChat::default();
    let input_channel = InputChannel::new();
    chat.input_channel = Some(input_channel);
    
    let sender = input_channel.sender().clone();
    
    // Simulate long conversation
    for i in 0..1000 {
        sender.send(InterruptionMessage {
            content: format!("message {}", i),
            is_interrupt: false,
            timestamp: SystemTime::now(),
            original: format!("message {}", i),
        }).await.unwrap();
        
        // Process deferred input
        process_deferred_inputs(&mut chat).await.unwrap();
        
        // Validate history after each message
        validate_and_fix_history(&mut chat.messages).unwrap();
        
        // Simulate LLM response
        chat.messages.push(Message {
            role: "assistant".to_string(),
            content: format!("response {}", i),
            ..Default::default()
        });
        
        // Check memory usage
        let size = calculate_conversation_size(&chat.messages);
        assert!(size < 10_000_000, // 10MB limit
                "Conversation size should stay under 10MB, got {}", size);
    }
}
```

### 6. Regression Tests

#### Existing Functionality Tests

**File**: `tests/regression/existing_functionality.rs`

```rust
#[test]
fn test_normal_repl_functionality() {
    // Test that existing REPL functionality still works
    // This would spawn apchat and test basic commands
    // Implementation depends on actual REPL commands
    
    assert!(true, "Placeholder for regression test");
}

#[test]
fn test_tool_execution() {
    // Test that tool execution still works
    // This would verify tool calls are properly handled
    
    assert!(true, "Placeholder for regression test");
}

#[test]
fn test_history_management() {
    // Test that existing history management works
    // This would verify compaction, summarization, etc.
    
    assert!(true, "Placeholder for regression test");
}
```

## Test Execution Plan

### Test Execution Order

1. **Unit Tests**: Run first, fastest feedback
2. **Integration Tests**: Run second, verify component interactions
3. **Regression Tests**: Run third, ensure no existing functionality broken
4. **Performance Tests**: Run fourth, verify performance goals
5. **Stress Tests**: Run fifth, verify robustness
6. **PTY Tests**: Run sixth, verify end-to-end workflows

### Test Environments

| Test Type | Environment | Frequency |
|-----------|-------------|-----------|
| Unit Tests | Local | After every commit |
| Integration Tests | Local/CI | After every PR |
| Regression Tests | CI | After every PR |
| Performance Tests | Staging | Daily |
| Stress Tests | Staging | Weekly |
| PTY Tests | Staging | Daily |

### Test Automation

**CI Pipeline**:

```yaml
name: CI Pipeline

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --lib

  integration-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --test integration

  regression-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --test regression

  pty-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --test pty

  performance-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --test performance

  stress-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --test stress
```

## Test Metrics

### Coverage Metrics

| Component | Target Coverage | Current Coverage |
|-----------|-----------------|-------------------|
| Input Channel | 95% | 0% |
| Terminal Input | 90% | 0% |
| History Validation | 95% | 0% |
| Interruption Handling | 95% | 0% |
| Main Loop | 85% | 0% |
| Overall | 90% | 0% |

### Performance Metrics

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Input Latency | < 100ms | Performance tests |
| Interruption Latency | < 500ms | Performance tests |
| Message Throughput | > 100 msg/sec | Stress tests |
| History Validation Time | < 100ms | Performance tests |
| Tool Call Cleanup Time | < 50ms | Performance tests |

### Quality Metrics

| Metric | Target | Current |
|--------|--------|---------|
| Test Success Rate | 99%+ | N/A |
| Flaky Tests | 0 | N/A |
| Test Execution Time | < 30 min | N/A |
| Code Coverage | 90%+ | N/A |

## Test Maintenance Plan

### Test Maintenance Tasks

1. **Weekly**: Review flaky tests
2. **Bi-weekly**: Update test data
3. **Monthly**: Review test coverage
4. **Quarterly**: Add new test scenarios
5. **As Needed**: Fix broken tests

### Test Data Management

- Store test fixtures in `tests/fixtures/`
- Use realistic conversation data
- Rotate test data periodically
- Keep test data size manageable

### Test Documentation

- Maintain `TESTING.md` with test instructions
- Document test scenarios in test files
- Update test documentation with each change
- Include troubleshooting guide

## Risk Mitigation

### Test Risks

| Risk | Mitigation Strategy |
|------|---------------------|
| Flaky PTY tests | Implement retry logic, use timeouts |
| Slow performance tests | Optimize test setup, use smaller datasets |
| Test environment differences | Standardize test environments, use containers |
| Incomplete coverage | Regular coverage reviews, add missing tests |
| Test maintenance burden | Automate test generation where possible |

### Contingency Plans

1. **Failed Test Suite**: Run tests in isolation to identify root cause
2. **Performance Regression**: Rollback changes, investigate bottleneck
3. **Environment Issues**: Rebuild test environment, use alternative
4. **Test Flakes**: Implement retry logic, stabilize tests

## Test Reporting

### Test Report Format

```markdown
# Test Report - {Date}

## Summary
- Total Tests: {number}
- Passed: {number}
- Failed: {number}
- Skipped: {number}
- Success Rate: {percentage}

## Test Categories

### Unit Tests
- Total: {number}
- Passed: {number}
- Failed: {number}
- Coverage: {percentage}

### Integration Tests
- Total: {number}
- Passed: {number}
- Failed: {number}

### Regression Tests
- Total: {number}
- Passed: {number}
- Failed: {number}

### Performance Tests
- Total: {number}
- Passed: {number}
- Failed: {number}
- Metrics: {latency}, {throughput}

### Stress Tests
- Total: {number}
- Passed: {number}
- Failed: {number}

## Failed Tests

1. {test_name}
   - Status: {failed}
   - Error: {error_message}
   - Retry: {yes/no}

## Performance Metrics

| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| Input Latency | < 100ms | {value} | {pass/fail} |
| Interruption Latency | < 500ms | {value} | {pass/fail} |
| Message Throughput | > 100 msg/sec | {value} | {pass/fail} |

## Recommendations

- [ ] Investigate failed tests
- [ ] Optimize slow tests
- [ ] Add missing test coverage
- [ ] Update performance targets

## Sign-off

- Tester: {name}
- Date: {date}
- Approved: {yes/no}
```

## Test Tools

### Required Tools

1. **Rust Testing Framework**: Built-in `cargo test`
2. **PTY Testing**: `nix` crate for terminal control
3. **Performance Testing**: `criterion` crate for benchmarks
4. **Coverage Testing**: `tarpaulin` for coverage reports
5. **Mocking**: `mockall` crate for unit tests

### Tool Configuration

```toml
# Cargo.toml
[dev-dependencies]
tokio = { version = "1.0", features = ["full"] }
nix = "0.26"
criterion = "0.4"
tarpaulin = "0.22"
mockall = "0.11"

[[bench]]
name = "performance_tests"
harness = false
```

## Test Checklist

### Before Implementation

- [ ] Define test scenarios
- [ ] Set up test environment
- [ ] Create test fixtures
- [ ] Write test stubs
- [ ] Configure CI pipeline

### During Implementation

- [ ] Write unit tests
- [ ] Write integration tests
- [ ] Write regression tests
- [ ] Write performance tests
- [ ] Write stress tests
- [ ] Write PTY tests
- [ ] Run tests locally
- [ ] Fix failing tests

### Before Merge

- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] All regression tests pass
- [ ] Performance targets met
- [ ] Coverage goals achieved
- [ ] CI pipeline green
- [ ] Code reviewed

### After Merge

- [ ] Run full test suite
- [ ] Update test documentation
- [ ] Update test metrics
- [ ] Archive test reports
- [ ] Review test coverage

---

**Test Plan Version**: 1.0
**Created**: 2026-01-17
**Last Updated**: 2026-01-17
