mod init;
mod commands;
mod input_router;
mod inference;
pub mod llm_task;

use anyhow::Result;
use colored::Colorize;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;

use apchat_vty::{print_heart_yellow, print_heart_red};
use apchat_models::ModelColor;
use apchat_policy::PolicyManager;

use crate::APChat;
use crate::api::OutputChunk;
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

        // Use the tool loop (calls existing API functions directly)
        let outcome = run_tool_loop(
            &mut chat,
            line,
            &cancel_token,
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

                // Response display is now handled by run_tool_loop
                // Just add a separator after streaming output
                if chat.stream_responses {
                    print_heart_red(&format!(""), true);
                }
            }

            InferenceOutcome::Interrupted | InferenceOutcome::Error => {
                // Error/interrupted messages were already pushed by run_tool_loop
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

/// Run the tool-calling loop using the LLM task channels.
///
/// This implementation spawns an LLM task and uses it for all API calls,
/// enabling proper cancellation handling and channel-based communication.
pub(crate) async fn run_tool_loop(
    chat: &mut APChat,
    input: &str,
    cancel_token: &tokio_util::sync::CancellationToken,
) -> InferenceOutcome {
    use apchat_vty::print_heart_yellow;
    use apchat_models::Message;
    use crate::app::repl::llm_task::{spawn_llm_task, LLMRequest, LLMResponse};

    // Spawn the LLM task
    let mut llm_channels = spawn_llm_task();

    // Prepare for LLM call (add user message, summarize history)
    chat.messages.push(Message {
        role: "user".to_string(),
        content: input.to_string(),
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning: None,
    });

    crate::chat::history::summarize_and_trim_history(chat).await;

    // Tool loop configuration
    let mut tool_call_iterations = 0;
    let mut recent_tool_calls: Vec<(String, String)> = Vec::new();
    const MAX_TOOL_ITERATIONS: usize = 250;
    const LOOP_DETECTION_WINDOW: usize = 8;

    // Track total tokens for the session
    let total_tokens_start = chat.total_tokens_used;

    loop {
        // Check for cancellation
        if cancel_token.is_cancelled() {
            print_heart_yellow(&format!("{} {}", "⚠️".yellow(), "Interrupted by user"), true);
            chat.messages.push(Message {
                role: "assistant".to_string(),
                content: "[Interrupted by user]".to_string(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                reasoning: None,
            });
            return InferenceOutcome::Interrupted;
        }

        // Validate and fix tool calls in the conversation history
        if let Ok(fixed) = crate::tools_execution::validation::validate_and_fix_tool_calls_in_place(chat) {
            if fixed {
                print_heart_yellow(&format!("{} {}", "✅".green(), "Tool calls were automatically fixed in conversation history"), true);
            }
        }

        // Create API call parameters
        let params = crate::api::ApiCallParams {
            messages: chat.messages.clone(),
            current_model: chat.current_model.clone(),
            client_config: chat.client_config.clone(),
            api_key: chat.api_key.clone(),
            tools: chat.get_tools(),
            stream_responses: chat.stream_responses,
            verbose: chat.verbose,
            debug_level: chat.debug_level,
            http_client: chat.client.clone(),
        };

        // Prepare streaming channel if needed
        let stream_sender = if chat.stream_responses {
            let (tx, mut rx) = tokio::sync::mpsc::channel::<crate::api::OutputChunk>(100);
            tokio::spawn(async move {
                while let Some(chunk) = rx.recv().await {
                    print_heart_red(&format!("{}", chunk.text), false);
                }
            });
            Some(tx)
        } else {
            None
        };

        // Send request to LLM task
        let request = LLMRequest {
            params,
            cancel_token: cancel_token.clone(),
            stream_sender,
        };

        if let Err(_) = llm_channels.request_tx.send(request).await {
            print_heart_yellow(&format!("{} {}", "❌".bright_red(), "Failed to send request to LLM task"), true);
            return InferenceOutcome::Error;
        }

        // Wait for response from LLM task
        let llm_response = match llm_channels.response_rx.recv().await {
            Some(response) => response,
            None => {
                print_heart_yellow(&format!("{} {}", "❌".bright_red(), "LLM task channel closed unexpectedly"), true);
                return InferenceOutcome::Error;
            }
        };

        // Process the LLM response
        match llm_response {
            LLMResponse::Success {
                content,
                usage,
                tool_calls,
                model: current_model,
            } => {
                // Update model if it changed
                if chat.current_model != current_model {
                    print_heart_red(&format!("Model switched: {:?} -> {:?}", &chat.current_model, &current_model), true);
                    chat.current_model = current_model;
                }

                // Display token usage
                if let Some(usage) = &usage {
                    chat.total_tokens_used = total_tokens_start + usage.total_tokens;
                    print_heart_red(&format!("📊 Prompt: {} | Completion: {} | Total: {} | Session: {}",
                        usage.prompt_tokens,
                        usage.completion_tokens,
                        usage.total_tokens,
                        chat.total_tokens_used), true);
                }

                // Handle tool calls
                if let Some(calls) = tool_calls {
                    tool_call_iterations += 1;

                    if tool_call_iterations > MAX_TOOL_ITERATIONS {
                        print_heart_yellow(&format!("{} {} tool calls - stopping to avoid infinite loop.",
                            "⚠️".yellow(), tool_call_iterations), true);
                        chat.messages.push(Message {
                            role: "assistant".to_string(),
                            content: format!("[Stopped after {} tool calls]", tool_call_iterations),
                            tool_calls: None,
                            tool_call_id: None,
                            name: None,
                            reasoning: None,
                        });
                        return InferenceOutcome::Response("".to_string());
                    }

                    // Loop detection
                    let current_signature = calls.iter()
                        .map(|tc| format!("{}:{}", tc.function.name, tc.function.arguments))
                        .collect::<Vec<_>>()
                        .join("|");

                    recent_tool_calls.push((current_signature.clone(), String::new()));
                    if recent_tool_calls.len() > LOOP_DETECTION_WINDOW {
                        recent_tool_calls.remove(0);
                    }

                    let consecutive_count = recent_tool_calls.iter()
                        .rev()
                        .take_while(|(sig, _)| sig == &current_signature)
                        .count();

                    if consecutive_count >= LOOP_DETECTION_WINDOW {
                        print_heart_yellow(&format!("{} Detected infinite tool call loop - stopping.",
                            "🔄".yellow()), true);
                        chat.messages.push(Message {
                            role: "assistant".to_string(),
                            content: "[Detected tool call loop, stopping]".to_string(),
                            tool_calls: None,
                            tool_call_id: None,
                            name: None,
                            reasoning: None,
                        });
                        return InferenceOutcome::Response("".to_string());
                    }

                    // Execute tools
                    let assistant_message = Message {
                        role: "assistant".to_string(),
                        content: String::new(), // Will be filled if there's also text content
                        tool_calls: Some(calls.clone()),
                        tool_call_id: None,
                        name: None,
                        reasoning: None,
                    };

                    chat.messages.push(assistant_message.clone());

                    if chat.verbose {
                        print_heart_yellow(&format!("{} Executing {} tool(s)...",
                            "🔧".bright_yellow(), calls.len()), true);
                    }

                    // Execute each tool call
                    for tool_call in &calls {
                        let tool_result = match chat.execute_tool(
                            &tool_call.function.name,
                            &tool_call.function.arguments,
                        ).await {
                            Ok(r) => r,
                            Err(e) => {
                                let error_msg = format!("Error executing tool {}: {}", &tool_call.function.name, e);
                                print_heart_yellow(&format!("{} {}", "❌".bright_red(), &error_msg), true);
                                error_msg
                            }
                        };

                        let tool_response_message = Message {
                            role: "tool".to_string(),
                            content: tool_result,
                            tool_calls: None,
                            tool_call_id: Some(tool_call.id.clone()),
                            name: Some(tool_call.function.name.clone()),
                            reasoning: None,
                        };

                        chat.messages.push(tool_response_message);
                    }

                    print_heart_red("", true); // New line after tool outputs

                    // Continue the loop to get the next response
                } else {
                    // No tool calls - this is the final response
                    let final_message = Message {
                        role: "assistant".to_string(),
                        content: content.clone(),
                        tool_calls: None,
                        tool_call_id: None,
                        name: None,
                        reasoning: None,
                    };

                    chat.messages.push(final_message);
                    return InferenceOutcome::Response(content);
                }
            }
            LLMResponse::Interrupted => {
                print_heart_yellow(&format!("{} {}", "⚠️".yellow(), "LLM call interrupted"), true);
                chat.messages.push(Message {
                    role: "assistant".to_string(),
                    content: "[Interrupted]".to_string(),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                    reasoning: None,
                });
                return InferenceOutcome::Interrupted;
            }
            LLMResponse::Error(e) => {
                print_heart_yellow(&format!("{} {}: {}", "❌".bright_red(), "LLM API Error", e), true);
                chat.messages.push(Message {
                    role: "assistant".to_string(),
                    content: format!("[Error: {}]", e),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                    reasoning: None,
                });
                return InferenceOutcome::Error;
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
