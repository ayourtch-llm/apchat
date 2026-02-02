use std::sync::Arc;
use colored::Colorize;

use apchat_vty::{print_heart_yellow, print_heart_red};
use apchat_models::Message;
use apchat_vty::RequestGuard;

use crate::APChat;
use crate::mspc::{MspcChannel, MspcMessage};

/// Outcome of a single inference cycle.
pub enum InferenceOutcome {
    /// Inference completed successfully with this response text.
    Response(String),
    /// Inference was interrupted by the user (Ctrl-C or interrupt signal).
    /// An "[Interrupted by user]" assistant message has already been pushed.
    Interrupted,
    /// Inference failed with an error.
    /// An "[Error: ...]" assistant message has already been pushed.
    Error,
}

/// Run a single inference cycle with interrupt handling.
///
/// Races the LLM inference future against interrupt signals arriving on the
/// MSPC channel using `tokio::select!`.  On interruption or error, an
/// assistant message is pushed to maintain turn alternation before returning.
pub async fn run_inference(
    chat: &mut APChat,
    input: &str,
    cancel_token: &tokio_util::sync::CancellationToken,
    mspc_channel: &Arc<MspcChannel>,
) -> InferenceOutcome {
    let _request_guard = RequestGuard::new();
    run_chat_inference(chat, input, cancel_token, mspc_channel).await
}

async fn run_chat_inference(
    chat: &mut APChat,
    input: &str,
    cancel_token: &tokio_util::sync::CancellationToken,
    mspc_channel: &Arc<MspcChannel>,
) -> InferenceOutcome {
    loop {
        tokio::select! {
            result = crate::chat::session::chat(chat, input, Some(cancel_token.clone())) => {
                match result {
                    Ok(response) => {
                        return InferenceOutcome::Response(response);
                    }
                    Err(e) if e.to_string().contains("interrupted") => {
                        print_heart_yellow(&format!("{} Unexpected interruption: {}", "⚠️".yellow(), e), true);
                        chat.messages.push(Message {
                            role: "assistant".to_string(),
                            content: format!("[Interrupted: {}]", e),
                            tool_calls: None,
                            tool_call_id: None,
                            name: None,
                            reasoning: None,
                        });
                        return InferenceOutcome::Interrupted;
                    }
                    Err(e) => {
                        print_heart_yellow(&format!("{} {}\n", "Error:".bright_red().bold(), e), true);
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
            interrupt_msg = mspc_channel.recv() => {
                if let Some(msg) = interrupt_msg {
                    match msg {
                        MspcMessage::InterruptSignal(_content, _sender) => {
                            print_heart_red(&format!("\n{}", "^C - Interrupting current operation...".bright_yellow()), true);
                            cancel_token.cancel();
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
                        _ => {
                            // Non-interrupt message - ignore during inference
                        }
                    }
                } else {
                    return InferenceOutcome::Response(String::new());
                }
            }
        }
    }
}
