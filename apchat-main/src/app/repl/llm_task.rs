//! LLM Task Module
//!
//! This module implements the LLM task - a stateless service that processes
//! single LLM API calls and can be cleanly interrupted via CancellationToken.
//!
//! The LLM task:
//! - Runs as a long-lived tokio task
//! - Monitors request channel for LLMRequest messages
//! - Makes ONE LLM API call per request
//! - Returns raw response (content, usage, tool_calls)
//! - Can be interrupted via CancellationToken
//! - Has NO access to APChat state - completely stateless

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::api::{ApiCallParams, OutputChunk};
use apchat_models::{ModelColor, Usage, ToolCall};
use apchat_models::types::ContentPart;
use apchat_vty::print_heart_yellow;

/// A chunk of streaming output from the LLM (internal type)
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// Incremental text delta
    pub delta: String,
    /// True when stream is complete
    pub is_final: bool,
}

/// Request to the LLM task for a single API call.
#[derive(Debug)]
pub struct LLMRequest {
    /// Parameters for the API call (includes all needed state)
    pub params: ApiCallParams,
    /// Cancellation token - REPL sets before sending request
    pub cancel_token: CancellationToken,
    /// Optional channel for streaming chunks (created per-request when streaming enabled)
    pub stream_sender: Option<mpsc::Sender<OutputChunk>>,
    /// Whether inference debug output should be shown
    pub inference_debug: bool,
}

/// Response from the LLM task after processing a request.
#[derive(Debug, Clone)]
pub enum LLMResponse {
    /// Successful response with content and optional tool calls
    Success {
        /// The response content
        content: Vec<ContentPart>,
        /// Token usage statistics
        usage: Option<Usage>,
        /// Tool calls requested by the LLM
        tool_calls: Option<Vec<ToolCall>>,
        /// Model that was used
        model: ModelColor,
    },
    /// Request was interrupted by user (Ctrl-C)
    Interrupted,
    /// Request failed with an error
    Error(String),
}

/// Channels for communicating with the LLM task.
pub struct LLMTaskChannels {
    /// Send requests to the LLM task
    pub request_tx: mpsc::Sender<LLMRequest>,
    /// Receive responses from the LLM task
    pub response_rx: mpsc::Receiver<LLMResponse>,
}

/// Spawn the LLM task and return channels for communication.
///
/// The LLM task runs as a long-lived tokio task that continuously
/// monitors the request channel and processes LLM API calls.
pub fn spawn_llm_task() -> LLMTaskChannels {
    // Buffer of 1: only one request in flight at a time
    let (request_tx, request_rx) = mpsc::channel::<LLMRequest>(1);
    // Buffer of 1: one response per request
    let (response_tx, response_rx) = mpsc::channel::<LLMResponse>(1);

    tokio::spawn(async move {
        llm_task_loop(request_rx, response_tx).await;
    });

    LLMTaskChannels {
        request_tx,
        response_rx,
    }
}

/// The main loop for the LLM task.
///
/// Continuously receives requests and processes them, sending responses back.
async fn llm_task_loop(
    mut request_rx: mpsc::Receiver<LLMRequest>,
    response_tx: mpsc::Sender<LLMResponse>,
) {
    while let Some(request) = request_rx.recv().await {
        let inference_debug = request.inference_debug;
        if inference_debug {
            print_heart_yellow(&format!("📨 [LLM TASK] Received request from channel"), true);
        }
        let response = process_llm_request(request).await;
        if inference_debug {
            print_heart_yellow(&format!("📨 [LLM TASK] Sending response: {:?}", match &response {
                LLMResponse::Success { .. } => "Success",
                LLMResponse::Interrupted => "Interrupted",
                LLMResponse::Error(_) => "Error",
            }), true);
        }
        if response_tx.send(response).await.is_err() {
            // Response channel closed, exit the task
            print_heart_yellow(&format!("❌ [LLM TASK] Response channel closed, exiting"), true);
            break;
        }
    }
}

