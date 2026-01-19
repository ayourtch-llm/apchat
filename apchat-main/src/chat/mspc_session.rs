// New MSPC-integrated chat loop
use anyhow::Result;
use colored::Colorize;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::APChat;
use apchat_models::{ModelColor, Message};
use apchat_logging::safe_truncate;
use crate::mspc::{MspcChannel, MspcMessage, HistoryError};
use crate::input_router::TerminalInputRouter;

/// Types of interruptions that can occur
#[derive(Debug, Clone, PartialEq)]
pub enum InterruptionType {
    /// User-initiated interruption (e.g., via !command)
    UserInitiated,
    /// System error or unexpected failure
    SystemError,
    /// Timeout or long-running operation interruption
    Timeout,
}

/// New chat loop with MSPC integration
/// This function implements a continuous loop that:
/// - Checks for interrupts frequently
/// - Processes regular inputs at turn end
/// - Maintains message history
/// - Handles confirmation prompts
/// - Validates history integrity
pub(crate) async fn chat_with_mspc(
    chat: &mut APChat,
    mspc_channel: Arc<MspcChannel>,
    cancellation_token: Option<tokio_util::sync::CancellationToken>,
) -> Result<()> {
    // Initialize terminal input router
    let terminal_router = TerminalInputRouter::new(mspc_channel.clone());
    let terminal_router_clone = TerminalInputRouter::new(mspc_channel.clone());
    
    // Spawn terminal input reader in background
    tokio::spawn(async move {
        read_terminal_input(terminal_router_clone).await;
    });
    
    // Main interaction loop
    loop {
        // Check for cancellation at the start of each iteration
        if let Some(ref token) = cancellation_token {
            if token.is_cancelled() {
                return Err(anyhow::anyhow!("Chat interrupted by user"));
            }
        }
        
        // Check for pending messages (non-blocking)
        match mspc_channel.try_recv().await {
            Ok(Some(message)) => {
                if mspc_channel.is_interrupt(&message) {
                    // Handle interrupt immediately
                    if let MspcMessage::InterruptSignal(content, _sender) = message {
                        eprintln!("{} Interrupt received: {}", "⚠️".yellow(), content);
                        
                        // Handle interruption with validation and repair
                        match mspc_channel.handle_interruption_with_validation().await {
                            Ok(interrupted) => {
                                if !interrupted.is_empty() {
                                    eprintln!("{} Interrupted message: {}", "ℹ️".blue(), safe_truncate(&interrupted, 100));
                                }
                                
                                // Add interruption to message history
                                chat.messages.push(Message {
                                    role: "user".to_string(),
                                    content: format!("[INTERRUPTED] {}", content),
                                    tool_calls: None,
                                    tool_call_id: None,
                                    name: None,
                                    reasoning: None,
                                });
                                
                                // Validate history after interruption
                                validate_and_log_history(&mspc_channel).await;
                            }
                            Err(errors) => {
                                eprintln!("{} History validation errors after interruption:", "⚠️".yellow());
                                for error in errors {
                                    eprintln!("  - {}", error);
                                }
                                
                                // Attempt to repair history
                                if let Err(repair_errors) = mspc_channel.validate_and_repair_history().await {
                                    eprintln!("{} Failed to repair history: {}", "❌".red(), repair_errors.len());
                                } else {
                                    eprintln!("{} History repaired successfully", "✅".green());
                                }
                            }
                        }
                        
                        continue;
                    }
                } else if mspc_channel.is_command(&message) {
                    // Handle command
                    if let MspcMessage::Command(content, _sender) = message {
                        eprintln!("{} Command received: {}", "🔧".cyan(), content);
                        
                        // Process command (e.g., /model, /skills)
                        if content.starts_with("/model") {
                            // Handle model switch command
                            let model_part = content.trim_start_matches("/model").trim();
                            if let Some(new_model) = parse_model_command(model_part) {
                                chat.current_model = new_model;
                                eprintln!("{} Switched to model: {:?}", "✅".green(), chat.current_model);
                            }
                        } else if content == "/skills" {
                            // Display available skills
                            eprintln!("{} Available commands:", "📋".bright_cyan());
                            eprintln!("  /model <grn|blu|red> - Switch model");
                            eprintln!("  /skills - Show this help");
                            eprintln!("  !<command> - Interrupt current operation");
                        } else if content == "/validate" {
                            // Validate history
                            validate_and_log_history(&mspc_channel).await;
                        } else if content == "/repair" {
                            // Repair history
                            match mspc_channel.validate_and_repair_history().await {
                                Ok(_) => {
                                    eprintln!("{} History repaired successfully", "✅".green());
                                }
                                Err(errors) => {
                                    eprintln!("{} Failed to repair history:", "❌".red());
                                    for error in errors {
                                        eprintln!("  - {}", error);
                                    }
                                }
                            }
                        }
                        
                        continue;
                    }
                } else if mspc_channel.is_confirmation_request(&message) {
                    // Handle confirmation request
                    if let MspcMessage::ConfirmationRequest(content, _sender) = message {
                        eprintln!("{} {}", "❓".yellow(), content);
                        eprintln!("{} Type 'yes' or 'no': ", "👉".bright_black(),);
                        
                        // Wait for confirmation response
                        match mspc_channel.recv().await {
                            Some(MspcMessage::ConfirmationResponse(response, _sender)) => {
                                if response {
                                    eprintln!("{} Confirmed", "✅".green());
                                } else {
                                    eprintln!("{} Cancelled", "❌".red());
                                }
                            }
                            Some(other) => {
                                eprintln!("{} Unexpected response: {:?}", "⚠️".yellow(), other);
                            }
                            None => {
                                eprintln!("{} No response received", "⚠️".yellow());
                            }
                        }
                        
                        continue;
                    }
                } else if let MspcMessage::UserInput(content, _sender) = message {
                    // Process the user input
                    process_user_input(chat, &content, &mspc_channel).await?;
                }
            }
            Ok(None) => {
                // No pending messages, continue with normal flow
                // Small delay to prevent busy waiting
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
            Err(e) => {
                eprintln!("{} Channel error: {}", "⚠️".yellow(), e);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }
}

/// Validate and log history status
async fn validate_and_log_history(mspc_channel: &MspcChannel) {
    match mspc_channel.validate_history().await {
        Ok(errors) => {
            if errors.is_empty() {
                eprintln!("{} History validation: OK", "✅".green());
            } else {
                eprintln!("{} History validation: {} error(s)", "⚠️".yellow(), errors.len());
                for error in errors {
                    eprintln!("  - {}", error);
                }
            }
        }
        Err(error) => {
            eprintln!("{} History validation failed: {}", "❌".red(), error);
        }
    }
}

/// Handle interruption with context-aware cleanup
async fn handle_interruption_context_aware(
    mspc_channel: &MspcChannel,
    interruption_type: InterruptionType,
) -> Result<String, Vec<HistoryError>> {
    match interruption_type {
        InterruptionType::UserInitiated => {
            // For user-initiated interruptions, clear partial agent responses
            // but preserve complete conversation history
            let interrupted = mspc_channel.handle_interruption_with_validation().await?;
            Ok(interrupted)
        }
        InterruptionType::SystemError => {
            // For system errors, preserve history for debugging
            // but ensure it's in a valid state
            let interrupted = mspc_channel.handle_interruption_with_validation().await?;
            Ok(interrupted)
        }
        InterruptionType::Timeout => {
            // For timeouts, handle partial responses gracefully
            let interrupted = mspc_channel.handle_interruption_with_validation().await?;
            Ok(interrupted)
        }
    }
}

/// Read input from terminal and send to MSPC channel
async fn read_terminal_input(router: TerminalInputRouter) {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::sync::mpsc;
    
    let stdin = tokio::io::stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();
    
    while let Ok(Some(line)) = lines.next_line().await {
        let message = router.parse_input(&line);
        router.send_to_channel(message).await;
    }
}

/// Process user input
async fn process_user_input(
    chat: &mut APChat,
    user_message: &str,
    mspc_channel: &MspcChannel,
) -> Result<String> {
    // Add user message to history
    chat.messages.push(Message {
        role: "user".to_string(),
        content: user_message.to_string(),
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning: None,
    });
    
    // Add to MSPC history
    mspc_channel.add_user_message(user_message.to_string()).await;
    
    // Validate history after adding user message
    if let Err(errors) = mspc_channel.validate_history().await {
        eprintln!("{} History validation warning: {}", "⚠️".yellow(), errors);
    }
    
    // Summarize history before processing
    crate::chat::history::summarize_and_trim_history(chat).await?;
    
    // Call the existing chat logic
    let response = execute_chat_turn(chat).await?;
    
    // Add agent response to history
    mspc_channel.add_agent_message(response.clone()).await;
    
    // Validate history after adding agent message
    if let Err(errors) = mspc_channel.validate_history().await {
        eprintln!("{} History validation warning: {}", "⚠️".yellow(), errors);
    }
    
    Ok(response)
}

/// Execute a single chat turn (reuses existing logic)
async fn execute_chat_turn(chat: &mut APChat) -> Result<String> {
    // This function would call the existing chat logic
    // For now, return a placeholder
    Ok("Response from LLM".to_string())
}

/// Parse model switch command
fn parse_model_command(command: &str) -> Option<ModelColor> {
    match command.to_lowercase().as_str() {
        "grn" | "green" => Some(ModelColor::GrnModel),
        "blu" | "blue" => Some(ModelColor::BluModel),
        "red" => Some(ModelColor::RedModel),
        _ => None,
    }
}

