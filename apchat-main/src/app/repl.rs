#![deny(unused_must_use)]
mod init;
mod commands;
mod input_router;
pub mod llm_task;

use anyhow::Result;
use colored::Colorize;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;

use apchat_vty::{print_heart_yellow, print_heart_red, status_info};
use apchat_models::{ModelColor, Message};
use apchat_policy::PolicyManager;

use crate::APChat;
use crate::api::{OutputChunk, TypingIndicator};
use crate::cli::Cli;
use crate::config::ClientConfig;
use crate::mspc::{MspcChannel, MspcMessage, get_readline_receiver};

use crate::app::repl::llm_task::LLMTaskChannels;
use crate::app::repl::llm_task::spawn_llm_task;

use commands::CommandResult;
use apchat_types::InferenceOutcome;
use apchat_vty::request_counter::RequestGuard;
use crate::chat::history::calculate_conversation_size;
use crate::scheduled_instructions::poller::ScheduledInstructionPoller;

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


pub enum ApchatReplLoopState {
    Idle,
}

pub enum ApchatCommandResult {
    Continue,
    Break,
    DoInference(String)
}

async fn process_repl_command(chat: &mut APChat, message: MspcMessage, interrupt_sender_for_main: tokio::sync::mpsc::Sender<MspcMessage>, mspc_channel: &Arc<MspcChannel>, current_model_for_main: Arc<std::sync::RwLock<ModelColor>>) -> ApchatCommandResult {
        let _ = apchat_vty::print_heart_to_file(&format!("RCVD: {:?}", &message), true);

        // Route special message types before extracting the text payload
        let line = match message {
            MspcMessage::UserInput(content, _sender) => content,
            MspcMessage::Command(content, _sender)   => content,

            MspcMessage::InterruptSignal(content, sender) => {
                // Forward to tools, then inform user there's nothing to cancel
                let _ = interrupt_sender_for_main
                    .send(MspcMessage::InterruptSignal(content.clone(), sender)).await;
                print_heart_red(&format!("\n{}", "No operation in progress to interrupt".bright_yellow()), true);
                format!("!{}", content)
            }

            MspcMessage::ConfirmationRequest(content, _sender) => {
                handle_confirmation_request(&content, &mspc_channel).await;
                return ApchatCommandResult::Continue;
            }

            _ => {
                return ApchatCommandResult::Continue;
            }
        };

        let line = line.trim();
        if line.is_empty() {
            return ApchatCommandResult::Continue;
        }

        // Exit commands
        if line == "exit" || line == "quit" {
            print_heart_red(&format!("{}", "Goodbye - exit command!".bright_cyan()), true);
            if let Err(e) = apchat_vty::ReadlineInstance::save_history() {
                if chat.debug_level > 0 {
                    print_heart_yellow(&format!("{} Failed to save readline history: {}", "⚠️".yellow(), e), true);
                }
            }
            return ApchatCommandResult::Break;
        }

        // Slash-command dispatch
        match commands::dispatch_command(chat, line, &current_model_for_main).await {
            CommandResult::Handled    => return ApchatCommandResult::Continue,
            CommandResult::NotACommand => {} // fall through to inference
        }

        // Persist input, log it, and auto-save chat state
        save_input_and_log(chat, line).await;
        ApchatCommandResult::DoInference(line.to_string())
}