/// Process a single LLM request.
///
/// Makes the API call while respecting the cancellation token.
async fn process_llm_request(request: LLMRequest) -> LLMResponse {
    // Check if already cancelled before starting
    if request.cancel_token.is_cancelled() {
        use apchat_vty::print_heart_yellow;
        if request.inference_debug {
            print_heart_yellow(&format!("🚫 [LLM TASK] Request already cancelled before starting"), true);
        }
        return LLMResponse::Interrupted;
    }

    // Race the API call against cancellation
    if request.inference_debug {
        print_heart_yellow(&format!("🚀 [LLM TASK] Starting API call"), true);
    }
    tokio::select! {
        result = make_llm_call(&request) => {
            if request.inference_debug {
                print_heart_yellow(&format!("✅ [LLM TASK] API call completed"), true);
            }
            result
        }
        _ = request.cancel_token.cancelled() => {
            if request.inference_debug {
                print_heart_yellow(&format!("🚫 [LLM TASK] Request cancelled during API call"), true);
            }
            LLMResponse::Interrupted
        }
    }
}

/// Make the actual LLM API call.
async fn make_llm_call(request: &LLMRequest) -> LLMResponse {
    use apchat_vty::print_heart_yellow;
    if request.params.stream_responses {
        if request.inference_debug {
            print_heart_yellow(&format!("📡 [LLM TASK] Making streaming API call"), true);
        }
        make_streaming_call(request).await
    } else {
        if request.inference_debug {
            print_heart_yellow(&format!("📡 [LLM TASK] Making non-streaming API call"), true);
        }
        make_non_streaming_call(request).await
    }
}

/// Make a non-streaming LLM API call.
async fn make_non_streaming_call(request: &LLMRequest) -> LLMResponse {
    use apchat_vty::print_heart_yellow;
    if request.inference_debug {
        print_heart_yellow(&format!("🔧 [LLM TASK] Calling call_api_stateless"), true);
    }
    // Use the stateless API function
    match crate::api::call_api_stateless(&request.params).await {
        Ok((message, usage, model, _finish_reason)) => {
            if request.inference_debug {
                print_heart_yellow(&format!("✅ [LLM TASK] call_api_stateless succeeded - content_len: {}, has_tool_calls: {}", message.content.len(), message.tool_calls.is_some()), true);
            }
            LLMResponse::Success {
                content: message.content,
                usage,
                tool_calls: message.tool_calls,
                model,
            }
        }
        Err(e) => {
            if request.inference_debug {
                print_heart_yellow(&format!("❌ [LLM TASK] call_api_stateless failed: {}", e), true);
            }
            LLMResponse::Error(e.to_string())
        }
    }
}

/// Make a streaming LLM API call.
async fn make_streaming_call(request: &LLMRequest) -> LLMResponse {
    use apchat_vty::print_heart_yellow;
    if request.inference_debug {
        print_heart_yellow(&format!("🔧 [LLM TASK] Calling call_api_streaming_stateless"), true);
    }
    // Use the stateless streaming API function
    match crate::api::call_api_streaming_stateless(&request.params, request.stream_sender.clone()).await {
        Ok((message, usage, model, _metrics)) => {
            if request.inference_debug {
                print_heart_yellow(&format!("✅ [LLM TASK] call_api_streaming_stateless succeeded - content_len: {}, has_tool_calls: {}", message.content.len(), message.tool_calls.is_some()), true);
            }
            LLMResponse::Success {
                content: message.content,
                usage,
                tool_calls: message.tool_calls,
                model,
            }
        }
        Err(e) => {
            if request.inference_debug {
                print_heart_yellow(&format!("❌ [LLM TASK] call_api_streaming_stateless failed: {}", e), true);
            }
            LLMResponse::Error(e.to_string())
        }
    }
}
