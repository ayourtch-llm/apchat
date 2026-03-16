use anyhow::{Context, Result};
use colored::Colorize;
use std::time::Duration;
use tokio::time::sleep;
use uuid;
use chrono;

use crate::APChat;
use crate::MAX_RETRIES;
use apchat_vty::{print_heart_red, print_heart_yellow};
use apchat_models::{ModelColor, Message, Usage, ChatRequest, ChatResponse, ToolCall, FunctionCall};
use apchat_models::types::ContentPart;
use apchat_llm_api::{ToolDefinition, ChatMessage};
use apchat_logging::{log_request, log_request_to_file, log_response, log_response_to_file, log_raw_response_to_file};
use apchat_logging::safe_truncate;
use apchat_toolcore::parse_xml_tool_calls;
use apchat_toolcore::sql_logger;
use super::ApiCallParams;

/// Detect provider from API URL
fn detect_provider(api_url: &str) -> String {
    if api_url.contains("api.anthropic.com") {
        "anthropic".to_string()
    } else if api_url.contains("api.openai.com") {
        "openai".to_string()
    } else if api_url.contains("api.groq.com") {
        "groq".to_string()
    } else if api_url.contains("localhost") || api_url.contains("127.0.0.1") {
        "llama.cpp".to_string()
    } else {
        "unknown".to_string()
    }
}

/// Extract model from request
fn extract_model(request: &ChatRequest) -> String {
    request.model.clone()
}

/// Extract endpoint from API URL
fn extract_endpoint(api_url: &str) -> String {
    // Get the path part of the URL
    if let Some(pos) = api_url.find("/v1") {
        api_url[pos..].to_string()
    } else {
        "/v1/chat/completions".to_string()
    }
}

/// Non-streaming API call for Groq-style APIs (convenience wrapper for backward compatibility)
pub(crate) async fn call_api(
    chat: &APChat,
    orig_messages: &[Message],
) -> Result<(Message, Option<Usage>, ModelColor, Option<String>)> {
    let mut params = ApiCallParams::from_apchat(chat);
    params.messages = orig_messages.to_vec();
    call_api_stateless(&params).await
}

