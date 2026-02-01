mod init;
mod commands;
mod input_router;
mod inference;

use anyhow::Result;
use colored::Colorize;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;

use apchat_vty::{print_heart_yellow, print_heart_red};
use apchat_models::ModelColor;
use apchat_policy::PolicyManager;

use crate::APChat;
use crate::cli::Cli;
use crate::config::ClientConfig;
use crate::mspc::{MspcChannel, MspcMessage};

use commands::CommandResult;
use inference::InferenceOutcome;

/// Get the display name for a model color from client config.
///
/// Checks for a user-specified model override first, then falls back to the
/// provider's default model name.  Used by both the prompt builder in the
/// input router task and the response display in the main loop.
pub(crate) fn get_model_name_for_prompt(color: &ModelColor, client_config: &ClientConfig) -> String {
    if let Some(override_model) = client_config.get_model_override(*color) {
        override_model.to_string()
    } else {
        let provider = client_config.get_provider(*color);
        provider.model_name.clone()
    }
}

/// Run interactive REPL mode.
///
/// Orchestrates the full lifecycle: initialization, Ctrl-C handler, MSPC
/// channel wiring, background input router, the main message loop, and
/// cleanup on exit.
pub async fn run_repl_mode(
    cli: &Cli,
    client_config: ClientConfig,
    work_dir: PathBuf,
    policy_manager: PolicyManager,
    webex_sink: Option<std::sync::Arc<apchat_webex::WebexOutputSink>>,
    mspc_channel_opt: Option<Arc<MspcChannel>>,
) -> Result<()> {
    // ── Initialization ─────────────────────────────────────────────────────
    let (mut chat, _idle_config) =
        init::initialize_repl(cli, client_config, work_dir, policy_manager).await?;

    // ── Persistent Ctrl-C handler ──────────────────────────────────────────
    // Holds the current operation's cancellation token so Ctrl-C can find it.
    let current_token: Arc<std::sync::Mutex<Option<tokio_util::sync::CancellationToken>>> =
        Arc::new(std::sync::Mutex::new(None));
    let current_token_for_handler = current_token.clone();

    tokio::spawn(async move {
        loop {
            if tokio::signal::ctrl_c().await.is_ok() {
                if let Ok(guard) = current_token_for_handler.lock() {
                    if let Some(ref token) = *guard {
                        print_heart_red(&format!("\n{}", "^C - Interrupting...".bright_yellow()), true);
                        token.cancel();
                    }
                }
            }
        }
    });

    // ── MSPC channel & confirmation plumbing ───────────────────────────────
    let mspc_channel = mspc_channel_opt.unwrap_or_else(|| Arc::new(MspcChannel::new(100)));
    chat.mspc_channel = Some(mspc_channel.clone());

    use apchat_toolcore::confirmation::ConfirmationRegistry;
    let confirmation_registry = Arc::new(ConfirmationRegistry::new());
    chat.confirmation_registry = Some(confirmation_registry.clone());

    let (signal_sender, signal_receiver) = tokio::sync::mpsc::channel::<MspcMessage>(10);
    let (interrupt_sender, interrupt_receiver) = tokio::sync::mpsc::channel::<MspcMessage>(10);
    let interrupt_receiver_mutex = Arc::new(tokio::sync::Mutex::new(interrupt_receiver));

    chat.signal_sender = Some(signal_sender);
    chat.signal_receiver = Some(interrupt_receiver_mutex);

    // ── Background input router ────────────────────────────────────────────
    let current_model_shared = Arc::new(std::sync::RwLock::new(chat.current_model));
    let current_model_for_main = current_model_shared.clone();

    let router_handle = input_router::spawn_input_router(input_router::RouterConfig {
        mspc_channel:         mspc_channel.clone(),
        client_config:        chat.client_config.clone(),
        current_model_shared,
        confirmation_registry: confirmation_registry.clone(),
        signal_receiver,
        interrupt_sender:     interrupt_sender.clone(),
    });

    let interrupt_sender_for_main = interrupt_sender;

    // ── Main REPL loop ─────────────────────────────────────────────────────
    'outer: loop {
        // Wait for next message from the input router
        let message = match mspc_channel.recv().await {
            Some(msg) => msg,
            None => {
                // Channel closed — normal exit path
                print_heart_red(&format!("\n{}", "Goodbye!".bright_cyan()), true);
                if let Err(save_err) = apchat_vty::ReadlineInstance::save_history() {
                    if chat.debug_level > 0 {
                        print_heart_yellow(&format!("{} Failed to save readline history: {}", "⚠️".yellow(), save_err), true);
                    }
                }
                break;
            }
        };

        // Route special message types before extracting the text payload
        let line = match message {
            MspcMessage::UserInput(content, _sender) => content,
            MspcMessage::Command(content, _sender)   => content,

            MspcMessage::InterruptSignal(content, sender) => {
                // Forward to tools, then inform user there's nothing to cancel
                let _ = interrupt_sender_for_main
                    .send(MspcMessage::InterruptSignal(content, sender)).await;
                print_heart_red(&format!("\n{}", "No operation in progress to interrupt".bright_yellow()), true);
                continue;
            }

            MspcMessage::ConfirmationRequest(content, _sender) => {
                handle_confirmation_request(&content, &mspc_channel).await;
                continue;
            }

            _ => continue,
        };

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Exit commands
        if line == "exit" || line == "quit" {
            print_heart_red(&format!("{}", "Goodbye!".bright_cyan()), true);
            if let Err(e) = apchat_vty::ReadlineInstance::save_history() {
                if chat.debug_level > 0 {
                    print_heart_yellow(&format!("{} Failed to save readline history: {}", "⚠️".yellow(), e), true);
                }
            }
            break;
        }

        // Slash-command dispatch
        match commands::dispatch_command(&mut chat, line, &current_model_for_main).await {
            CommandResult::Handled    => continue,
            CommandResult::NotACommand => {} // fall through to inference
        }

        // Persist input, log it, and auto-save chat state
        save_input_and_log(&mut chat, line).await;

        // ── Inference with interrupt support ───────────────────────────────
        let cancel_token = tokio_util::sync::CancellationToken::new();
        {
            let mut guard = current_token.lock().unwrap();
            *guard = Some(cancel_token.clone());
        }

        let outcome = inference::run_inference(
            &mut chat,
            line,
            &cancel_token,
            &mspc_channel,
        ).await;

        // Always clear the cancellation token, regardless of outcome
        {
            let mut guard = current_token.lock().unwrap();
            *guard = None;
        }

        match outcome {
            InferenceOutcome::Response(response) => {
                // Log assistant response
                if let Some(logger) = &mut chat.logger {
                    logger.log("assistant", &response, None, false).await;
                }

                // Broadcast to Webex if enabled
                if let Some(ref webex) = webex_sink {
                    if let Err(e) = webex.send_response(&response).await {
                        print_heart_yellow(&format!("{} Failed to send to Webex: {}", "⚠️".yellow(), e), true);
                    }
                }

                // Display response (streaming already printed it inline)
                if !chat.stream_responses {
                    let model_name = get_model_name_for_prompt(&chat.current_model, &chat.client_config);
                    let model_label     = format!("[{} ({})]", chat.current_model.display_name(), model_name).bright_magenta();
                    let assistant_label = "Assistant:".bright_blue().bold();
                    print_heart_red(&format!("\n{} {} {}\n", model_label, assistant_label, response), true);
                } else {
                    // Separator after streaming output
                    print_heart_red(&format!(""), true);
                }
            }

            InferenceOutcome::Interrupted | InferenceOutcome::Error => {
                // Error/interrupted messages were already pushed by run_inference
                continue 'outer;
            }
        }
    }

    // ── Cleanup ────────────────────────────────────────────────────────────
    router_handle.abort();

    if let Some(logger) = &mut chat.logger {
        logger.shutdown().await;
    }

    if let Err(e) = apchat_vty::ReadlineInstance::cleanup() {
        if chat.debug_level > 0 {
            print_heart_yellow(&format!("{} Failed to cleanup readline instance: {}", "⚠️".yellow(), e), true);
        }
    }

    Ok(())
}

