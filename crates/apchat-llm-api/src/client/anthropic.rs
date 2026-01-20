use crate::client::{LlmClient, LlmResponse, ChatMessage, ToolDefinition, StreamingChunk, ToolCall, FunctionCall, TokenUsage, ToolCallEvent};
use anyhow::{Result, Context};
use async_trait::async_trait;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use futures::Stream;
use futures::StreamExt;
use async_stream::stream;
use apchat_logging::get_logs_dir;
use apchat_logging::vty_output::print_heart_yellow;

/// Anthropic LLM client implementation using native Anthropic API
pub struct AnthropicLlmClient {
    api_key: String,
    model: String,
    base_url: String,
    agent_name: String,
    client: reqwest::Client,
    verbose: bool,
}

impl AnthropicLlmClient {
    pub fn new(api_key: String, model: String, base_url: String, agent_name: String) -> Self {
        Self::new_with_verbose(api_key, model, base_url, agent_name, false)
    }

    pub fn new_with_verbose(api_key: String, model: String, base_url: String, agent_name: String, verbose: bool) -> Self {
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
            api_key,
            model,
            base_url,
            agent_name,
            client,
            verbose,
        }
    }

    fn get_messages_url(&self) -> String {
        format!("{}/v1/messages", self.base_url)
    }

    fn convert_messages_to_anthropic_format(&self, messages: Vec<ChatMessage>) -> Vec<Value> {
        messages.into_iter().filter_map(|msg| {
            // Skip system messages as they should be handled separately
            if msg.role == "system" {
                return None;
            }

            // Convert role to Anthropic format (only user/assistant allowed)
            let anthropic_role = if msg.role == "user" || msg.role == "assistant" {
                msg.role.clone()
            } else {
                // Convert any other roles to "user"
                "user".to_string()
            };

            let content = if msg.role == "tool" {
                // Tool result messages need special handling
                vec![
                    serde_json::json!({
                        "type": "tool_result",
                        "tool_use_id": msg.tool_call_id.unwrap_or_default(),
                        "content": msg.content
                    })
                ]
            } else if let Some(tool_calls) = msg.tool_calls {
                // Assistant message with tool calls
                let mut content = vec![];
                if !msg.content.is_empty() {
                    content.push(serde_json::json!({
                        "type": "text",
                        "text": msg.content
                    }));
                }
                for tool_call in tool_calls {
                    content.push(serde_json::json!({
                        "type": "tool_use",
                        "id": tool_call.id,
                        "name": tool_call.function.name,
                        "input": serde_json::from_str::<Value>(&tool_call.function.arguments)
                            .unwrap_or_else(|_| serde_json::json!({}))
                    }));
                }
                content
            } else {
                // Regular text message
                vec![serde_json::json!({
                    "type": "text",
                    "text": msg.content
                })]
            };

            Some(serde_json::json!({
                "role": anthropic_role,
                "content": content
            }))
        }).collect()
    }

    fn convert_tools_to_anthropic_format(&self, tools: Vec<ToolDefinition>) -> Vec<Value> {
        tools.into_iter().map(|tool| {
            serde_json::json!({
                "name": tool.name,
                "description": tool.description,
                "input_schema": tool.parameters
            })
        }).collect()
    }

    fn convert_anthropic_response_to_chat_message(&self, response: &Value) -> ChatMessage {
        let empty_vec = vec![];
        let content = response["content"].as_array().unwrap_or(&empty_vec);

        let mut text_content = String::new();
        let mut tool_calls = Vec::new();

        for item in content {
            if let Some(content_type) = item["type"].as_str() {
                match content_type {
                    "text" => {
                        if let Some(text) = item["text"].as_str() {
                            text_content.push_str(text);
                        }
                    }
                    "tool_use" => {
                        if let Some(name) = item["name"].as_str() {
                            if let Some(id) = item["id"].as_str() {
                                let input = item["input"].clone();
                                tool_calls.push(ToolCall {
                                    id: id.to_string(),
                                    function: FunctionCall {
                                        name: name.to_string(),
                                        arguments: input.to_string(),
                                    },
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        ChatMessage {
            role: response["role"].as_str().unwrap_or("assistant").to_string(),
            content: text_content,
            tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
            tool_call_id: None,
            name: None,
            reasoning: None,
        }
    }
}

#[async_trait]
impl LlmClient for AnthropicLlmClient {
    async fn chat(&self, messages: Vec<ChatMessage>, tools: Vec<ToolDefinition>) -> Result<LlmResponse> {
        // Extract system messages and combine them
        let system_messages: Vec<String> = messages.iter()
            .filter(|msg| msg.role == "system")
            .map(|msg| msg.content.clone())
            .collect();

        let combined_system = if system_messages.is_empty() {
            None
        } else {
            Some(system_messages.join("\n\n"))
        };

        let anthropic_messages = self.convert_messages_to_anthropic_format(messages);
        let anthropic_tools = self.convert_tools_to_anthropic_format(tools);

        let mut request = serde_json::json!({
            "model": self.model,
            "messages": anthropic_messages,
            "max_tokens": 4096,
            "tools": anthropic_tools,
            "tool_choice": {"type": "auto"}
        });

        // Add system message if present
        if let Some(system_content) = combined_system {
            request["system"] = serde_json::Value::String(system_content);
        }

        // Log request to file for persistent debugging
        let _ = self.log_request_to_file(&self.get_messages_url(), &request);

        let response = self.client
            .post(&self.get_messages_url())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .header("Connection", "keep-alive")  // Keep connection alive for streaming
            .header("Cache-Control", "no-cache")  // Prevent caching of streaming data
            .header("Accept", "text/event-stream")  // Explicitly accept SSE
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Anthropic API error: {}", error_text));
        }

        let response_text = response.text().await?;
        let response_json: Value = serde_json::from_str(&response_text)?;

        let message = self.convert_anthropic_response_to_chat_message(&response_json);

        let usage = response_json.get("usage").map(|u| {
            TokenUsage {
                prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: (u["input_tokens"].as_u64().unwrap_or(0) +
                               u["output_tokens"].as_u64().unwrap_or(0)) as u32,
            }
        });

        Ok(LlmResponse {
            message,
            usage,
        })
    }

    async fn chat_streaming(&self, messages: Vec<ChatMessage>, tools: Vec<ToolDefinition>) -> Result<Box<dyn Stream<Item = Result<StreamingChunk>> + Send + Unpin>> {
        // Extract system messages and combine them
        let system_messages: Vec<String> = messages.iter()
            .filter(|msg| msg.role == "system")
            .map(|msg| msg.content.clone())
            .collect();

        let combined_system = if system_messages.is_empty() {
            None
        } else {
            Some(system_messages.join("\n\n"))
        };

        let anthropic_messages = self.convert_messages_to_anthropic_format(messages);
        let anthropic_tools = self.convert_tools_to_anthropic_format(tools);

        let mut request = serde_json::json!({
            "model": self.model,
            "messages": anthropic_messages,
            "max_tokens": 4096,
            "tools": anthropic_tools,
            "tool_choice": {"type": "auto"},
            "stream": true
        });

        // Add system message if present
        if let Some(system_content) = combined_system {
            request["system"] = serde_json::Value::String(system_content);
        }

        // Log request to file for persistent debugging
        let _ = self.log_request_to_file(&self.get_messages_url(), &request);

        let response = self.client
            .post(&self.get_messages_url())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .header("Connection", "keep-alive")  // Keep connection alive for streaming
            .header("Cache-Control", "no-cache")  // Prevent caching of streaming data
            .header("Accept", "text/event-stream")  // Explicitly accept SSE
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Anthropic API streaming error: {}", error_text));
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
                print_heart_yellow(format!("🔍 [Anthropic Streaming] Starting stream at {:?}", start_time).as_str(), true);
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
                            print_heart_yellow(format!("🔍 [Anthropic Streaming] Chunk #{}: {} bytes [total: {:?}, since last: {:?}]",
                                chunk_counter, chunk.len(), elapsed_total, elapsed_since_last).as_str(), true);
                            print_heart_yellow(format!("🔍 [Anthropic Streaming] Raw bytes (hex): {}",
                                chunk.iter().take(200).map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ")).as_str(), true);
                            print_heart_yellow(format!("🔍 [Anthropic Streaming] Raw bytes (debug): {:?}",
                                &chunk[..chunk.len().min(200)]).as_str(), true);
                        }

                        // Convert bytes to string, handling UTF-8 errors gracefully
                        let chunk_str = match String::from_utf8(chunk.to_vec()) {
                            Ok(s) => s,
                            Err(e) => {
                                if verbose {
                                    print_heart_yellow(format!("❌ [Anthropic Streaming] Invalid UTF-8 in stream: {}", e).as_str(), true);
                                    print_heart_yellow(format!("❌ [Anthropic Streaming] Failed bytes (hex): {}",
                                        chunk.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ")).as_str(), true);
                                }
                                yield Err(anyhow::anyhow!("Invalid UTF-8 in stream: {}", e));
                                break;
                            }
                        };

                        // Print raw chunk content after UTF-8 decoding
                        if verbose {
                            print_heart_yellow(format!("🔍 [Anthropic Streaming] Decoded UTF-8 string ({} chars):\n[START]{}[END]", chunk_str.len(), chunk_str).as_str(), true);
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
                                print_heart_yellow(format!("🔍 [Anthropic Streaming] Processing line #{}: {:?}", line_num, line).as_str(), true);
                            }

                            // Parse SSE line and yield result (including errors)
                            match Self::parse_sse_line(&line) {
                                Ok(Some(streaming_chunk)) => {
                                    if verbose {
                                        print_heart_yellow(format!("✅ [Anthropic Streaming] Yielding chunk: delta_len={}, finish_reason={:?}",
                                            streaming_chunk.delta.len(), streaming_chunk.finish_reason).as_str(), true);
                                    }
                                    yield Ok(streaming_chunk);
                                }
                                Ok(None) => {
                                    if verbose {
                                        print_heart_yellow(format!("⏭️  [Anthropic Streaming] No content in line (ping/metadata)").as_str(), true);
                                    }
                                }
                                Err(e) => {
                                    if verbose {
                                        print_heart_yellow(format!("❌ [Anthropic Streaming] Parse error: {}", e).as_str(), true);
                                    }
                                    yield Err(e);
                                    // Continue processing other lines instead of breaking
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if verbose {
                            print_heart_yellow(format!("❌ [Anthropic Streaming] Stream error: {}", e).as_str(), true);
                        }
                        yield Err(anyhow::anyhow!("Stream error: {}", e));
                        break;
                    }
                }
            }

            // Process any remaining complete line in the buffer
            if !buffer.is_empty() && !buffer.trim().is_empty() {
                if verbose {
                    print_heart_yellow(format!("🔍 [Anthropic Streaming] Processing remaining buffer: {:?}", buffer).as_str(), true);
                }
                match Self::parse_sse_line(&buffer) {
                    Ok(Some(streaming_chunk)) => {
                        if verbose {
                            print_heart_yellow(format!("✅ [Anthropic Streaming] Yielding final chunk: delta_len={}", streaming_chunk.delta.len()).as_str(), true);
                        }
                        yield Ok(streaming_chunk);
                    }
                    Ok(None) => {
                        if verbose {
                            print_heart_yellow(format!("⏭️  [Anthropic Streaming] No content in remaining buffer").as_str(), true);
                        }
                    }
                    Err(e) => {
                        if verbose {
                            print_heart_yellow(format!("❌ [Anthropic Streaming] Final parse error: {}", e).as_str(), true);
                        }
                        yield Err(e);
                    }
                }
            }

            if verbose {
                let total_duration = std::time::Instant::now().duration_since(start_time);
                print_heart_yellow(format!("🏁 [Anthropic Streaming] Stream ended. Total chunks: {}, Total duration: {:?}", chunk_counter, total_duration).as_str(), true);
            }
        };

        Ok(Box::new(Box::pin(stream)))
    }

    async fn chat_completion(&self, messages: &[ChatMessage]) -> Result<String> {
        let anthropic_messages = self.convert_messages_to_anthropic_format(messages.to_vec());

        let request = serde_json::json!({
            "model": self.model,
            "messages": anthropic_messages,
            "max_tokens": 2000,
            "temperature": 0.1
        });

        // Log request to file for persistent debugging
        let _ = self.log_request_to_file(&self.get_messages_url(), &request);

        let response = self.client
            .post(&self.get_messages_url())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .header("Connection", "keep-alive")  // Keep connection alive for streaming
            .header("Cache-Control", "no-cache")  // Prevent caching of streaming data
            .header("Accept", "text/event-stream")  // Explicitly accept SSE
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("API request failed: {} - {}", status, error_text));
        }

        let response_text = response.text().await?;
        let response_json: serde_json::Value = serde_json::from_str(&response_text)?;

        if let Some(content) = response_json["content"].as_array() {
            let mut text = String::new();
            for item in content {
                if item["type"] == "text" {
                    if let Some(text_content) = item["text"].as_str() {
                        text.push_str(text_content);
                    }
                }
            }
            Ok(text)
        } else {
            Err(anyhow::anyhow!("No content in response"))
        }
    }
}

impl AnthropicLlmClient {
    /// Parse a single SSE line and return a streaming chunk if it contains text
    /// Returns:
    /// - Ok(Some(chunk)) if the line contains streamable content
    /// - Ok(None) if the line is valid but doesn't contain content (e.g., ping)
    /// - Err if the line contains an error event or malformed data
    fn parse_sse_line(line: &str) -> Result<Option<StreamingChunk>> {
        // Skip empty lines
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        // Only process data lines (SSE format)
        if !trimmed.starts_with("data: ") {
            // Non-data lines (like "event:", ":comment", etc.) are valid SSE but not data
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

        // Parse JSON event
        let json = serde_json::from_str::<Value>(data)
            .with_context(|| format!("Failed to parse SSE JSON data: {}", data))?;

        // Check for error events first
        if let Some(content_type) = json["type"].as_str() {
            if content_type == "error" {
                let error_msg = json.get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error from API");
                let error_type = json.get("error")
                    .and_then(|e| e.get("type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown_error");
                return Err(anyhow::anyhow!("API error ({}): {}", error_type, error_msg));
            }

            match content_type {
                "content_block_delta" => {
                    // Get the index for this content block
                    let index = json.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

                    if let Some(delta) = json.get("delta").and_then(|v| v.as_object()) {
                        // Check if this is text content
                        if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                            return Ok(Some(StreamingChunk {
                                content: String::new(),
                                delta: text.to_string(),
                                finish_reason: None,
                                tool_call_event: None,
                            }));
                        }

                        // Check if this is tool input JSON delta
                        if delta.get("type").and_then(|v| v.as_str()) == Some("input_json_delta") {
                            if let Some(partial_json) = delta.get("partial_json").and_then(|v| v.as_str()) {
                                return Ok(Some(StreamingChunk {
                                    content: String::new(),
                                    delta: String::new(),
                                    finish_reason: None,
                                    tool_call_event: Some(ToolCallEvent::Delta {
                                        index,
                                        arguments_delta: partial_json.to_string(),
                                    }),
                                }));
                            }
                        }
                    }
                }
                "content_block_start" => {
                    // Get the index for this content block
                    let index = json.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

                    if let Some(content_block) = json.get("content_block").and_then(|v| v.as_object()) {
                        let block_type = content_block.get("type").and_then(|v| v.as_str());

                        // Handle text content block
                        if block_type == Some("text") {
                            if let Some(text) = content_block.get("text").and_then(|v| v.as_str()) {
                                return Ok(Some(StreamingChunk {
                                    content: text.to_string(),
                                    delta: text.to_string(),
                                    finish_reason: None,
                                    tool_call_event: None,
                                }));
                            }
                        }

                        // Handle tool_use content block
                        if block_type == Some("tool_use") {
                            if let (Some(id), Some(name)) = (
                                content_block.get("id").and_then(|v| v.as_str()),
                                content_block.get("name").and_then(|v| v.as_str())
                            ) {
                                return Ok(Some(StreamingChunk {
                                    content: String::new(),
                                    delta: String::new(),
                                    finish_reason: None,
                                    tool_call_event: Some(ToolCallEvent::Start {
                                        index,
                                        id: id.to_string(),
                                        name: name.to_string(),
                                    }),
                                }));
                            }
                        }
                    }
                }
                "message_stop" => {
                    return Ok(Some(StreamingChunk {
                        content: String::new(),
                        delta: String::new(),
                        finish_reason: Some("stop".to_string()),
                        tool_call_event: None,
                    }));
                }
                // Other event types (ping, message_start, content_block_stop, etc.) are valid but don't contain content
                _ => {}
            }
        }

        // Valid SSE event but no content to stream
        Ok(None)
    }

    fn log_request_to_file(&self, url: &str, request: &serde_json::Value) -> Result<()> {
        // Use centralized logs directory
        let logs_dir: PathBuf = get_logs_dir()?;

        // Generate timestamp for filename
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create filename with timestamp and model name
        let model_name = self.model.replace('/', "-");
        let filename = logs_dir.join(format!("req-{}-{}-agent-{}.txt", timestamp, model_name, self.agent_name));

        // Build the log content
        let mut log_content = String::new();
        log_content.push_str(&format!("HTTP REQUEST LOG (ANTHROPIC)\n"));
        log_content.push_str(&format!("===============================\n\n"));
        log_content.push_str(&format!("Timestamp: {}\n", timestamp));
        log_content.push_str(&format!("Model: {}\n", self.model));
        log_content.push_str(&format!("Agent: {}\n\n", self.agent_name));

        // Parse URL to show host and port
        if let Ok(parsed_url) = reqwest::Url::parse(url) {
            log_content.push_str(&format!("URL: {}\n", url));
            log_content.push_str(&format!("Host: {}\n", parsed_url.host_str().unwrap_or("unknown")));
            log_content.push_str(&format!("Port: {}\n",
                parsed_url.port().map(|p| p.to_string()).unwrap_or_else(||
                    if parsed_url.scheme() == "https" { "443 (default)".to_string() } else { "80 (default)".to_string() }
                )
            ));
            log_content.push_str(&format!("Scheme: {}\n\n", parsed_url.scheme()));
        } else {
            log_content.push_str(&format!("URL: {}\n\n", url));
        }

        log_content.push_str("Headers:\n");
        log_content.push_str("  Content-Type: application/json\n");
        log_content.push_str(&format!("  x-api-key: {}***\n", &self.api_key.chars().take(10).collect::<String>()));
        log_content.push_str("  anthropic-version: 2023-06-01\n\n");

        log_content.push_str("Request Body:\n");
        match serde_json::to_string_pretty(&request) {
            Ok(json) => {
                log_content.push_str(&json);
                log_content.push_str("\n");
            }
            Err(e) => {
                log_content.push_str(&format!("Error serializing request: {}\n", e));
            }
        }

        // Write to file
        fs::write(&filename, log_content)
            .with_context(|| format!("Failed to write request log to {}", filename.display()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_line_text_delta() {
        let line = "data: {\"type\": \"content_block_delta\", \"index\": 0, \"delta\": {\"type\": \"text_delta\", \"text\": \"Hello\"}}";
        let result = AnthropicLlmClient::parse_sse_line(line);
        assert!(result.is_ok());

        let chunk = result.unwrap().unwrap();
        assert_eq!(chunk.delta, "Hello");
        assert!(chunk.tool_call_event.is_none());
    }

    #[test]
    fn test_parse_sse_line_tool_use_start() {
        let line = "data: {\"type\": \"content_block_start\", \"index\": 0, \"content_block\": {\"type\": \"tool_use\", \"id\": \"toolu_01234\", \"name\": \"test_function\"}}";
        let result = AnthropicLlmClient::parse_sse_line(line);
        assert!(result.is_ok());

        let chunk = result.unwrap().unwrap();
        assert!(chunk.delta.is_empty());
        assert!(matches!(chunk.tool_call_event, Some(ToolCallEvent::Start { index: 0, id, name }) if id == "toolu_01234" && name == "test_function"));
    }

    #[test]
    fn test_parse_sse_line_tool_use_delta() {
        let line = "data: {\"type\": \"content_block_delta\", \"index\": 0, \"delta\": {\"type\": \"input_json_delta\", \"partial_json\": \"{\\\"arg1\\\": \\\"value1\\\"\"}}";
        let result = AnthropicLlmClient::parse_sse_line(line);
        assert!(result.is_ok());

        let chunk = result.unwrap().unwrap();
        assert!(chunk.delta.is_empty());
        assert!(matches!(chunk.tool_call_event, Some(ToolCallEvent::Delta { index: 0, arguments_delta }) if arguments_delta.contains("arg1")));
    }

    #[test]
    fn test_parse_sse_line_message_stop() {
        let line = "data: {\"type\": \"message_stop\"}";
        let result = AnthropicLlmClient::parse_sse_line(line);
        assert!(result.is_ok());

        let chunk = result.unwrap().unwrap();
        assert!(chunk.delta.is_empty());
        assert_eq!(chunk.finish_reason, Some("stop".to_string()));
    }

    #[test]
    fn test_parse_sse_line_done_marker() {
        let line = "data: [DONE]";
        let result = AnthropicLlmClient::parse_sse_line(line);
        assert!(result.is_ok());

        let chunk = result.unwrap().unwrap();
        assert!(chunk.delta.is_empty());
        assert_eq!(chunk.finish_reason, Some("stop".to_string()));
    }

    #[test]
    fn test_parse_sse_line_error() {
        let line = "data: {\"type\": \"error\", \"error\": {\"type\": \"invalid_request_error\", \"message\": \"Invalid request\"}}";
        let result = AnthropicLlmClient::parse_sse_line(line);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid request"));
    }

    #[test]
    fn test_parse_sse_line_non_data_line() {
        let line = "event: ping";
        let result = AnthropicLlmClient::parse_sse_line(line);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_parse_sse_line_empty_line() {
        let line = "";
        let result = AnthropicLlmClient::parse_sse_line(line);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