fn get_urgent_input(urgent_messages: &mut Vec<String>) -> String {
    let mut urgent_input = format!("START URGENT:\n");
    while let Some(urgent_msg) = urgent_messages.pop() {
      print_heart_yellow(&format!("Injecting urgent message: {:?}", &urgent_msg), true);
      // Preserve the ! prefix for urgent messages
      urgent_input.push_str(&format!("    {}\n", urgent_msg));
    }
    urgent_input.push_str("END URGENT\n");
    urgent_input
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
    mspc_channel_opt: Option<(std::sync::Arc<MspcChannel>, std::sync::Arc<apchat_webex::WebexClient>, String)>,
) -> Result<()> {
    // ── Initialization ─────────────────────────────────────────────────────
    let (mut chat, idle_config) =
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

    let mut repl_state = ApchatReplLoopState::Idle;

    // ── MSPC channel & confirmation plumbing ───────────────────────────────
    let mspc_channel = mspc_channel_opt.as_ref().map(|(c, _, _)| c.clone()).unwrap_or_else(|| Arc::new(MspcChannel::new(100)));
    chat.mspc_channel = Some(mspc_channel.clone());

    // Register Webex messaging tools if Webex is configured
    if let Some((_, webex_client, authorized_email)) = &mspc_channel_opt {
        crate::config::register_webex_tools(&mut chat.tool_registry, webex_client.clone(), authorized_email.clone());
    }

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
        idle_config,
    });

    let interrupt_sender_for_main = interrupt_sender;

    // ── Scheduled instruction poller ───────────────────────────────────────
    // Create and start the scheduled instruction poller only if enabled
    let poller_handle = if cli.delayed_instructions {
        print_heart_yellow(&format!("{} Scheduled instructions enabled - starting poller", "⏰".bright_cyan()), true);
        let db_path = apchat_tools::memory::get_memory_db_path();
        let mut scheduled_instruction_poller = ScheduledInstructionPoller::new(db_path);
        scheduled_instruction_poller.set_channel(mspc_channel.clone());
        Some(scheduled_instruction_poller.start())
    } else {
        print_heart_yellow(&format!("{} Scheduled instructions disabled", "⏰".bright_black()), true);
        None
    };

    // Spawn the LLM task
    let mut llm_channels = spawn_llm_task();
    let mut queued_messages: Vec<String> = vec![];
    let mut urgent_messages: Vec<String> = vec![];

    let mut llm_running = false;
    let mut request_guard = None;
    // Tool loop configuration
    let mut tool_call_iterations = 0;
    let mut recent_tool_calls: Vec<(String, String)> = Vec::new();
    let mut total_tokens_start = chat.total_tokens_used;
    
    // Empty response retry configuration
    const MAX_EMPTY_RESPONSE_RETRIES: usize = 3;
    let mut empty_response_retries: usize = 0;

    // ── Main REPL loop ─────────────────────────────────────────────────────
    'outer: loop {
        // ── Inference with interrupt support ───────────────────────────────
        if !llm_running {
            // First, inject urgent messages in FIFO order before regular queued messages
            if !urgent_messages.is_empty() {
                let mut urgent_input = get_urgent_input(&mut urgent_messages);

                let cancel_token = tokio_util::sync::CancellationToken::new();
                {
                    let mut guard = current_token.lock().unwrap();
                    *guard = Some(cancel_token.clone());
                }
                add_msg_to_history(&mut chat, &mut llm_channels, &urgent_input, &cancel_token).await;
		tool_call_iterations = 0;
                total_tokens_start = chat.total_tokens_used;
        if prep_and_send_request(&mut chat, &mut llm_channels, &cancel_token, None, webex_sink.as_ref()).await {
            if chat.get_inference_debug() {
                print_heart_yellow(&format!("✅ [DEBUG] Request sent successfully - creating RequestGuard"), true);
            }
            llm_running = true;
            request_guard = Some(RequestGuard::new());
        } else {
            print_heart_yellow(&format!("❌ [DEBUG] Failed to send request - not creating RequestGuard"), true);
            print_heart_yellow(&format!("Error running inference on urgent message: {:?}", &urgent_input), true);
                }
            } else if let Some(input) = queued_messages.pop() {
                print_heart_yellow(&format!("Got new pending input: {:?}", &input), true);
                let cancel_token = tokio_util::sync::CancellationToken::new();
                {
                    let mut guard = current_token.lock().unwrap();
                    *guard = Some(cancel_token.clone());
                }
		tool_call_iterations = 0;
                add_msg_to_history(&mut chat, &mut llm_channels, &input, &cancel_token).await;
                total_tokens_start = chat.total_tokens_used;
                if prep_and_send_request(&mut chat, &mut llm_channels, &cancel_token, None, webex_sink.as_ref()).await {
                    if chat.get_inference_debug() {
                        print_heart_yellow(&format!("✅ [DEBUG] Request sent successfully - creating RequestGuard"), true);
                    }
                    llm_running = true;
                    request_guard = Some(RequestGuard::new());
                } else {
                    print_heart_yellow(&format!("❌ [DEBUG] Failed to send request - not creating RequestGuard"), true);
                    print_heart_yellow(&format!("Error running inference on: {:?}", &input), true);
                }
                
            }
        }

        update_status_info(&chat, &queued_messages);

        if chat.debug_level > 0 {
            print_heart_yellow(&format!("Select start"), true);
        }
        tokio::select! {
            llm_response_res = llm_channels.response_rx.recv() => {
                print_heart_yellow(&format!("📨 [DEBUG] LLM response received from channel"), true);
                llm_running = false;
                request_guard = None;
                if chat.get_inference_debug() {
                    print_heart_yellow(&format!("📉 [DEBUG] request_guard set to None - counter should decrement"), true);
                }
                // Always clear the cancellation token, regardless of outcome
                {
                    let mut guard = current_token.lock().unwrap();
                    *guard = None;
                }
                let llm_response = match llm_response_res {
                    Some(response) => response,
                    None => {
                        print_heart_yellow(&format!("{} {}", "❌".bright_red(), "LLM task channel closed unexpectedly"), true);
                        continue 'outer;
                        // return InferenceOutcome::Error;
                    }
                };
                let outcome = process_llm_response(&mut chat, &mut llm_channels, llm_response, &mut recent_tool_calls, &mut tool_call_iterations, total_tokens_start).await;
                apchat_vty::print_outcome_box("process_llm_response outcome", &outcome);
                match outcome {
                    InferenceOutcome::Response(response) => {
                        print_heart_yellow(&format!("✅ [DEBUG] InferenceOutcome::Response - response length: {}", response.len()), true);
                        
                        // Check for empty response and apply retry logic
                        if response.trim().is_empty() {
                            empty_response_retries += 1;
                            print_heart_yellow(&format!("⚠️ [DEBUG] Empty response detected! Retry attempt {}/{}", empty_response_retries, MAX_EMPTY_RESPONSE_RETRIES), true);
                            
                            if empty_response_retries >= MAX_EMPTY_RESPONSE_RETRIES {
                                print_heart_yellow(&format!("❌ [DEBUG] Max empty response retries ({}) exceeded - treating as error", MAX_EMPTY_RESPONSE_RETRIES), true);
                                empty_response_retries = 0; // Reset for next time
                                continue 'outer;
                            }
                            
                            // Retry: push request again without consuming user input
                            let cancel_token = tokio_util::sync::CancellationToken::new();
                            {
                                let mut guard = current_token.lock().unwrap();
                                *guard = Some(cancel_token.clone());
                            }
                            let maybe_urgent_input = if urgent_messages.is_empty() {
                                None
                            } else {
                                Some(get_urgent_input(&mut urgent_messages))
                            };
                            
                            // Add a bogus assistant message and encouragement before retry
                            // This helps the LLM continue when it produces empty responses
                            chat.messages.push(Message {
                                role: "assistant".to_string(),
                                content: "".to_string(),
                                tool_calls: None,
                                tool_call_id: None,
                                name: None,
                                reasoning: None,
                            });
                            chat.messages.push(Message {
                                role: "user".to_string(),
                                content: "You are doing great, please continue!".to_string(),
                                tool_calls: None,
                                tool_call_id: None,
                                name: None,
                                reasoning: None,
                            });
                            
                            // Remove trailing empty assistant message before retry to avoid duplicate
                            // ensure_proper_role_alternation will add a fresh one if needed
                            if let Some(last_msg) = chat.messages.last() {
                                if last_msg.role == "assistant" && last_msg.content.is_empty() {
                                    chat.messages.pop();
                                }
                            }
                            
                            if prep_and_send_request(&mut chat, &mut llm_channels, &cancel_token, maybe_urgent_input, webex_sink.as_ref()).await {
                                print_heart_yellow(&format!("✅ [DEBUG] Empty response retry {} - request sent successfully", empty_response_retries), true);
                                llm_running = true;
                                request_guard = Some(RequestGuard::new());
                                continue; // Continue the select loop to wait for retry response
                            } else {
                                print_heart_yellow(&format!("❌ [DEBUG] Empty response retry {} - failed to send request", empty_response_retries), true);
                                empty_response_retries = 0;
                                continue 'outer;
                            }
                        }
                        
                        // Normal response - reset retry counter
                        empty_response_retries = 0;
                        
                        // Log assistant response (only for non-empty responses)
                        if let Some(logger) = &mut chat.logger {
                            logger.log("assistant", &response, None, false).await;
                        }

                        // Stop Webex typing indicator and broadcast response
                        if let Some(ref webex) = webex_sink {
                            if !chat.feature_flags.disable_webex_broadcast {
                                // Stop the typing indicator first
                                if let Err(e) = webex.stop_typing().await {
                                    print_heart_yellow(&format!("{} Failed to stop Webex typing indicator: {}", "⚠️".yellow(), e), true);
                                }
                                // Then send the actual response
                                if let Err(e) = webex.send_response(&response).await {
                                    print_heart_yellow(&format!("{} Failed to send to Webex: {}", "⚠️".yellow(), e), true);
                                }
                            } else {
                                print_heart_yellow("🔕 Webex broadcast disabled (--disable-webex-broadcast)", true);
                            }
                        }
                    }

                    InferenceOutcome::Interrupted | InferenceOutcome::Error => {
                        print_heart_yellow(&format!("🚫 [DEBUG] InferenceOutcome::Interrupted or Error - continuing outer loop"), true);
                        // Reset empty response retries on error/interrupt
                        empty_response_retries = 0;
                        // Error/interrupted messages were already pushed by run_tool_loop
                        continue 'outer;
                    }
                    InferenceOutcome::ToolsContinue => {
                        if chat.get_inference_debug() {
                            print_heart_yellow(&format!("🔄 [DEBUG] InferenceOutcome::ToolsContinue - will repeat inference"), true);
                        }
                        // Reset empty response retries when we have a valid tool call response
                        empty_response_retries = 0;
                        
                        // Should push the request again
                        let cancel_token = tokio_util::sync::CancellationToken::new();
                        {
                            let mut guard = current_token.lock().unwrap();
                            *guard = Some(cancel_token.clone());
                        }
                        let maybe_urgent_input = if urgent_messages.is_empty() {
                            None
                        } else {
                            Some(get_urgent_input(&mut urgent_messages))
                        };

                        if prep_and_send_request(&mut chat, &mut llm_channels, &cancel_token, maybe_urgent_input, webex_sink.as_ref()).await {
                            if chat.debug_level > 0 {
                                print_heart_yellow(&format!("✅ [DEBUG] Repeat inference request sent successfully - creating RequestGuard"), true);
                            }
                            if chat.debug_level > 0 {
                                print_heart_yellow(&format!("Started repeat inference"), true);
                            }
                            llm_running = true;
                            request_guard = Some(RequestGuard::new());
                        } else {
                            print_heart_yellow(&format!("❌ [DEBUG] Failed to send repeat inference request"), true);
                            print_heart_yellow(&format!("Error running repeat inference"), true);
                        }
                    }
                }
            }
            mspc_message = mspc_channel.recv() => {
                // Wait for next message from the input router
                let message = match mspc_message {
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

                let line = match process_repl_command(&mut chat, message, interrupt_sender_for_main.clone(), &mspc_channel, current_model_for_main.clone()).await {
                    ApchatCommandResult::Continue => {
                        continue;
                    }
                    ApchatCommandResult::Break => {
                        break;
                    }
                    ApchatCommandResult::DoInference(line) => {
                        line
                    }
                };
                if line.starts_with("!") {
                    // Preserve the ! prefix for urgent messages
                    urgent_messages.push(line.to_string());
                } else {
                    queued_messages.push(line.to_string());
                }
            },
        }

        update_status_info(&chat, &queued_messages);

        if chat.debug_level > 0 {
            print_heart_yellow(&format!("Select end"), true);
        }
    }

    // ── Cleanup ────────────────────────────────────────────────────────────
    router_handle.abort();
    
    // Stop the scheduled instruction poller if it was started
    if let Some(poller_handle) = poller_handle {
        poller_handle.abort();
        let _ = poller_handle.await;
    }

    if let Some(logger) = &mut chat.logger {
        logger.shutdown().await;
    }

    // Restore terminal settings before exiting (critical for Ctrl-D handling)
    if let Err(e) = apchat_vty::ReadlineInstance::cleanup() {
        print_heart_yellow(&format!("Warning: Failed to cleanup readline instance: {}", e), true);
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

/// Update status info for title bar display based on current state
fn update_status_info(chat: &APChat, queued_messages: &[String]) {
    let context_size = calculate_conversation_size(&chat.messages);
    let history_count = chat.messages.len();
    let queued_count = queued_messages.len();
    status_info::set_queued(queued_count);
    status_info::set_history(history_count);
    status_info::set_context_bytes(context_size);
    status_info::set_urgent(0); // Reset urgent count - will be set by MSPC router if needed
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

    print_heart_red("", true);

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


async fn add_msg_to_history(
    chat: &mut APChat,
    llm_channels: &mut LLMTaskChannels,
    input: &str,
    cancel_token: &tokio_util::sync::CancellationToken,
) {
    use apchat_vty::print_heart_yellow;
    use apchat_models::Message;
    use crate::app::repl::llm_task::{spawn_llm_task, LLMRequest, LLMResponse};

    // Check for bogus_ack_msg deduplication logic
    if let Some(ref bogus_ack) = chat.bogus_ack_msg {
        // Remove spaces from bogus_ack for comparison
        let bogus_ack_normalized: String = bogus_ack.chars().filter(|c| !c.is_whitespace()).collect();
        
        // Check if we have at least 2 messages (previous user + last assistant)
        if chat.messages.len() >= 2 {
            // Clone the last assistant message content for comparison
            let last_assistant_content = chat.messages.last().map(|m| m.content.clone());
            
            if let Some(ref last_assistant_content) = last_assistant_content {
                if let Some(last_assistant_msg) = chat.messages.last() {
                    if last_assistant_msg.role == "assistant" {
                        // Remove spaces from last assistant message content for comparison
                        let last_assistant_normalized: String = last_assistant_content.chars().filter(|c| !c.is_whitespace()).collect();
                        
                        // Check if last assistant message matches bogus_ack (ignoring spaces)
                        if last_assistant_normalized == bogus_ack_normalized {
                            // Clone the previous user message content before we mutate
                            let prev_user_content = chat.messages.get(chat.messages.len() - 2).map(|m| m.content.clone());
                            
                            if let Some(ref prev_user_content) = prev_user_content {
                                if prev_user_content == input {
                                    // Deduplication condition met!
                                    // Remove the last assistant message
                                    chat.messages.pop();
                                    
                                    print_heart_yellow(&format!("🔄 [DEDUP] Deduplication triggered!"), true);
                                    print_heart_yellow(&format!("   📝 Last assistant message (removed): {:?}", last_assistant_content), true);
                                    print_heart_yellow(&format!("   📝 Previous user message (skipped): {:?}", prev_user_content), true);
                                    print_heart_yellow(&format!("   🎯 Bogus ack pattern: {:?}", bogus_ack), true);
                                    
                                    // Don't add the new user message - skip to return early
                                    let _ = crate::chat::history::summarize_and_trim_history(chat).await;
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Prepare for LLM call (add user message, summarize history)
    chat.messages.push(Message {
        role: "user".to_string(),
        content: input.to_string(),
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning: None,
    });

    let _ = crate::chat::history::summarize_and_trim_history(chat).await;

}

async fn prep_and_send_request(
    chat: &mut APChat,
    llm_channels: &mut LLMTaskChannels,
    cancel_token: &tokio_util::sync::CancellationToken,
    maybe_urgent_input: Option<String>,
    webex_sink: Option<&std::sync::Arc<apchat_webex::WebexOutputSink>>,
) -> bool {
    use apchat_vty::print_heart_yellow;
    use apchat_models::Message;
    use crate::app::repl::llm_task::{spawn_llm_task, LLMRequest, LLMResponse};

    if chat.get_inference_debug() {
        print_heart_yellow(&format!("📤 [DEBUG] prep_and_send_request called - messages count: {}", chat.messages.len()), true);
    }

    // Validate and fix tool calls in the conversation history
    if let Ok(fixed) = crate::tools_execution::validation::validate_and_fix_tool_calls_in_place(chat) {
        if fixed {
            print_heart_yellow(&format!("{} {}", "✅".green(), "Tool calls were automatically fixed in conversation history"), true);
        }
    }
    if let Some(urgent_input) = maybe_urgent_input {
        chat.messages.push(Message {
            role: "assistant".to_string(),
            content: "I notice there is some urgent messages from the user ?".to_string(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning: None,
        });
        chat.messages.push(Message {
            role: "user".to_string(),
            content: urgent_input,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning: None,
        });
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
        llm_overrides: Some(chat.llm_overrides.clone()),
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

    // Show typing indicator when starting inference (streaming mode)
    if chat.stream_responses {
        // Typing indicator removed
    }

    // Start Webex typing indicator if available
    if let Some(ref webex) = webex_sink {
        if chat.stream_responses && !chat.feature_flags.disable_webex_broadcast {
            if let Err(e) = webex.start_typing().await {
                print_heart_yellow(&format!("{} Failed to start Webex typing indicator: {}", "⚠️".yellow(), e), true);
            }
        }
    }

    // Send request to LLM task
    let request = LLMRequest {
        params,
        cancel_token: cancel_token.clone(),
        stream_sender,
        inference_debug: chat.get_inference_debug(),
    };

    if chat.get_inference_debug() {
        print_heart_yellow(&format!("📤 [DEBUG] Sending request to LLM task channel"), true);
    }
    if let Err(_) = llm_channels.request_tx.send(request).await {
        print_heart_yellow(&format!("{} {}", "❌".bright_red(), "Failed to send request to LLM task"), true);
        print_heart_yellow(&format!("❌ [DEBUG] Failed to send request - returning false"), true);
        return false;
    }
    if chat.get_inference_debug() {
        print_heart_yellow(&format!("✅ [DEBUG] Request sent successfully - returning true"), true);
    }
    true
}

async fn process_llm_response(
    chat: &mut APChat,
    llm_channels: &mut LLMTaskChannels,
    llm_response: crate::app::repl::llm_task::LLMResponse,
    recent_tool_calls: &mut Vec<(String, String)>,
    tool_call_iterations: &mut usize,
    total_tokens_start: usize,
) -> InferenceOutcome {
    use apchat_vty::print_heart_yellow;
    use apchat_models::Message;
    use crate::app::repl::llm_task::{spawn_llm_task, LLMRequest, LLMResponse};
    const MAX_TOOL_ITERATIONS: usize = 250;
    const LOOP_DETECTION_WINDOW: usize = 8;

    // Print newline before response content
    print_heart_red("", true);

    // Process the LLM response
        match llm_response {
            LLMResponse::Success {
                content,
                usage,
                tool_calls,
                model: current_model,
            } => {
                if chat.get_inference_debug() {
                    print_heart_yellow(&format!("🔍 [DEBUG] LLMResponse::Success received - content_len: {}, has_tool_calls: {}", content.len(), tool_calls.is_some()), true);
                }
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
                    if chat.get_inference_debug() {
                        print_heart_yellow(&format!("🔧 [DEBUG] Tool calls detected: {} call(s)", calls.len()), true);
                    }
                    *tool_call_iterations += 1;
                    if chat.get_inference_debug() {
                        print_heart_yellow(&format!("🔢 [DEBUG] tool_call_iterations = {}", tool_call_iterations), true);
                    }

                    if *tool_call_iterations > MAX_TOOL_ITERATIONS {
                        if chat.get_inference_debug() {
                            print_heart_yellow(&format!("⚠️ [DEBUG] MAX_TOOL_ITERATIONS exceeded: {} > {}", tool_call_iterations, MAX_TOOL_ITERATIONS), true);
                        }
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

                    print_heart_yellow(&format!("🔄 [DEBUG] consecutive_count = {} (threshold: {})", consecutive_count, LOOP_DETECTION_WINDOW), true);

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

                    let pre_batch_len = chat.messages.len();
                    chat.messages.push(assistant_message.clone());

                    if chat.verbose {
                        print_heart_yellow(&format!("{} Executing {} tool(s)...",
                            "🔧".bright_yellow(), calls.len()), true);
                    }

                    // Execute each tool call
                    for tool_call in &calls {
                        print_heart_red(&format!("🔧 [DEBUG] Executing tool: {} with args: {}", &tool_call.function.name, &tool_call.function.arguments), true);
                        print_heart_red(&format!("TOOL: {} {}", &tool_call.function.name, &tool_call.function.arguments), true);
                        let tool_result = {
                          let _tool_guard = apchat_vty::ToolGuard::new_with_tool_name(&tool_call.function.name);
                          print_heart_yellow(&format!("🔧 [DEBUG] ToolGuard created - counter incremented"), true);
                          match chat.execute_tool(
                            &tool_call.function.name,
                            &tool_call.function.arguments,
                          ).await {
                            Ok(r) => {
                              print_heart_yellow(&format!("✅ [DEBUG] Tool '{}' executed successfully", &tool_call.function.name), true);
                              r
                            },
                            Err(e) => {
                                let error_msg = format!("Error executing tool {}: {}", &tool_call.function.name, e);
                                print_heart_yellow(&format!("{} {}", "❌".bright_red(), &error_msg), true);
                                error_msg
                            }
                          }
                        };
                        print_heart_yellow(&format!("🔧 [DEBUG] ToolGuard dropped - counter decremented"), true);
                        print_heart_red(&format!("TOOL-RESULT: {}", &tool_result), true);

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

                    // Apply any pending context edits from self-edit tools
                    crate::chat::context_edit::apply_pending_context_edits(chat, pre_batch_len);

                    print_heart_red("", true); // New line after tool outputs

                    // Continue the loop to get the next response
                    if chat.get_inference_debug() {
                        print_heart_yellow(&format!("✅ [DEBUG] Returning InferenceOutcome::ToolsContinue"), true);
                    }
                    return InferenceOutcome::ToolsContinue;
                } else {
                    // No tool calls - this is the final response
                    print_heart_yellow(&format!("📝 [DEBUG] No tool calls - final response with content length: {}", content.len()), true);
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
                print_heart_yellow(&format!("🚫 [DEBUG] Returning InferenceOutcome::Interrupted"), true);
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
                print_heart_yellow(&format!("❌ [DEBUG] Returning InferenceOutcome::Error: {}", e), true);
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
            llm_overrides: Arc::new(std::sync::Mutex::new(None)),
            context_edits: Arc::new(std::sync::Mutex::new(Vec::new())),
            summarize_subagents: true,
            mcp_clients: Vec::new(),
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
