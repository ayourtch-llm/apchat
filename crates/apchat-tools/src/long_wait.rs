use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use colored::Colorize;
use std::time::Duration;

/// Default wait duration in seconds (30 seconds)
const DEFAULT_DURATION: f64 = 30.0;

/// Maximum wait duration in seconds (10 minutes)
const MAX_DURATION: f64 = 600.0;

/// Tool for pausing execution with progress updates
pub struct LongWaitTool;

#[async_trait]
impl Tool for LongWaitTool {
    fn name(&self) -> &str {
        "long_wait"
    }

    fn description(&self) -> &str {
        "Pause execution for a specified duration with progress updates. Useful for long-running operations where you want to show progress to the user. The tool supports cancellation and provides periodic status updates."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("duration", "number", "Duration to wait in seconds (must be positive, max 600)", required),
            param!("message", "string", "Optional message to display during wait. Use {progress} placeholder for percentage.", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let duration = match params.get_required::<f64>("duration") {
            Ok(duration) => duration,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let message = params.get_optional::<String>("message")
            .ok()
            .flatten()
            .unwrap_or_else(|| "Waiting".to_string());

        // Validate duration
        if duration <= 0.0 {
            return ToolResult::error("Duration must be positive".to_string());
        }

        if duration > MAX_DURATION {
            return ToolResult::error(format!(
                "Duration cannot exceed {} seconds (10 minutes)",
                MAX_DURATION
            ));
        }

        println!("{} {} for {:.1} seconds", "LongWait:".yellow(), message.cyan(), duration);

        // Implement wait loop with progress updates
        match wait_with_progress(duration, &message, context).await {
            Ok(msg) => ToolResult::success(msg),
            Err(e) => ToolResult::error(e.to_string()),
        }
    }
}

/// Wait for the specified duration with exponential backoff progress updates
async fn wait_with_progress(duration: f64, message: &str, context: &ToolContext) -> Result<String, Box<dyn std::error::Error>> {
    use tokio::time::{sleep, Instant};
    
    let start_time = Instant::now();
    let total_duration = Duration::from_secs_f64(duration);
    let mut elapsed = Duration::ZERO;
    let mut next_update_interval = Duration::from_secs(1);
    let mut last_update_time = Instant::now();
    
    while elapsed < total_duration {
        // Check for interrupts
        if check_for_interrupts(context).await? {
            let progress_pct = (elapsed.as_secs_f64() / duration) * 100.0;
            return Err(format!("Wait interrupted after {:.1} seconds ({:.1}% complete)", 
                elapsed.as_secs_f64(), progress_pct).into());
        }
        
        let now = Instant::now();
        
        // Send progress update if enough time has passed
        if now.duration_since(last_update_time) >= next_update_interval {
            let progress_pct = (elapsed.as_secs_f64() / duration) * 100.0;
            let formatted_msg = message.replace("{progress}", &format!("{:.1}", progress_pct));
            
            send_progress(
                context,
                &formatted_msg,
                progress_pct,
                elapsed.as_secs_f64(),
                duration
            ).await;
            
            // Exponential backoff: double the interval up to 32 seconds
            next_update_interval = std::cmp::min(
                next_update_interval * 2,
                Duration::from_secs(32)
            );
            
            last_update_time = now;
        }
        
        // Sleep for a small interval to avoid busy-waiting
        sleep(Duration::from_millis(100)).await;
        elapsed = start_time.elapsed();
    }
    
    // Final progress update at 100%
    let formatted_msg = message.replace("{progress}", "100.0");
    send_progress(context, &formatted_msg, 100.0, duration, duration).await;
    
    Ok(format!("Waited for {:.1} seconds: {}", duration, message))
}

/// Check for interrupt signals via ToolContext.mspc_receiver
async fn check_for_interrupts(context: &ToolContext) -> Result<bool, Box<dyn std::error::Error>> {
    // Only check if receiver is available
    let receiver_ref = match &context.mspc_receiver {
        Some(rx) => rx,
        None => return Ok(false), // No receiver configured, no interrupts possible
    };
    
    // Try to lock the receiver
    let mut receiver = match receiver_ref.try_lock() {
        Ok(guard) => guard,
        Err(_) => return Ok(false), // Lock failed, continue without checking
    };
    
    // Try to receive a message without blocking
    match receiver.try_recv() {
        Ok(msg) => {
            // Check if this is an interrupt signal
            match msg {
                apchat_mspc::MspcMessage::InterruptSignal(_, _) => Ok(true),
                _ => Ok(false),
            }
        }
        Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
            // No interrupt, continue
            Ok(false)
        }
        Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
            // Channel closed, treat as interrupt
            Ok(true)
        }
    }
}

/// Send progress updates via ToolContext.mspc_sender
async fn send_progress(context: &ToolContext, message: &str, progress_pct: f64, elapsed_secs: f64, total_secs: f64) {
    let progress_msg = format!(
        "{}: {:.1}% complete ({:.1}s / {:.1}s elapsed)",
        message,
        progress_pct,
        elapsed_secs,
        total_secs
    );
    
    println!("{}", progress_msg.cyan());
    
    // Only send if sender is available
    if let Some(sender) = &context.mspc_sender {
        // Use ToolResult message type for progress updates
        let msg = apchat_mspc::MspcMessage::ToolResult(progress_msg.clone(), None);
        
        // Try to send progress update via MSPC
        // Note: We don't want to fail if sending fails, just log and continue
        match sender.try_send(msg) {
            Ok(_) => {},
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                // Channel full, skip this update
            }
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                eprintln!("Warning: MSPC channel closed, progress updates will not be broadcast");
            }
        }
    }
}