/// Non-streaming API call for Groq-style APIs (stateless version)
pub(crate) async fn call_api_stateless(
    params: &ApiCallParams,
) -> Result<(Message, Option<Usage>, ModelColor, Option<String>)> {
    let current_model = params.current_model.clone();
    // Clone messages and strip reasoning field (only supported by some models like Groq)
    let messages: Vec<Message> = params.messages.iter().map(|m| {
        let mut msg = m.clone();
        msg.reasoning = None; // Strip reasoning field to avoid compatibility issues
        msg
    }).collect();

    // Check if we need to use the new LlmClient system for Anthropic-compatible APIs
    let should_use_anthropic =
        (params.client_config.get_api_url(ModelColor::BluModel).as_ref().map(|u| u.contains("anthropic")).unwrap_or(false)) ||
        (params.client_config.get_api_url(ModelColor::GrnModel).as_ref().map(|u| u.contains("anthropic")).unwrap_or(false)) ||
        (params.client_config.get_api_url(ModelColor::RedModel).as_ref().map(|u| u.contains("anthropic")).unwrap_or(false));

    if params.should_show_debug(1) {
        print_heart_red(&format!("🔧 DEBUG: current_model = {:?}", current_model), true);
        print_heart_red(&format!("🔧 DEBUG: should_use_anthropic = {}", should_use_anthropic), true);
    }
    if should_use_anthropic {
        if params.should_show_debug(1) {
            print_heart_red("🔧 DEBUG: Using call_api_with_llm_client for Anthropic", true);
        }
        return call_api_with_llm_client_stateless(params, &messages, &current_model).await;
    } else {
        if params.should_show_debug(1) {
            print_heart_red("🔧 DEBUG: Using regular OpenAI-style call_api", true);
        }
    }

    // Retry logic with exponential backoff
    let mut retry_count = 0;
    loop {
        // Convert messages for Mistral/Ministral compatibility:
        // Ministral allows ONE optional system message at the start, then alternating user/assistant.
        // Merge all consecutive leading system messages into one system message.
        let mut converted_messages: Vec<Message> = Vec::new();
        let mut leading_system_contents: Vec<String> = Vec::new();
        let mut past_leading_systems = false;

        for msg in messages.iter() {
            if msg.role == "system" && !past_leading_systems {
                // Collect leading system message contents
                leading_system_contents.push(msg.text_only());
            } else {
                // Past the leading system messages
                if !past_leading_systems {
                    // Add merged system message if we collected any
                    if !leading_system_contents.is_empty() {
                        converted_messages.push(Message {
                            role: "system".to_string(),
                            content: vec![ContentPart::Text(leading_system_contents.join("\n\n"))],
                            tool_calls: None,
                            tool_call_id: None,
                            name: None,
                            reasoning: None,
                        });
                    }
                    past_leading_systems = true;
                }
                converted_messages.push(msg.clone());
            }
        }

        let request = ChatRequest {
            model: current_model.as_str(
                params.client_config.get_model_override(ModelColor::BluModel).as_deref().map(|x| x.as_str()),
                params.client_config.get_model_override(ModelColor::GrnModel).as_deref().map(|x| x.as_str()),
                params.client_config.get_model_override(ModelColor::RedModel).as_deref().map(|x| x.as_str())
            ).to_string(),
            messages: converted_messages,
            tools: params.tools.clone(),
            tool_choice: "auto".to_string(),
            stream: None,
        };

        // Get the appropriate API URL based on the current model
        let api_url = crate::config::get_api_url(&params.client_config, &current_model);

        // Capture request timestamp for response logging correlation
        let request_start = std::time::Instant::now();
        let request_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Log request details in verbose mode
        log_request(&api_url, &request, &params.api_key, params.verbose);

        // Log request to file for persistent debugging
        let _ = log_request_to_file(&api_url, &request, &current_model, &params.api_key);

        // Log to SQL database if enabled
        let request_body_str = serde_json::to_string(&request).unwrap_or_default();
        let call_id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().to_rfc3339();
        let _ = sql_logger::log_http(
            call_id.clone(),
            &timestamp,
            &detect_provider(&api_url),
            &extract_model(&request),
            &extract_endpoint(&api_url),
            0,  // Will update on response
            0,
            None,
            None,
            None,
            None,
            Some(request_body_str.clone()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,  // span_name
        ).await;

        let api_key = crate::config::get_api_key(&params.client_config, &params.api_key, &current_model);

        // Serialize request and apply LLM overrides if set (from self-regulate tool)
        let mut request_json = serde_json::to_value(&request)?;
        if let Some(ref overrides_arc) = params.llm_overrides {
            if let Ok(mut guard) = overrides_arc.lock() {
                if let Some(ref mut overrides) = *guard {
                    overrides.apply_to_request(&mut request_json);
                    if overrides.remaining_calls <= 1 {
                        *guard = None;
                    } else {
                        overrides.remaining_calls -= 1;
                    }
                }
            }
        }

        let response = params.http_client
            .post(&api_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_json)
            .send()
            .await?;

        let status = response.status();
        let headers = response.headers().clone();

        // Handle rate limiting with exponential backoff
        if status == 429 {
            if retry_count >= MAX_RETRIES {
                anyhow::bail!("Rate limit exceeded after {} retries", MAX_RETRIES);
            }

            let wait_time = Duration::from_secs(2u64.pow(retry_count));
            print_heart_red(&format!(
                "{} Rate limited. Waiting {} seconds before retry {}/{}...",
                "⏳".yellow(),
                wait_time.as_secs(),
                retry_count + 1,
                MAX_RETRIES
            ), true);
            sleep(wait_time).await;
            retry_count += 1;
            continue;
        }

        // Check for errors and provide detailed debugging
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Unable to read error body".to_string());

            let latency_ms = request_start.elapsed().as_millis() as u64;

            // Log error response in verbose mode
            log_response(&status, &headers, &error_body, params.verbose);
            let _ = log_response_to_file(&status, &headers, &error_body, request_timestamp, &current_model);
            let _ = log_raw_response_to_file(&error_body, request_timestamp, &current_model);

            // Log to SQL database
            let _ = sql_logger::log_http(
                call_id.clone(),
                &timestamp,
                &detect_provider(&api_url),
                &extract_model(&request),
                &extract_endpoint(&api_url),
                status.as_u16(),
                latency_ms,
                None,
                None,
                None,
                None,
                Some(request_body_str.clone()),
                Some(error_body.clone()),
                Some(format!("HTTP {}", status)),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,  // span_name
            ).await;

            // Check if this is a tool-related error
            if status == 400 && error_body.contains("tool_use_failed") {
                print_heart_yellow(&format!("{}{}", "❌ Tool calling error detected!".red().bold(), error_body.yellow()), true);
                // No automatic model switching - let the error propagate
            }

            print_heart_yellow(&format!("{}\nStatus: {}\nError body: {}", "=== API Error Details ===".red(), status, error_body), true);

            // Try to show the request that caused the error
            print_heart_yellow(&format!("\n{}\nMessages count: {}", "Request details:".yellow(), messages.len()), true);
            if let Ok(req_json) = serde_json::to_string_pretty(&request) {
                // Truncate very long requests
                if req_json.chars().count() > 2000 {
                    print_heart_yellow(&format!("Request (truncated): {}...", safe_truncate(&req_json, 2000)), true);
                } else {
                    print_heart_yellow(&format!("Request: {}", req_json), true);
                }
            }
            print_heart_yellow(&format!("{}", "======================".red()), true);

            return Err(anyhow::anyhow!("API request failed with status {}: {}", status, error_body));
        }

        let response_text = response.text().await?;

        let latency_ms = request_start.elapsed().as_millis() as u64;

        // In task mode, dump raw HTTP response for debugging vanishing tokens
        if params.verbose && params.non_interactive {
            print_heart_yellow(&format!("[TASK DEBUG] Raw API response: {}", &response_text), true);
        }

        // Log successful response in verbose mode
        log_response(&status, &headers, &response_text, params.verbose);
        let _ = log_response_to_file(&status, &headers, &response_text, request_timestamp, &current_model);
        let _ = log_raw_response_to_file(&response_text, request_timestamp, &current_model);

        let chat_response: ChatResponse = serde_json::from_str(&response_text)
            .with_context(|| format!("Failed to parse API response: {}", response_text))?;

        // Log to SQL database with usage data
        let input_tokens = chat_response.usage.as_ref().map(|u| u.prompt_tokens as i64);
        let output_tokens = chat_response.usage.as_ref().map(|u| u.completion_tokens as i64);
        
        let _ = sql_logger::log_http(
            call_id.clone(),
            &timestamp,
            &detect_provider(&api_url),
            &extract_model(&request),
            &extract_endpoint(&api_url),
            status.as_u16(),
            latency_ms,
            None,  // ttft_ms not available in non-streaming
            input_tokens,
            output_tokens,
            None,  // cost not calculated here
            Some(request_body_str.clone()),
            Some(response_text.clone()),
            None,
            headers.get("x-request-id").and_then(|h| h.to_str().ok().map(|s| s.to_string())),
            None,
            None,
            None,
            None,
            None,
            None,
            None,  // span_name
        ).await;

        let (mut message, finish_reason) = chat_response
            .choices
            .into_iter()
            .next()
            .map(|c| (c.message, c.finish_reason))
            .context("No response from API")?;

        // If no structured tool calls were received, check for XML format in content
        if message.tool_calls.is_none() {
            if let Some(parsed_calls) = parse_xml_tool_calls(&message.text_only()) {
                print_heart_yellow(&format!("{} Detected XML-format tool calls in content, parsing {} call(s)", "🔧".bright_yellow(), parsed_calls.len()), true);
                message.tool_calls = Some(parsed_calls);
                // Clear the XML from content to avoid displaying it
                message.content = vec![];
            }
        }

        // Also check reasoning field for XML tool calls (some models like Qwen put them there)
        if message.tool_calls.is_none() {
            if let Some(ref reasoning) = message.reasoning {
                if let Some(parsed_calls) = parse_xml_tool_calls(reasoning) {
                    print_heart_yellow(&format!("{} Detected XML-format tool calls in reasoning_content, parsing {} call(s)", "🔧".bright_yellow(), parsed_calls.len()), true);
                    message.tool_calls = Some(parsed_calls);
                    // Clear reasoning since we extracted the tool calls from it
                    message.reasoning = None;
                }
            }
        }

        return Ok((message, chat_response.usage, current_model, finish_reason));
    }
}