/// Handle an inline confirmation request from a tool.
///
/// Displays the prompt, reads the user's y/n response (and an optional
/// rejection reason), then sends a `ConfirmationResponse` back on the channel.
async fn handle_confirmation_request(content: &str, mspc_channel: &Arc<MspcChannel>) {
    print_heart_red(&format!("\n{}", content.bright_green().bold()), true);
    print_heart_red(&format!("{} ", ">>>".bright_cyan()), false);
    std::io::stdout().flush().ok();

    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut response = String::new();

    if let Err(e) = handle.read_line(&mut response) {
        print_heart_yellow(&format!("{} Failed to read response: {}", "❌".bright_red(), e), true);
        let _ = mspc_channel.send(
            MspcMessage::ConfirmationResponse(false, Some("Failed to read response".to_string()))
        ).await;
        return;
    }

    let response = response.trim();
    let response_lower = response.to_lowercase();
    let approved = response_lower.is_empty() || response_lower == "y" || response_lower == "yes";

    let rejection_reason = if !approved {
        print_heart_red(&format!("{} ", "Why not? (optional - helps the AI understand):".bright_yellow()), false);
        std::io::stdout().flush().ok();

        let mut reason = String::new();
        if handle.read_line(&mut reason).is_ok() {
            let reason = reason.trim();
            if reason.is_empty() { None } else { Some(reason.to_string()) }
        } else {
            None
        }
    } else {
        None
    };

    let _ = mspc_channel.send(
        MspcMessage::ConfirmationResponse(approved, rejection_reason)
    ).await;
}

