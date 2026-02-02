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

use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use apchat_llm_api::client::LlmClient;
use apchat_llm_api::{ChatMessage, ToolDefinition};
use apchat_models::{ModelColor, Usage, ToolCall, FunctionCall};
use apchat_toolcore::parse_xml_tool_calls;

/// A chunk of streaming output from the LLM.
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
    /// Messages to send to the LLM
    pub messages: Vec<ChatMessage>,
    /// Which model color to use
    pub model: ModelColor,
    /// The LLM client to use for this request
    pub llm_client: Arc<dyn LlmClient>,
    /// Tool definitions available to the LLM
    pub tools: Vec<ToolDefinition>,
    /// Cancellation token - REPL sets before sending request
    pub cancel_token: CancellationToken,
    /// Whether to use streaming mode
    pub stream: bool,
    /// Optional channel for streaming chunks (created per-request when streaming enabled)
    pub stream_sender: Option<mpsc::Sender<StreamChunk>>,
}

/// Response from the LLM task after processing a request.
#[derive(Debug)]
pub enum LLMResponse {
    /// Successful response with content and optional tool calls
    Success {
        /// The response content
        content: String,
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
        let response = process_llm_request(request).await;
        if response_tx.send(response).await.is_err() {
            // Response channel closed, exit the task
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
        return LLMResponse::Interrupted;
    }

    // Race the API call against cancellation
    tokio::select! {
        result = make_llm_call(&request) => {
            result
        }
        _ = request.cancel_token.cancelled() => {
            LLMResponse::Interrupted
        }
    }
}

/// Make the actual LLM API call.
async fn make_llm_call(request: &LLMRequest) -> LLMResponse {
    if request.stream {
        make_streaming_call(request).await
    } else {
        make_non_streaming_call(request).await
    }
}

/// Make a non-streaming LLM API call.
async fn make_non_streaming_call(request: &LLMRequest) -> LLMResponse {
    match request.llm_client.chat(request.messages.clone(), request.tools.clone()).await {
        Ok(response) => {
            // Convert tool calls from LLM API format to our format
            let mut tool_calls = response.message.tool_calls.map(|calls| {
                calls
                    .into_iter()
                    .map(|call| ToolCall {
                        id: call.id,
                        tool_type: "function".to_string(),
                        function: FunctionCall {
                            name: call.function.name,
                            arguments: call.function.arguments,
                        },
                    })
                    .collect()
            });

            let mut content = response.message.content.clone();

            // If no structured tool calls were received, check for XML format in content
            if tool_calls.is_none() {
                if let Some(parsed_calls) = parse_xml_tool_calls(&content) {
                    tool_calls = Some(parsed_calls);
                    // Clear the XML from content to avoid displaying it
                    content = String::new();
                }
            }

            let usage = response.usage.map(|u| Usage {
                prompt_tokens: u.prompt_tokens as usize,
                completion_tokens: u.completion_tokens as usize,
                total_tokens: u.total_tokens as usize,
            });

            LLMResponse::Success {
                content,
                usage,
                tool_calls,
                model: request.model.clone(),
            }
        }
        Err(e) => LLMResponse::Error(e.to_string()),
    }
}

/// Make a streaming LLM API call.
async fn make_streaming_call(request: &LLMRequest) -> LLMResponse {
    use futures::StreamExt;
    use apchat_llm_api::client::ToolCallEvent;

    match request.llm_client.chat_streaming(request.messages.clone(), request.tools.clone()).await {
        Ok(mut stream) => {
            let mut accumulated_content = String::new();
            let mut accumulated_tool_calls: Vec<ToolCall> = Vec::new();
            let mut tool_calls_in_progress: std::collections::HashMap<usize, (String, String, String)> =
                std::collections::HashMap::new();

            while let Some(chunk_result) = stream.next().await {
                // Check for cancellation during streaming
                if request.cancel_token.is_cancelled() {
                    return LLMResponse::Interrupted;
                }

                match chunk_result {
                    Ok(chunk) => {
                        // Accumulate content
                        if !chunk.delta.is_empty() {
                            accumulated_content.push_str(&chunk.delta);

                            // Send streaming chunk if channel is available
                            if let Some(ref sender) = request.stream_sender {
                                let _ = sender
                                    .send(StreamChunk {
                                        delta: chunk.delta.clone(),
                                        is_final: false,
                                    })
                                    .await;
                            }
                        }

                        // Handle tool call events
                        if let Some(ref tool_event) = chunk.tool_call_event {
                            match tool_event {
                                ToolCallEvent::Start { index, id, name } => {
                                    tool_calls_in_progress
                                        .insert(*index, (id.clone(), name.clone(), String::new()));
                                }
                                ToolCallEvent::Delta { index, arguments_delta } => {
                                    if let Some((_id, _name, args)) =
                                        tool_calls_in_progress.get_mut(index)
                                    {
                                        args.push_str(arguments_delta);
                                    }
                                }
                            }
                        }

                        // Check if we're done
                        if let Some(ref reason) = chunk.finish_reason {
                            if reason == "stop" || reason == "end_turn" || reason == "tool_use" {
                                // Finalize tool calls
                                for (_index, (id, name, arguments)) in &tool_calls_in_progress {
                                    if !id.is_empty() && !name.is_empty() {
                                        accumulated_tool_calls.push(ToolCall {
                                            id: id.clone(),
                                            tool_type: "function".to_string(),
                                            function: FunctionCall {
                                                name: name.clone(),
                                                arguments: arguments.clone(),
                                            },
                                        });
                                    }
                                }

                                // Send final chunk
                                if let Some(ref sender) = request.stream_sender {
                                    let _ = sender
                                        .send(StreamChunk {
                                            delta: String::new(),
                                            is_final: true,
                                        })
                                        .await;
                                }

                                break;
                            }
                        }
                    }
                    Err(e) => {
                        return LLMResponse::Error(format!("Streaming error: {}", e));
                    }
                }
            }

            // If no structured tool calls were received, check for XML format in content
            let (final_content, final_tool_calls) = if accumulated_tool_calls.is_empty() {
                if let Some(parsed_calls) = parse_xml_tool_calls(&accumulated_content) {
                    // Clear the XML from content to avoid displaying it
                    (String::new(), Some(parsed_calls))
                } else {
                    (accumulated_content, None)
                }
            } else {
                (accumulated_content, Some(accumulated_tool_calls))
            };

            LLMResponse::Success {
                content: final_content,
                usage: None, // Usage not available in streaming mode
                tool_calls: final_tool_calls,
                model: request.model.clone(),
            }
        }
        Err(e) => LLMResponse::Error(e.to_string()),
    }
}
