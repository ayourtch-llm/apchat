use anyhow::Result;
use colored::Colorize;
use std::time::{Instant, Duration};

use crate::APChat;
use apchat_models::{ModelColor, Message, Usage, ChatRequest, StreamChunk, ToolCall, FunctionCall};
use apchat_llm_api::{ToolDefinition, ChatMessage};
use apchat_llm_api::client::ToolCallEvent;
use apchat_logging::{log_request, log_request_to_file, log_response, log_response_to_file, log_raw_response_to_file, log_stream_chunk};
use apchat_toolcore::parse_xml_tool_calls;
use apchat_vty::{print_heart_yellow, print_heart_red};

/// Metrics for token generation rate tracking
#[derive(Debug, Clone)]
pub struct StreamingMetrics {
    pub start_time: Instant,
    pub total_tokens: usize,
    pub completion_tokens: usize,
    pub prompt_tokens: Option<usize>,
    pub duration: Option<Duration>,
}

impl StreamingMetrics {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            total_tokens: 0,
            completion_tokens: 0,
            prompt_tokens: None,
            duration: None,
        }
    }

    pub fn finish(&mut self) {
        self.duration = Some(self.start_time.elapsed());
    }

    pub fn tokens_per_second(&self) -> f64 {
        match self.duration {
            Some(duration) => {
                let seconds = duration.as_secs_f64();
                if seconds > 0.0 {
                    self.completion_tokens as f64 / seconds
                } else {
                    0.0
                }
            }
            None => 0.0,
        }
    }

    pub fn print_metrics(&self) {
        match self.duration {
            Some(duration) => {
                let seconds = duration.as_secs_f64();
                let tps = self.tokens_per_second();
                
                print_heart_red(&format!("\n{}", "📊 Streaming Metrics:".bright_blue()), true);
                print_heart_red(&format!("  ⏱️  Duration: {:.2}s", seconds), true);
                print_heart_red(&format!("  🪪 Completion Tokens: {}", self.completion_tokens), true);
                
                if let Some(prompt) = self.prompt_tokens {
                    print_heart_red(&format!("  📝 Prompt Tokens: {}", prompt), true);
                    print_heart_red(&format!("  🪪 Total Tokens: {}", self.completion_tokens + prompt), true);
                }
                
                print_heart_red(&format!("  ⚡ Rate: {:.1} tokens/sec", tps), true);
                
                if tps >= 100.0 {
                    print_heart_red(&format!("  🚀 Performance: Excellent"), true);
                } else if tps >= 50.0 {
                    print_heart_red(&format!("  👍 Performance: Good"), true);
                } else if tps >= 20.0 {
                    print_heart_red(&format!("  📈 Performance: Moderate"), true);
                } else {
                    print_heart_red(&format!("  🐌 Performance: Slow"), true);
                }
                
                print_heart_red(&format!("{}", "─".repeat(50).dimmed()), true);
            }
            None => {
                print_heart_red(&format!("\n{} Streaming metrics unavailable - duration not measured", "⚠️".yellow()), true);
            }
        }
    }
}

