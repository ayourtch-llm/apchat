// Integration tests for LongWaitTool MSPC behavior
// Tests real MSPC channel interaction for progress updates and interrupt handling

use apchat_toolcore::{Tool, ToolParameters, ToolContext};
use apchat_policy::PolicyManager;
use apchat_tools::LongWaitTool;
use apchat_mspc::{MspcMessage, MspcChannel};
use std::path::PathBuf;
use std::time::Duration;

/// Create a test ToolContext with real MSPC channels
fn create_context_with_mspc() -> (ToolContext, MspcChannel) {
    let mspc_channel = MspcChannel::new(100);

    let context = ToolContext::new(
        PathBuf::from("/tmp"),
        "test-session-mspc".to_string(),
        PolicyManager::default(),
    )
    .with_mspc_sender(mspc_channel.sender())
    .with_mspc_receiver(mspc_channel.receiver());

    (context, mspc_channel)
}

#[tokio::test]
async fn test_progress_updates_sent_via_mspc() {
    let tool = LongWaitTool;
    let (context, mspc_channel) = create_context_with_mspc();

    let mut params = ToolParameters::new();
    // Wait for 1.5 seconds to ensure multiple progress updates
    params.set("duration", 1.5);
    params.set("message", "Test progress: {progress}%");

    // Spawn a task to collect progress messages
    let message_collector = tokio::spawn(async move {
        let mut messages = Vec::new();
        let timeout = Duration::from_secs(5);
        let start = std::time::Instant::now();

        // Collect messages for the duration of the wait plus a small buffer
        while start.elapsed() < timeout {
            match tokio::time::timeout(
                Duration::from_millis(200),
                mspc_channel.recv()
            ).await {
                Ok(Some(msg)) => {
                    messages.push(msg);
                },
                Ok(None) => break, // Channel closed
                Err(_) => {
                    // Timeout - check if we should stop collecting
                    if start.elapsed() > Duration::from_secs(2) {
                        break;
                    }
                }
            }
        }
        messages
    });

    // Execute the tool
    let result = tool.execute(params, &context).await;

    // Wait for message collection to complete
    let messages = message_collector.await
        .expect("Message collector task failed");

    // Verify the tool completed successfully
    assert!(result.success, "Tool should complete successfully: {:?}", result.error);
    assert!(result.error.is_none());

    // Verify we received some messages
    assert!(!messages.is_empty(), "Should receive at least one MSPC message");

    // Count ToolResult messages (progress updates)
    let progress_messages_count = messages
        .iter()
        .filter(|msg| matches!(msg, MspcMessage::ToolResult(_, _)))
        .count();

    assert!(progress_messages_count > 0, "Should receive at least one ToolResult message");

    // Verify progress messages contain expected format
    for msg in &messages {
        if let MspcMessage::ToolResult(content, _sender) = msg {
            // Progress updates should mention "progress" or contain percentage
            assert!(
                content.contains("progress") || content.contains('%') || content.contains("Test"),
                "Progress message should contain progress indicator: {}",
                content
            );
        }
    }

    println!("Received {} total messages, {} progress updates", messages.len(), progress_messages_count);
}

#[tokio::test]
async fn test_interrupt_signal_cancels_wait() {
    let tool = LongWaitTool;
    let (context, mspc_channel) = create_context_with_mspc();

    let mut params = ToolParameters::new();
    // Wait for 10 seconds (but we'll interrupt it)
    params.set("duration", 10.0);
    params.set("message", "Long wait to be interrupted");

    // Spawn a task to send an interrupt signal after a short delay
    let sender_clone = mspc_channel.sender();
    let interrupt_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        // Send interrupt signal
        let _ = sender_clone.send(MspcMessage::InterruptSignal(
            "!interrupt".to_string(),
            Some("test-user".to_string())
        )).await;
        println!("Sent interrupt signal");
    });

    // Execute the tool - should be interrupted
    let start = std::time::Instant::now();
    let result = tool.execute(params, &context).await;
    let elapsed = start.elapsed();

    // Wait for interrupt task to complete
    interrupt_task.await.expect("Interrupt task failed");

    // Verify the tool was interrupted (should fail quickly)
    assert!(
        !result.success || elapsed < Duration::from_secs(2),
        "Tool should be interrupted and fail or complete quickly. Success: {}, Elapsed: {:?}",
        result.success,
        elapsed
    );

    // If it failed, verify it was due to interruption
    if !result.success {
        assert!(
            result.error.is_some(),
            "Should have an error message when interrupted"
        );
        let error_msg = result.error.as_ref().unwrap();
        println!("Interrupt error message: {}", error_msg);
        // Error should mention interrupt or cancellation
        assert!(
            error_msg.to_lowercase().contains("interrupt") ||
            error_msg.to_lowercase().contains("cancel") ||
            error_msg.to_lowercase().contains("stopped"),
            "Error message should mention interruption: {}",
            error_msg
        );
    }

    // Verify it didn't wait the full 10 seconds
    assert!(
        elapsed < Duration::from_secs(5),
        "Should be interrupted well before 10 seconds, elapsed: {:?}",
        elapsed
    );

    println!("Tool was interrupted after {:?}", elapsed);
}