/// Call API using the new LlmClient system (for Anthropic and llama.cpp backends) - convenience wrapper
pub(crate) async fn call_api_with_llm_client(
    chat: &APChat,
    messages: &[Message],
    model: &ModelColor,
) -> Result<(Message, Option<Usage>, ModelColor, Option<String>)> {
    let params = ApiCallParams::from_apchat(chat);
    call_api_with_llm_client_stateless(&params, messages, model).await
}

/// Call API using the new LlmClient system (for Anthropic and llama.cpp backends) - stateless version
pub(crate) async fn call_api_with_llm_client_stateless(
    params: &ApiCallParams,
    messages: &[Message],
    model: &ModelColor,
) -> Result<(Message, Option<Usage>, ModelColor, Option<String>)> {
    if params.should_show_debug(1) {
        print_heart_red(&format!("🔧 DEBUG: call_api_with_llm_client called with model: {:?}", model), true);
    }
    if params.should_show_debug(2) {
        print_heart_red(&format!("🔧 DEBUG: client_config.get_api_url(BluModel): {:?}", params.client_config.get_api_url(ModelColor::BluModel)), true);
        print_heart_red(&format!("🔧 DEBUG: client_config.get_api_url(GrnModel): {:?}", params.client_config.get_api_url(ModelColor::GrnModel)), true);
        print_heart_red(&format!("🔧 DEBUG: client_config.get_api_url(RedModel): {:?}", params.client_config.get_api_url(ModelColor::RedModel)), true);
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
    let tools: Vec<ToolDefinition> = params.tools.clone().into_iter().map(|tool| {
        ToolDefinition {
            name: tool.function.name,
            description: tool.function.description,
            parameters: tool.function.parameters,
        }
    }).collect();

    // Create the appropriate LlmClient using the centralized helper
    let llm_client = crate::config::create_client_for_model_color(
        model,
        &params.client_config,
        &params.api_key,
    );

    // Apply and consume one use of LLM request overrides (from self-regulate tool)
    if let Some(ref overrides_arc) = params.llm_overrides {
        if let Ok(mut guard) = overrides_arc.lock() {
            if let Some(ref mut overrides) = *guard {
                llm_client.set_request_overrides(Some(overrides.clone()));
                if overrides.remaining_calls <= 1 {
                    *guard = None;
                } else {
                    overrides.remaining_calls -= 1;
                }
            }
        }
    }

    // Make the API call
    let response = llm_client.chat(chat_messages, tools).await?;

    // Convert the response back to the old format
    let message = Message {
        role: response.message.role,
        content: response.message.content,
        tool_calls: response.message.tool_calls.map(|calls| {
            calls.into_iter().map(|call| ToolCall {
                id: call.id,
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: call.function.name,
                    arguments: call.function.arguments,
                },
            }).collect()
        }),
        tool_call_id: response.message.tool_call_id,
        name: response.message.name,
        reasoning: None,
    };

    let usage = response.usage.map(|u| Usage {
        prompt_tokens: u.prompt_tokens as usize,
        completion_tokens: u.completion_tokens as usize,
        total_tokens: u.total_tokens as usize,
    });

    Ok((message, usage, model.clone(), response.finish_reason))
}
