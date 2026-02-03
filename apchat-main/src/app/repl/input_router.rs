use std::sync::Arc;
use colored::Colorize;
use tokio::task::JoinHandle;

use apchat_vty::{print_heart_yellow, request_counter};
use apchat_models::ModelColor;
use apchat_toolcore::confirmation::ConfirmationRegistry;

use crate::mspc::{MspcChannel, MspcMessage, get_readline_receiver};
use crate::input_router::TerminalInputRouter;
use crate::config::ClientConfig;


/// Configuration for the background input router task.
pub struct RouterConfig {
    pub mspc_channel: Arc<MspcChannel>,
    pub client_config: ClientConfig,
    pub current_model_shared: Arc<std::sync::RwLock<ModelColor>>,
    pub confirmation_registry: Arc<ConfirmationRegistry>,
    pub signal_receiver: tokio::sync::mpsc::Receiver<MspcMessage>,
    pub interrupt_sender: tokio::sync::mpsc::Sender<MspcMessage>,
}

/// Spawn the background terminal input router task.
///
/// The task loops: builds a colored prompt from the current model, calls
/// readline (via spawn_blocking), and routes the result onto the MSPC channel.
/// Confirmation responses and EOF/interrupt signals are decoded from readline
/// error strings and dispatched appropriately.
///
/// Returns the JoinHandle so the caller can abort it on exit.
pub fn spawn_input_router(config: RouterConfig) -> JoinHandle<()> {
    let RouterConfig {
        mspc_channel,
        client_config,
        current_model_shared,
        confirmation_registry,
        signal_receiver,
        interrupt_sender: _interrupt_sender,
    } = config;

    let mut terminal_router = TerminalInputRouter::new(mspc_channel);
    terminal_router = terminal_router.with_signal_receiver(signal_receiver);

    tokio::spawn(async move {
        // Wrap signal receiver in a Tokio Mutex so it can be shared across spawn_blocking calls
        let signal_receiver_mutex = Arc::new(tokio::sync::Mutex::new(
            terminal_router.take_signal_receiver().expect("Signal receiver should be set")
        ));
        println!("");

        loop {
            // Get current model state for prompt
            let current_model = {
                current_model_shared.read().unwrap().clone()
            };

            let model_name = super::get_model_name_for_prompt(&current_model, &client_config);
            let model_indicator = format!("[{} ({})]", current_model.display_name(), model_name).bright_magenta();
            let request_count = request_counter::get_count();
            let prompt_string = format!("{}[{}] {}", model_indicator, request_count, "You:".bright_green().bold());


            // Clone the Arc for use in spawn_blocking
            let receiver_mutex_clone = signal_receiver_mutex.clone();
            let mut readline_receiver = get_readline_receiver();

            // Use spawn_blocking for readline (it's a blocking operation)
            let line_result = tokio::task::spawn_blocking(move || {
                let mut receiver_guard = receiver_mutex_clone.blocking_lock();
                let receiver_ref = &mut *receiver_guard;

                apchat_vty::ReadlineInstance::readline_with_mspc(&prompt_string, Some(receiver_ref), Some(&mut readline_receiver))
            }).await;

            match line_result {
                Ok(Ok(Some(line))) => {
                    // Add to readline history immediately so up-arrow works in next readline() call
                    if let Err(e) = apchat_vty::ReadlineInstance::add_history(&line) {
                        print_heart_yellow(&format!("{} Failed to add to history: {}", "⚠️".bright_yellow(), e), true);
                    }

                    let message = terminal_router.parse_input(&line);
                    if terminal_router.send_to_channel(message).await.is_err() {
                        break; // Channel closed, exit
                    }
                }
                Ok(Ok(None)) => {
                    // Empty line or confirmation request handled - continue
                    continue;
                }
                Ok(Err(e)) => {
                    let err_str = e.to_string();

                    if let Some((approved, confirmation_id, reason)) = parse_tool_confirmation_response(&err_str) {
                        // Forward to confirmation registry
                        if let Err(e) = confirmation_registry.complete(&confirmation_id, (approved, reason)).await {
                            print_heart_yellow(&format!("{} Failed to complete confirmation: {}", "⚠️".yellow(), e), true);
                        }
                    } else if let Some((approved, reason)) = parse_confirmation_response(&err_str) {
                        // Forward confirmation response to main channel
                        let _ = terminal_router.send_to_channel(
                            MspcMessage::ConfirmationResponse(approved, reason)
                        ).await;
                    } else if err_str.contains("EOF") {
                        // Ctrl-D pressed - send exit command
                        let _ = terminal_router.send_to_channel(
                            MspcMessage::Command("exit".to_string(), Some("terminal".to_string()))
                        ).await;
                        break;
                    } else if err_str.contains("Interrupted") {
                        // Ctrl-C pressed - send interrupt signal
                        let _ = terminal_router.send_to_channel(
                            MspcMessage::InterruptSignal("interrupt".to_string(), Some("terminal".to_string()))
                        ).await;
                    } else {
                        // Other errors - exit
                        print_heart_yellow(&format!("{} {}", "Error reading input:".bright_red().bold(), e), true);
                        break;
                    }
                }
                Err(_) => break, // Task panic
            }
        }
    })
}

/// Parse a `__TOOL_CONFIRMATION_RESPONSE__:` error string.
/// Format: `__TOOL_CONFIRMATION_RESPONSE__:<approved>|<id>|<reason>`
fn parse_tool_confirmation_response(err_str: &str) -> Option<(bool, String, Option<String>)> {
    let response_str = err_str.strip_prefix("__TOOL_CONFIRMATION_RESPONSE__:")?;
    let parts: Vec<&str> = response_str.splitn(3, '|').collect();
    let approved = parts.get(0).map(|s| *s == "true").unwrap_or(false);
    let confirmation_id = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
    let reason = parts.get(2).map(|s| s.to_string());
    Some((approved, confirmation_id, reason))
}

/// Parse a `__CONFIRMATION_RESPONSE__:` error string.
/// Format: `__CONFIRMATION_RESPONSE__:<approved>|<reason>`
fn parse_confirmation_response(err_str: &str) -> Option<(bool, Option<String>)> {
    let response_str = err_str.strip_prefix("__CONFIRMATION_RESPONSE__:")?;
    let parts: Vec<&str> = response_str.splitn(2, '|').collect();
    let approved = parts.get(0).map(|s| *s == "true").unwrap_or(false);
    let reason = parts.get(1).map(|s| s.to_string());
    Some((approved, reason))
}