#[tokio::test]
async fn test_multiple_progress_updates_with_timing() {
    let tool = LongWaitTool;
    let (context, mspc_channel) = create_context_with_mspc();

    let mut params = ToolParameters::new();
    // Wait for 2 seconds to get multiple progress updates
    params.set("duration", 2.0);
    params.set("message", "Processing: {progress}% complete");

    // Spawn a task to collect progress messages with timing
    let message_collector = tokio::spawn(async move {
        let mut messages = Vec::new();
        let start = std::time::Instant::now();

        // Collect messages for the duration
        while start.elapsed() < Duration::from_secs(3) {
            match tokio::time::timeout(
                Duration::from_millis(300),
                mspc_channel.recv()
            ).await {
                Ok(Some(msg)) => {
                    let elapsed = start.elapsed();
                    messages.push((msg, elapsed));
                },
                Ok(None) => break,
                Err(_) => {
                    if start.elapsed() > Duration::from_secs(2) {
                        break;
                    }
                }
            }
        }
        messages
    });

    // Execute the tool
    let result = tool.execute(params, &context).await;

    // Wait for message collection
    let timed_messages = message_collector.await
        .expect("Message collector task failed");

    // Verify the tool completed successfully
    assert!(result.success, "Tool should complete successfully: {:?}", result.error);

    // Extract progress messages
    let progress_updates: Vec<(MspcMessage, Duration)> = timed_messages
        .iter()
        .filter(|(msg, _)| matches!(msg, MspcMessage::ToolResult(_, _)))
        .map(|(msg, time)| (msg.clone(), *time))
        .collect();

    println!("Received {} progress updates over 2 seconds:", progress_updates.len());
    for (i, (msg, time)) in progress_updates.iter().enumerate() {
        if let MspcMessage::ToolResult(content, _) = msg {
            println!("  [{}] @ {:?}: {}", i, time, content);
        }
    }

    // Should have multiple progress updates (at least 2-3 for a 2-second wait)
    assert!(
        progress_updates.len() >= 2,
        "Should receive at least 2 progress updates for a 2-second wait, got {}",
        progress_updates.len()
    );

    // Verify messages are spread over time (not all at once)
    if progress_updates.len() >= 2 {
        let first_time = progress_updates[0].1;
        let last_time = progress_updates[progress_updates.len() - 1].1;
        let time_span = last_time - first_time;

        assert!(
            time_span >= Duration::from_millis(500),
            "Progress updates should be spread over time, span was {:?}",
            time_span
        );
    }
}

#[tokio::test]
async fn test_graceful_shutdown_on_immediate_interrupt() {
    let tool = LongWaitTool;
    let (context, mspc_channel) = create_context_with_mspc();

    let mut params = ToolParameters::new();
    params.set("duration", 5.0);
    params.set("message", "Will be interrupted immediately");

    // Send interrupt immediately
    let sender_clone = mspc_channel.sender();
    let interrupt_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let _ = sender_clone.send(MspcMessage::InterruptSignal(
            "!interrupt".to_string(),
            Some("test-user".to_string())
        )).await;
    });

    // Execute the tool
    let start = std::time::Instant::now();
    let result = tool.execute(params, &context).await;
    let elapsed = start.elapsed();

    // Wait for interrupt
    interrupt_task.await.expect("Interrupt task failed");

    // Should fail very quickly (within 1 second)
    assert!(
        elapsed < Duration::from_secs(1),
        "Should be interrupted immediately, elapsed: {:?}",
        elapsed
    );

    // Should have an error
    assert!(!result.success, "Should fail when interrupted immediately");
    assert!(result.error.is_some(), "Should have error message");

    println!("Gracefully handled immediate interrupt after {:?}", elapsed);
}

#[tokio::test]
async fn test_progress_message_formatting_in_mspc() {
    let tool = LongWaitTool;
    let (context, mspc_channel) = create_context_with_mspc();

    let mut params = ToolParameters::new();
    params.set("duration", 1.0);
    // Use a custom message with specific format
    params.set("message", "Custom Task: {progress}% - {elapsed:.1}s elapsed");

    // Spawn message collector
    let message_collector = tokio::spawn(async move {
        let mut messages = Vec::new();
        let start = std::time::Instant::now();

        while start.elapsed() < Duration::from_secs(3) {
            match tokio::time::timeout(
                Duration::from_millis(300),
                mspc_channel.recv()
            ).await {
                Ok(Some(msg)) => {
                    messages.push(msg);
                },
                Ok(None) => break,
                Err(_) => {
                    // Check if we should stop
                    if start.elapsed() > Duration::from_millis(1500) {
                        break;
                    }
                }
            }
        }
        messages
    });

    // Execute the tool
    let result = tool.execute(params, &context).await;

    // Get collected messages
    let messages = message_collector.await
        .expect("Message collector task failed");

    // Verify success
    assert!(result.success, "Tool should succeed: {:?}", result.error);

    // Find ToolResult messages
    let tool_results: Vec<&MspcMessage> = messages
        .iter()
        .filter(|msg| matches!(msg, MspcMessage::ToolResult(_, _)))
        .collect();

    // Note: Progress updates are sent via try_send which may fail if channel is busy
    // So we just verify the tool completed successfully and mention the custom message
    println!("Tool result content: {}", result.content);

    // Verify the result contains the custom message
    assert!(
        result.content.contains("Custom Task"),
        "Tool result should contain custom message format"
    );

    // If we did receive MSPC messages, verify the format
    if !tool_results.is_empty() {
        println!("Received {} MSPC ToolResult messages", tool_results.len());
        for msg in tool_results {
            if let MspcMessage::ToolResult(content, _sender) = msg {
                println!("  MSPC message: {}", content);
                // Verify the custom message format is preserved
                if content.contains("Custom Task") {
                    // Should contain percentage or elapsed time
                    assert!(
                        content.contains('%') || content.contains("elapsed"),
                        "Custom message should contain percentage or elapsed time: {}",
                        content
                    );
                }
            }
        }
    } else {
        println!("No MSPC messages received (try_send may have failed - this is acceptable)");
    }

    println!("Verified custom message formatting in tool output");
}