/// Save user input to persistent readline history, log it to the session
/// logger, and trigger an auto-save of the chat state.
async fn save_input_and_log(chat: &mut APChat, line: &str) {
    // Persist to readline history file
    match apchat_vty::history::save_to_file(
        &apchat_vty::history::ReadlineEntry::with_session(
            line,
            format!("session_{}", chat.process_id)
        )
    ) {
        Ok(_) => {
            if chat.debug_level > 2 {
                print_heart_red(&format!("{} Saved to readline history", "✏️".bright_blue()), true);
            }
        }
        Err(e) => {
            if chat.debug_level > 0 {
                print_heart_yellow(&format!("{} Failed to save readline history: {}", "⚠️".yellow(), e), true);
            }
        }
    }

    // Log the user message
    if let Some(logger) = &mut chat.logger {
        logger.log("user", line, None, false).await;
    }

    println!("");

    // Auto-save chat history
    match chat.auto_save_history() {
        Ok(_) => {
            if chat.debug_level > 1 {
                print_heart_red(&format!("{} Auto-saved history to history-{}.json",
                         "💾".bright_blue(), chat.process_id), true);
            }
        }
        Err(e) => {
            if chat.debug_level > 0 {
                print_heart_yellow(&format!("{} Auto-save failed: {}", "⚠️".yellow(), e), true);
            }
        }
    }
}

#[cfg(test)]
mod repl_compact_tests {
    use crate::APChat;
    use apchat_models::{Message, ModelColor};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tempfile::TempDir;
    use apchat_policy::PolicyManager;
    use apchat_toolcore::ToolRegistry;
    use apchat_terminal::TerminalManager;
    use apchat_todo::TodoManager;

    async fn create_test_chat() -> APChat {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().to_path_buf();

        APChat {
            api_key: "test-key".to_string(),
            work_dir: work_dir.clone(),
            client: reqwest::Client::new(),
            messages: Vec::new(),
            current_model: ModelColor::GrnModel,
            total_tokens_used: 0,
            logger: None,
            tool_registry: ToolRegistry::new(),
            agent_coordinator: None,
            use_agents: false,
            client_config: crate::config::ClientConfig::new(),
            policy_manager: PolicyManager::new(),
            terminal_manager: Arc::new(Mutex::new(TerminalManager::new(work_dir))),
            skill_registry: None,
            non_interactive: false,
            todo_manager: Arc::new(TodoManager::new()),
            stream_responses: false,
            verbose: false,
            debug_level: 0,
            process_id: 12345, // Fixed for testing
            readline_history: None,
            content_limiter: None,
            mspc_channel: None,
            signal_sender: None,
            signal_receiver: None,
            confirmation_registry: None,
        }
    }

    #[tokio::test]
    async fn test_compact_command_exists() {
        // This test verifies that the /compact command is properly handled
        // and doesn't cause crashes when executed

        let mut chat = create_test_chat().await;

        // Add some messages to make compaction meaningful
        chat.messages.push(Message {
            role: "user".to_string(),
            content: "Test message for compaction".repeat(100),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning: None,
        });

        // The compact command should not crash and should preserve system messages
        let result = crate::chat::history::intelligent_compaction(&mut chat, 0).await;

        assert!(result.is_ok(), "Compaction should succeed");

        // Should still have at least the system message
        assert!(chat.messages.len() >= 1, "Should have at least system message after compaction");
    }
}
