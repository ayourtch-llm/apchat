use crate::client::{LlmClient, LlmResponse, ChatMessage, ToolDefinition, TokenUsage, StreamingChunk};
use anyhow::{Result, Context};
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use async_stream::stream;
use serde_json::Value;
use apchat_vty::print_heart_yellow;

/// llama.cpp server LLM client implementation with OpenAI-compatible API
#[derive(Debug)]
pub struct LlamaCppClient {
    base_url: String,
    model: String,
    client: reqwest::Client,
    verbose: bool,
}

impl LlamaCppClient {
    pub fn new(base_url: String, model: String) -> Self {
        Self::new_with_verbose(base_url, model, false)
    }

    pub fn new_with_verbose(base_url: String, model: String, verbose: bool) -> Self {
        // Ensure base_url doesn't end with a slash
        let base_url = base_url.trim_end_matches('/').to_string();

        // Configure client for minimal buffering and immediate streaming
        let client = reqwest::Client::builder()
            .http1_only()  // Force HTTP/1.1 to avoid HTTP/2 buffering
            .tcp_nodelay(true)  // Disable Nagle's algorithm for immediate transmission
            .pool_max_idle_per_host(0)  // Disable connection pooling to avoid stale connections
            .timeout(std::time::Duration::from_secs(300))  // 5 minute timeout for long streams
            .build()
            .expect("Failed to build HTTP client");

        Self {
            base_url,
            model,
            client,
            verbose,
        }
    }

    fn get_chat_completions_url(&self) -> String {
        format!("{}/v1/chat/completions", self.base_url)
    }
}