/// Handle streaming API response for Groq-style APIs
pub(crate) async fn call_api_streaming(
    chat: &APChat,
    orig_messages: &[Message],
) -> Result<(Message, Option<Usage>, ModelColor, Option<String>, StreamingMetrics)> {
    use std::io::{self, Write};
    use futures_util::StreamExt;

    let current_model = chat.current_model.clone();

    // Convert messages for Mistral/Ministral compatibility:
    // Ministral allows ONE optional system message at the start, then alternating user/assistant.
    // Merge all consecutive leading system messages into one, and strip reasoning field.
    let mut messages: Vec<Message> = Vec::new();
    let mut leading_system_contents: Vec<String> = Vec::new();
    let mut past_leading_systems = false;

    for m in orig_messages.iter() {
        let mut msg = m.clone();
        msg.reasoning = None; // Strip reasoning field to avoid compatibility issues

        if msg.role == "system" && !past_leading_systems {
            // Collect leading system message contents
            leading_system_contents.push(msg.content.clone());
        } else {
            // Past the leading system messages
            if !past_leading_systems {
                // Add merged system message if we collected any
                if !leading_system_contents.is_empty() {
                    messages.push(Message {
                        role: "system".to_string(),
                        content: leading_system_contents.join("\n\n"),
                        tool_calls: None,
                        tool_call_id: None,
                        name: None,
                        reasoning: None,
                    });
                }
                past_leading_systems = true;
            }
            messages.push(msg);
        }
    }

    let request = ChatRequest {
        model: current_model.as_str(
            chat.client_config.get_model_override(ModelColor::BluModel).as_deref().map(|x| x.as_str()),
            chat.client_config.get_model_override(ModelColor::GrnModel).as_deref().map(|x| x.as_str()),
            chat.client_config.get_model_override(ModelColor::RedModel).as_deref().map(|x| x.as_str())
        ).to_string(),
        messages,
        tools: chat.get_tools(),
        tool_choice: "auto".to_string(),
        stream: Some(true),
    };

    // Get the appropriate API URL based on the current model
    let api_url = crate::config::get_api_url(&chat.client_config, &current_model);

    // Capture request timestamp for response logging correlation
    let request_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Log request details in verbose mode
    log_request(&api_url, &request, &chat.api_key, chat.verbose);

    // Log request to file for persistent debugging
    let _ = log_request_to_file(&api_url, &request, &current_model, &chat.api_key);

    let api_key = crate::config::get_api_key(&chat.client_config, &chat.api_key, &current_model);
    let response = chat
        .client
        .post(&api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;

    let status = response.status();
    let headers = response.headers().clone();

    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_else(|_| "Unable to read error body".to_string());

        // Log error response
        log_response(&status, &headers, &error_body, chat.verbose);
        let _ = log_response_to_file(&status, &headers, &error_body, request_timestamp, &current_model);
        let _ = log_raw_response_to_file(&error_body, request_timestamp, &current_model);

        return Err(anyhow::anyhow!("API request failed with status {}: {}", status, error_body));
    }

    if chat.verbose {
        print_heart_red(&format!("\n{}", "📡 Starting streaming response...".bright_cyan()), true);
        print_heart_red(&format!("{}", "═".repeat(80).bright_cyan()), true);
    }

    // Initialize metrics tracking
    let mut metrics = StreamingMetrics::new();

    // Process streaming response
    let mut accumulated_content = String::new();
    let mut accumulated_reasoning = String::new();
    let mut accumulated_tool_calls: Vec<ToolCall> = Vec::new();
    let mut role = String::new();
    let mut usage: Option<Usage> = None;
    let mut buffer = String::new();
    let mut raw_response_body = String::new(); // Capture raw response body
    let mut final_finish_reason: Option<String> = None; // Track finish_reason from streaming

    // Show thinking indicator
    print_heart_red(&format!("🤔 Thinking..."), false);
    io::stdout().flush().unwrap();
    let mut first_chunk = true;
    let mut first_reasoning = true;

    // Read the response as a stream of bytes
    let mut stream = response.bytes_stream();
    let mut chunk_counter = 0;

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(bytes) => {
                let chunk_str = String::from_utf8_lossy(&bytes);
                raw_response_body.push_str(&chunk_str); // Capture raw response
                buffer.push_str(&chunk_str);

                // Process complete lines (SSE format: "data: {json}\n\n")
                while let Some(line_end) = buffer.find("\n\n") {
                    let line = buffer[..line_end].to_string();
                    buffer = buffer[line_end + 2..].to_string();

                    // Skip empty lines and non-data lines
                    if line.trim().is_empty() || !line.starts_with("data: ") {
                        continue;
                    }

                    let data = &line[6..]; // Skip "data: " prefix

                    // Log stream chunk in verbose mode
                    chunk_counter += 1;
                    log_stream_chunk(chunk_counter, data, chat.verbose);

                    // Check for stream end marker
                    if data.trim() == "[DONE]" {
                        if chat.verbose {
                            print_heart_red(&format!("{}", "✓ Stream completed".bright_green()), true);
                            print_heart_red(&format!("{}", "═".repeat(80).bright_green()), true);
                        }
                        break;
                    }

                    // Parse the JSON chunk
                    if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                        if let Some(usage_data) = chunk.usage {
                            usage = Some(usage_data);
                            // Update metrics with usage information
                            if let Some(ref usage_info) = usage {
                                metrics.completion_tokens = usage_info.completion_tokens;
                                metrics.prompt_tokens = Some(usage_info.prompt_tokens);
                                metrics.total_tokens = usage_info.total_tokens;
                            }
                        }

                        if let Some(choice) = chunk.choices.first() {
                            // Capture finish_reason if present
                            if choice.finish_reason.is_some() {
                                final_finish_reason = choice.finish_reason.clone();
                            }

                            let delta = &choice.delta;

                            // Update role if present
                            if let Some(r) = &delta.role {
                                role = r.clone();
                            }

                            // Accumulate and display reasoning content (thinking process)
                            if let Some(reasoning) = &delta.reasoning_content {
                                if first_chunk {
                                    // Clear thinking indicator
                                    print_heart_red(&format!("\r\x1B[K"), false);
                                    io::stdout().flush().unwrap();
                                    first_chunk = false;
                                }

                                if first_reasoning {
                                    // Show reasoning header
                                    print_heart_red(&format!("{}", "💭 ".bright_black()), false);
                                    first_reasoning = false;
                                }

                                accumulated_reasoning.push_str(reasoning);
                                // Display reasoning in dim color to distinguish from actual response
                                print_heart_red(&format!("{}", reasoning.bright_black()), false);
                                io::stdout().flush().unwrap();
                            }

                            // Accumulate content and display it
                            if let Some(content) = &delta.content {
                                if first_chunk {
                                    // Clear thinking indicator
                                    print_heart_red(&format!("\r\x1B[K"), false);
                                    io::stdout().flush().unwrap();
                                    first_chunk = false;
                                }

                                // If we just finished reasoning, add separator
                                if !first_reasoning && accumulated_content.is_empty() {
                                    print_heart_red("", true); // New line after reasoning
                                }

                                accumulated_content.push_str(content);
                                // Update token count in metrics (rough estimate: 1 token ≈ 4 characters)
                                metrics.completion_tokens += content.len() / 4;
                                metrics.total_tokens += content.len() / 4;
                                print_heart_red(&format!("{}", content), false);
                                io::stdout().flush().unwrap();
                            }

                            // Accumulate tool calls if present (streaming deltas)
                            if let Some(tool_call_deltas) = &delta.tool_calls {
                                if first_chunk {
                                    // Clear thinking indicator
                                    print_heart_red(&format!("\r\x1B[K"), false);
                                    print_heart_red(&format!("🔧 Tool calls..."), false);
                                    io::stdout().flush().unwrap();
                                    first_chunk = false;
                                }

                                for delta_call in tool_call_deltas {
                                    // Ensure we have enough slots in the accumulated array
                                    while accumulated_tool_calls.len() <= delta_call.index {
                                        accumulated_tool_calls.push(ToolCall {
                                            id: String::new(),
                                            tool_type: "function".to_string(),
                                            function: FunctionCall {
                                                name: String::new(),
                                                arguments: String::new(),
                                            },
                                        });
                                    }

                                    let tool_call = &mut accumulated_tool_calls[delta_call.index];

                                    // Merge the delta into the accumulated tool call
                                    if let Some(id) = &delta_call.id {
                                        tool_call.id = id.clone();
                                    }
                                    if let Some(tool_type) = &delta_call.tool_type {
                                        tool_call.tool_type = tool_type.clone();
                                    }
                                    if let Some(function_delta) = &delta_call.function {
                                        if let Some(name) = &function_delta.name {
                                            tool_call.function.name = name.clone();
                                        }
                                        if let Some(args) = &function_delta.arguments {
                                            // Some models (e.g., GLM) send a final complete JSON object
                                            // as the arguments chunk instead of incremental additions.
                                            // Detect this: if args starts with '{' and is valid JSON, replace
                                            // rather than append to avoid duplication.
                                            if args.trim().starts_with('{') {
                                                // Check if this looks like a complete JSON object
                                                // If it parses as valid JSON and we already have content,
                                                // this might be a duplicate - prefer the complete version
                                                if tool_call.function.arguments.is_empty()
                                                    || !tool_call.function.arguments.trim().starts_with('{') {
                                                    tool_call.function.arguments.push_str(args);
                                                } else {
                                                    // We already have accumulated content and this is a complete JSON
                                                    // Check if this appears to be a duplicate (same semantic content)
                                                    // For safety, use the complete version if it looks valid
                                                    let args_trimmed = args.trim();
                                                    if let Ok(_) = serde_json::from_str::<serde_json::Value>(args_trimmed) {
                                                        // This is valid JSON - replace instead of append
                                                        tool_call.function.arguments = args_trimmed.to_string();
                                                    } else {
                                                        // Not valid JSON, just append normally
                                                        tool_call.function.arguments.push_str(args);
                                                    }
                                                }
                                            } else {
                                                tool_call.function.arguments.push_str(args);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => return Err(anyhow::anyhow!("Error reading stream: {}", e)),
        }
    }

    // Clear thinking indicator if it was never cleared (no content received)
    if first_chunk {
        print_heart_red(&format!("\r\x1B[K"), false);
        io::stdout().flush().unwrap();
    }

    print_heart_red("", true); // New line after streaming complete

    // Finish metrics collection
    metrics.finish();

    // Create a response body representation for logging with raw content
    let response_body_for_logging = if accumulated_tool_calls.is_empty() {
        // Simple text response - log raw accumulated content
        if accumulated_reasoning.is_empty() {
            accumulated_content.clone()
        } else {
            // Include reasoning in the logged response
            format!("<reasoning>\n{}\n</reasoning>\n\n{}", accumulated_reasoning, accumulated_content)
        }
    } else {
        // Response with tool calls - create raw format similar to API response
        let mut result = String::new();
        
        // Add reasoning if present
        if !accumulated_reasoning.is_empty() {
            result.push_str(&format!("<reasoning>\n{}\n</reasoning>\n\n", accumulated_reasoning));
        }
        
        // Add content if present
        if !accumulated_content.is_empty() {
            result.push_str(&accumulated_content);
        }
        
        // Add tool calls representation
        result.push_str("\n\n<tool_calls>\n");
        for (i, tool_call) in accumulated_tool_calls.iter().enumerate() {
            result.push_str(&format!("Tool Call {}:\n", i + 1));
            result.push_str(&format!("  ID: {}\n", tool_call.id));
            result.push_str(&format!("  Function: {}\n", tool_call.function.name));
            result.push_str(&format!("  Arguments: {}\n", tool_call.function.arguments));
        }
        result.push_str("</tool_calls>");
        
        result
    };

    // Log successful streaming response to file
    let _ = log_response_to_file(&status, &headers, &response_body_for_logging, request_timestamp, &current_model);
    
    // Also log the raw response body without any transformation
    let _ = log_raw_response_to_file(&raw_response_body, request_timestamp, &current_model);

    // Build the final message
    let mut message = Message {
        role: if role.is_empty() { "assistant".to_string() } else { role },
        content: accumulated_content.clone(),
        tool_calls: if accumulated_tool_calls.is_empty() { None } else { Some(accumulated_tool_calls) },
        tool_call_id: None,
        name: None,
        reasoning: None,
    };

    // If no structured tool calls were received, check for XML format in content
    if message.tool_calls.is_none() {
        if let Some(parsed_calls) = parse_xml_tool_calls(&accumulated_content) {
            print_heart_yellow(&format!("{} Detected XML-format tool calls, parsing {} call(s)", "🔧".bright_yellow(), parsed_calls.len()), true);
            message.tool_calls = Some(parsed_calls);
            // Clear the XML from content to avoid displaying it
            message.content = String::new();
        }
    }

    Ok((message, usage, current_model, final_finish_reason, metrics))
}

/// Streaming API call using the new LlmClient system (for Anthropic and llama.cpp)
pub(crate) async fn call_api_streaming_with_llm_client(
    chat: &APChat,
    messages: &[Message],
    model: &ModelColor,
) -> Result<(Message, Option<Usage>, ModelColor, Option<String>, StreamingMetrics)> {
    if chat.should_show_debug(1) {
        print_heart_red(&format!("🔧 DEBUG: call_api_streaming_with_llm_client called with model: {:?}", model), true);
    }

    // Convert old Message format to new ChatMessage format
    let chat_messages: Vec<ChatMessage> = messages.iter().map(|msg| {
        ChatMessage {
            role: msg.role.clone(),
            content: msg.content.clone(),
            tool_calls: msg.tool_calls.clone().map(|calls| {
                calls.into_iter().map(|call| apchat_llm_api::ToolCall {
                    id: call.id,
                    function: apchat_llm_api::FunctionCall {
                        name: call.function.name,
                        arguments: call.function.arguments,
                    },
                }).collect()
            }),
            tool_call_id: msg.tool_call_id.clone(),
            name: msg.name.clone(),
            reasoning: None,
        }
    }).collect();

    // Convert tools to the new format
    let tools: Vec<ToolDefinition> = chat.get_tools().into_iter().map(|tool| {
        ToolDefinition {
            name: tool.function.name,
            description: tool.function.description,
            parameters: tool.function.parameters,
        }
    }).collect();

    // Create the appropriate LlmClient using the centralized helper
    let llm_client = crate::config::helpers::create_client_for_model_color_with_verbose(
        model,
        &chat.client_config,
        &chat.api_key,
        chat.verbose,
    );

    // Get the appropriate API URL based on the current model
    let _api_url = crate::config::get_api_url(&chat.client_config, model);

    // Capture request timestamp for response logging correlation
    let request_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    print_heart_red(&format!("\n{}", "📡 Starting Anthropic streaming response...".bright_cyan()), true);

    // Initialize metrics tracking
    let mut metrics = StreamingMetrics::new();

    // Initialize response accumulation
    let mut accumulated_content = String::new();
    let mut accumulated_tool_calls: Vec<ToolCall> = Vec::new();
    let mut tool_calls_in_progress: std::collections::HashMap<usize, (String, String, String)> = std::collections::HashMap::new(); // index -> (id, name, arguments)
    let mut role = String::new();
    let mut final_finish_reason: Option<String> = None; // Track finish_reason from streaming

    // Get the streaming response
    let mut stream = llm_client.chat_streaming(chat_messages.clone(), tools.clone()).await?;

    // Process the stream with minimal buffering
    use futures::StreamExt;
    use std::io::{self, Write};

    // Show thinking indicator
    print_heart_red(&format!("🤔 Thinking..."), false);
    io::stdout().flush().unwrap();
    let mut first_chunk = true;

    // Ensure stdout is flushed immediately for each chunk
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                // Handle the first chunk
                if first_chunk {
                    // Clear the "Thinking..." indicator
                    print_heart_red(&format!("\r"), false);
                    io::stdout().flush().unwrap();
                    first_chunk = false;
                }

                // Print the delta immediately without any buffering
                if !chunk.delta.is_empty() {
                    // Use direct write and flush for minimal latency
                    io::stdout().write_all(chunk.delta.as_bytes()).unwrap();
                    io::stdout().flush().unwrap();
                    accumulated_content.push_str(&chunk.delta);
                    // Update token count in metrics (rough estimate: 1 token ≈ 4 characters)
                    metrics.completion_tokens += chunk.delta.len() / 4;
                    metrics.total_tokens += chunk.delta.len() / 4;
                }

                // Handle tool call events
                if let Some(ref tool_event) = chunk.tool_call_event {
                    match tool_event {
                        ToolCallEvent::Start { index, id, name } => {
                            if chat.verbose {
                                print_heart_yellow(&format!("🔧 Tool call started: index={}, id={}, name={}", index, id, name), true);
                            }
                            tool_calls_in_progress.insert(*index, (id.clone(), name.clone(), String::new()));
                        }
                        ToolCallEvent::Delta { index, arguments_delta } => {
                            if chat.verbose {
                                print_heart_yellow(&format!("🔧 Tool call delta: index={}, delta={}", index, arguments_delta), true);
                            }
                            if let Some((id, name, args)) = tool_calls_in_progress.get_mut(index) {
                                args.push_str(arguments_delta);
                            }
                        }
                    }
                }

                // Check if we're done
                if let Some(ref reason) = chunk.finish_reason {
                    final_finish_reason = Some(reason.clone());
                    if reason == "stop" {
                        // Finalize tool calls
                        for (index, (id, name, arguments)) in &tool_calls_in_progress {
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
                        break;
                    }
                }
            }
            Err(e) => {
                print_heart_yellow(&format!("{} Streaming error: {}", "❌".red(), e), true);
                break;
            }
        }
    }

    print_heart_red("", true); // New line after streaming complete

    // Finish metrics collection
    metrics.finish();

    // Convert the response back to the old format
    let message = Message {
        role: if role.is_empty() { "assistant".to_string() } else { role },
        content: accumulated_content.clone(),
        tool_calls: if accumulated_tool_calls.is_empty() { None } else { Some(accumulated_tool_calls) },
        tool_call_id: None,
        name: None,
        reasoning: None,
    };

    // For now, we don't have usage information from streaming
    // This could be enhanced by extracting usage from the final message_stop event
    let usage: Option<Usage> = None;

    // Log the final response for LlmClient streaming
    let response_body = format!("Role: {}\n\nContent:\n{}\n\nTool calls: {}\n\nUsage: {:?}",
        message.role,
        message.content,
        message.tool_calls.as_ref().map_or("None".to_string(), |calls| {
            format!("{} calls", calls.len())
        }),
        usage
    );
    
    // Create mock status and headers for logging
    let status = reqwest::StatusCode::OK;
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert("x-streaming", "anthropic".parse().unwrap());
    
    let _ = log_response_to_file(&status, &headers, &response_body, request_timestamp, model);

    Ok((message, usage, model.clone(), final_finish_reason, metrics))
}