#[async_trait]
impl LlmClient for LlamaCppClient {
    async fn chat(&self, messages: Vec<ChatMessage>, tools: Vec<ToolDefinition>) -> Result<LlmResponse> {
        let request = self.build_chat_request(messages, tools).await?;

        let response = self.client
            .post(self.get_chat_completions_url())
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("llama.cpp API error: {}", error_text));
        }

        let response_text = response.text().await?;
        let chat_response: apchat_models::ChatResponse = serde_json::from_str(&response_text)?;

        let (message, finish_reason) = if let Some(choice) = chat_response.choices.into_iter().next() {
            let msg = ChatMessage {
                role: choice.message.role,
                content: choice.message.content,
                tool_calls: choice.message.tool_calls.map(|calls| {
                    calls.into_iter().map(|call| crate::client::ToolCall {
                        id: call.id,
                        function: crate::client::FunctionCall {
                            name: call.function.name,
                            arguments: call.function.arguments,
                        },
                    }).collect()
                }),
                tool_call_id: choice.message.tool_call_id,
                name: choice.message.name,
                reasoning: None,
            };
            (msg, choice.finish_reason)
        } else {
            (ChatMessage {
                role: "assistant".to_string(),
                content: "No response generated".to_string(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                reasoning: None,
            }, None)
        };

        Ok(LlmResponse {
            message,
            usage: chat_response.usage.map(|usage| TokenUsage {
                prompt_tokens: usage.prompt_tokens as u32,
                completion_tokens: usage.completion_tokens as u32,
                total_tokens: usage.total_tokens as u32,
            }),
            finish_reason,
        })
    }

    async fn chat_completion(&self, messages: &[ChatMessage]) -> Result<String> {
        // Convert system messages to user messages for Mistral compatibility
        let converted_messages: Vec<serde_json::Value> = messages.iter().map(|msg| {
            let role = if msg.role == "system" {
                "user"
            } else {
                &msg.role
            };
            serde_json::json!({
                "role": role,
                "content": &msg.content
            })
        }).collect();

        // For progress evaluation, make a simple API call without tools
        let api_request = serde_json::json!({
            "model": self.model,
            "messages": converted_messages,
            "temperature": 0.1,
            "max_tokens": 2000
        });

        let response = self.client
            .post(self.get_chat_completions_url())
            .header("Content-Type", "application/json")
            .json(&api_request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("API request failed: {} - {}", status, error_text));
        }

        let response_text = response.text().await?;
        let chat_response: serde_json::Value = serde_json::from_str(&response_text)?;

        if let Some(content) = chat_response["choices"][0]["message"]["content"].as_str() {
            Ok(content.to_string())
        } else {
            Err(anyhow::anyhow!("No content in response"))
        }
    }

    async fn chat_streaming(&self, messages: Vec<ChatMessage>, tools: Vec<ToolDefinition>) -> Result<Box<dyn Stream<Item = Result<StreamingChunk>> + Send + Unpin>> {
        let request = self.build_chat_request(messages, tools).await?;

        // Enable streaming in the request
        let mut streaming_request = request.as_object().unwrap().clone();
        streaming_request.insert("stream".to_string(), serde_json::json!(true));

        let response = self.client
            .post(self.get_chat_completions_url())
            .header("Content-Type", "application/json")
            .header("Connection", "keep-alive")
            .header("Cache-Control", "no-cache")
            .header("Accept", "text/event-stream")
            .json(&streaming_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("llama.cpp streaming API error: {}", error_text));
        }

        let byte_stream = response.bytes_stream();
        let verbose = self.verbose;

        let stream = stream! {
            let mut buffer = String::new();
            let mut byte_stream = byte_stream;
            let mut chunk_counter = 0;
            let start_time = std::time::Instant::now();
            let mut last_chunk_time = start_time;

            if verbose {
                print_heart_yellow(&format!("🔍 [llama.cpp Streaming] Starting stream at {:?}", start_time), true);
            }

            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        chunk_counter += 1;
                        let now = std::time::Instant::now();
                        let elapsed_total = now.duration_since(start_time);
                        let elapsed_since_last = now.duration_since(last_chunk_time);
                        last_chunk_time = now;

                        // Print raw bytes received (before any conversion)
                        if verbose {
                            print_heart_yellow(&format!("🔍 [llama.cpp Streaming] Chunk #{}: {} bytes [total: {:?}, since last: {:?}]",
                                chunk_counter, chunk.len(), elapsed_total, elapsed_since_last), true);
                            print_heart_yellow(&format!("🔍 [llama.cpp Streaming] Raw bytes (hex): {}",
                                chunk.iter().take(200).map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ")), true);
                            print_heart_yellow(&format!("🔍 [llama.cpp Streaming] Raw bytes (debug): {:?}",
                                &chunk[..chunk.len().min(200)]), true);
                        }

                        // Convert bytes to string, handling UTF-8 errors gracefully
                        let chunk_str = match String::from_utf8(chunk.to_vec()) {
                            Ok(s) => s,
                            Err(e) => {
                                if verbose {
                                    print_heart_yellow(&format!("❌ [llama.cpp Streaming] Invalid UTF-8 in stream: {}", e), true);
                                    print_heart_yellow(&format!("❌ [llama.cpp Streaming] Failed bytes (hex): {}",
                                        chunk.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ")), true);
                                }
                                yield Err(anyhow::anyhow!("Invalid UTF-8 in stream: {}", e));
                                break;
                            }
                        };

                        // Print raw chunk content after UTF-8 decoding
                        if verbose {
                            print_heart_yellow(&format!("🔍 [llama.cpp Streaming] Decoded UTF-8 string ({} chars):\n{}", chunk_str.len(), chunk_str), true);
                        }

                        buffer.push_str(&chunk_str);

                        // Process complete SSE lines (lines ending with \n)
                        let mut line_num = 0;
                        while let Some(newline_pos) = buffer.find('\n') {
                            line_num += 1;
                            let line = buffer[..newline_pos].to_string();
                            buffer = buffer[newline_pos + 1..].to_string();

                            // Print each line being processed
                            if verbose {
                                print_heart_yellow(&format!("🔍 [llama.cpp Streaming] Processing line #{}: {:?}", line_num, line), true);
                            }

                            // Parse SSE line and yield result (including errors)
                            match Self::parse_openai_sse_line(&line) {
                                Ok(Some(streaming_chunk)) => {
                                    if verbose {
                                        print_heart_yellow(&format!("✅ [llama.cpp Streaming] Yielding chunk: delta_len={}, finish_reason={:?}",
                                            streaming_chunk.delta.len(), streaming_chunk.finish_reason), true);
                                    }
                                    yield Ok(streaming_chunk);
                                }
                                Ok(None) => {
                                    if verbose {
                                        print_heart_yellow(&format!("⏭️  [llama.cpp Streaming] No content in line"), true);
                                    }
                                }
                                Err(e) => {
                                    if verbose {
                                        print_heart_yellow(&format!("❌ [llama.cpp Streaming] Parse error: {}", e), true);
                                    }
                                    yield Err(e);
                                    // Continue processing other lines instead of breaking
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if verbose {
                            print_heart_yellow(&format!("❌ [llama.cpp Streaming] Stream error: {}", e), true);
                        }
                        yield Err(anyhow::anyhow!("Stream error: {}", e));
                        break;
                    }
                }
            }

            // Process any remaining complete line in the buffer
            if !buffer.is_empty() && !buffer.trim().is_empty() {
                if verbose {
                    print_heart_yellow(&format!("🔍 [llama.cpp Streaming] Processing remaining buffer: {:?}", buffer), true);
                }
                match Self::parse_openai_sse_line(&buffer) {
                    Ok(Some(streaming_chunk)) => {
                        if verbose {
                            print_heart_yellow(&format!("✅ [llama.cpp Streaming] Yielding final chunk: delta_len={}", streaming_chunk.delta.len()), true);
                        }
                        yield Ok(streaming_chunk);
                    }
                    Ok(None) => {
                        if verbose {
                            print_heart_yellow(&format!("⏭️  [llama.cpp Streaming] No content in remaining buffer"), true);
                        }
                    }
                    Err(e) => {
                        if verbose {
                            print_heart_yellow(&format!("❌ [llama.cpp Streaming] Final parse error: {}", e), true);
                        }
                        yield Err(e);
                    }
                }
            }

            if verbose {
                let total_duration = std::time::Instant::now().duration_since(start_time);
                print_heart_yellow(&format!("🏁 [llama.cpp Streaming] Stream ended. Total chunks: {}, Total duration: {:?}", chunk_counter, total_duration), true);
            }
        };

        Ok(Box::new(Box::pin(stream)))
    }
}

impl LlamaCppClient {
    /// Parse OpenAI-compatible SSE line and return a streaming chunk if it contains text
    /// Returns:
    /// - Ok(Some(chunk)) if the line contains streamable content
    /// - Ok(None) if the line is valid but doesn't contain content
    /// - Err if the line contains an error or malformed data
    fn parse_openai_sse_line(line: &str) -> Result<Option<StreamingChunk>> {
        // Skip empty lines
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        // Only process data lines (SSE format)
        if !trimmed.starts_with("data: ") {
            // Non-data lines are valid SSE but not data
            return Ok(None);
        }

        let data = trimmed[6..].trim();

        // Check for stream end marker
        if data == "[DONE]" {
            return Ok(Some(StreamingChunk {
                content: String::new(),
                delta: String::new(),
                finish_reason: Some("stop".to_string()),
                tool_call_event: None,
            }));
        }

        // Parse JSON event (OpenAI format)
        let json = serde_json::from_str::<Value>(data)
            .with_context(|| format!("Failed to parse OpenAI SSE JSON data: {}", data))?;

        // Check for errors
        if let Some(error) = json.get("error") {
            let error_msg = error.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error from API");
            let error_type = error.get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown_error");
            return Err(anyhow::anyhow!("API error ({}): {}", error_type, error_msg));
        }

        // Extract content from OpenAI streaming format
        if let Some(choices) = json.get("choices").and_then(|v| v.as_array()) {
            if let Some(first_choice) = choices.first() {
                // Check for finish reason
                let finish_reason = first_choice.get("finish_reason")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Extract delta content
                if let Some(delta) = first_choice.get("delta") {
                    if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                        return Ok(Some(StreamingChunk {
                            content: String::new(),
                            delta: content.to_string(),
                            finish_reason,
                            tool_call_event: None,
                        }));
                    }

                    // Handle role changes (first chunk often has role but no content)
                    if delta.get("role").is_some() {
                        return Ok(Some(StreamingChunk {
                            content: String::new(),
                            delta: String::new(),
                            finish_reason: None,
                            tool_call_event: None,
                        }));
                    }
                }

                // If we have a finish_reason but no content, signal completion
                if finish_reason.is_some() {
                    return Ok(Some(StreamingChunk {
                        content: String::new(),
                        delta: String::new(),
                        finish_reason,
                        tool_call_event: None,
                    }));
                }
            }
        }

        // Valid SSE event but no content to stream
        Ok(None)
    }

    async fn build_chat_request(&self, messages: Vec<ChatMessage>, tools: Vec<ToolDefinition>) -> Result<serde_json::Value> {
        let chat_messages: Vec<apchat_models::Message> = messages.into_iter().map(|msg| {
            // Convert system role to user role for models that don't support it
            // (e.g., Llama.cpp with Mistral models)
            let role = if msg.role == "system" {
                "user".to_string()
            } else {
                msg.role
            };

            apchat_models::Message {
                role,
                content: msg.content,
                tool_calls: msg.tool_calls.map(|calls| {
                    calls.into_iter().map(|call| apchat_models::ToolCall {
                        id: call.id,
                        tool_type: "function".to_string(),
                        function: apchat_models::FunctionCall {
                            name: call.function.name,
                            arguments: call.function.arguments,
                        },
                    }).collect()
                }),
                tool_call_id: msg.tool_call_id,
                name: msg.name,
                reasoning: None,
            }
        }).collect();

        let tool_definitions: Vec<apchat_models::Tool> = tools.into_iter().map(|tool| {
            apchat_models::Tool {
                tool_type: "function".to_string(),
                function: apchat_models::FunctionDef {
                    name: tool.name,
                    description: tool.description,
                    parameters: tool.parameters,
                },
            }
        }).collect();

        Ok(serde_json::json!({
            "model": self.model,
            "messages": chat_messages,
            "tools": tool_definitions,
            "tool_choice": "auto"
        }))
    }
}
