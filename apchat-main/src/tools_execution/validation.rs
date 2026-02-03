use anyhow::Result;
use colored::Colorize;
use regex::Regex;

use crate::APChat;
use apchat_vty::{print_heart_yellow};
use apchat_models::{ModelColor, Message, ToolCall, FunctionCall, ChatRequest, ChatResponse};
use apchat_logging::{log_request_to_file, log_response_to_file, log_raw_response_to_file};

/// Repair a malformed tool call using AI to fix the JSON arguments
pub(crate) async fn repair_tool_call_with_model(
    chat: &APChat,
    tool_call: &ToolCall,
    error_msg: &str,
) -> Result<ToolCall> {
    print_heart_yellow(&format!("{} Attempting to repair tool call '{}' using AI...", "🔧".bright_yellow(), tool_call.function.name), true);

    let repair_prompt = format!(
        "A tool call failed with a validation error. Please fix the JSON arguments.\n\n\
        Tool name: {}\n\
        Original arguments (malformed): {}\n\
        Error: {}\n\n\
        Requirements:\n\
        - Return ONLY the corrected JSON arguments as a valid JSON object\n\
        - Do not include any explanation, markdown formatting, or extra text\n\
        - Ensure all field types match the schema (integers as numbers, not strings)\n\
        - Common issues: trailing quotes after numbers, string instead of integer values\n\n\
        Corrected JSON arguments:",
        tool_call.function.name,
        tool_call.function.arguments,
        error_msg
    );

    // Create a simple repair request using Kimi (fast and good at structured output)
    let repair_request = ChatRequest {
        model: ModelColor::BluModel.as_str_default().to_string(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: "You are a JSON repair assistant. Return only valid JSON, no explanations.".to_string(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                reasoning: None,
            },
            Message {
                role: "user".to_string(),
                content: repair_prompt,
                tool_calls: None,
                tool_call_id: None,
                name: None,
                reasoning: None,
            },
        ],
        tools: vec![], // No tools for repair request
        tool_choice: "none".to_string(),
        stream: None,
    };

    // Make API call using BluModel's API URL
    let repair_api_url = crate::config::get_api_url(&chat.client_config, &ModelColor::BluModel);

    // Capture request timestamp for response logging correlation
    let request_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Log request to file for persistent debugging
    let _ = log_request_to_file(&repair_api_url, &repair_request, &ModelColor::BluModel, &chat.api_key);

    let api_key = crate::config::get_api_key(&chat.client_config, &chat.api_key, &ModelColor::BluModel);
    let response = chat.client
        .post(&repair_api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&repair_request)
        .send()
        .await?;

    let status = response.status();
    let headers = response.headers().clone();

    if !status.is_success() {
        let error_text = response.text().await?;
        let _ = log_response_to_file(&status, &headers, &error_text, request_timestamp, &ModelColor::BluModel);
        let _ = log_raw_response_to_file(&error_text, request_timestamp, &ModelColor::BluModel);
        anyhow::bail!("Repair API call failed: {}", error_text);
    }

    let response_text = response.text().await?;
    let _ = log_response_to_file(&status, &headers, &response_text, request_timestamp, &ModelColor::BluModel);
    let _ = log_raw_response_to_file(&response_text, request_timestamp, &ModelColor::BluModel);

    let api_response: ChatResponse = serde_json::from_str(&response_text)?;

    if let Some(choice) = api_response.choices.first() {
        let repaired_json = choice.message.content.trim();

        // Validate the repaired JSON
        if let Ok(_) = serde_json::from_str::<serde_json::Value>(repaired_json) {
            print_heart_yellow(&format!("{} Successfully repaired tool call arguments", "✓".bright_green()), true);

            // Return repaired tool call
            Ok(ToolCall {
                id: tool_call.id.clone(),
                tool_type: tool_call.tool_type.clone(),
                function: FunctionCall {
                    name: tool_call.function.name.clone(),
                    arguments: repaired_json.to_string(),
                },
            })
        } else {
            anyhow::bail!("Repaired JSON is still invalid: {}", repaired_json)
        }
    } else {
        anyhow::bail!("No response from repair API call")
    }
}

/// Validate and fix tool calls in the message history
/// Returns true if any fixes were applied
pub(crate) fn validate_and_fix_tool_calls_in_place(chat: &mut APChat) -> Result<bool> {
    let mut fixed_any = false;

    for message in chat.messages.iter_mut() {
        if let Some(tool_calls) = &mut message.tool_calls {
            for tool_call in tool_calls.iter_mut() {
                let original_args = tool_call.function.arguments.clone();

                // Try to parse the arguments as JSON
                match serde_json::from_str::<serde_json::Value>(&tool_call.function.arguments) {
                    Ok(mut json_args) => {
                        // Validate and fix based on tool name
                        let mut needs_fix = false;

                        match tool_call.function.name.as_str() {
                            "open_file" | "peek_file_top_10_lines" => {
                                // Check if start_line or end_line are strings instead of integers
                                if let Some(obj) = json_args.as_object_mut() {
                                    // Check start_line
                                    let start_fix = obj.get("start_line")
                                        .and_then(|v| v.as_str())
                                        .and_then(|s| s.parse::<i64>().ok());

                                    if let Some(num) = start_fix {
                                        obj.insert("start_line".to_string(), serde_json::json!(num));
                                        needs_fix = true;
                                        print_heart_yellow(&format!("{} Fixed start_line: string → integer {}", "🔧".yellow(), num), true);
                                    }

                                    // Check end_line
                                    let end_fix = obj.get("end_line")
                                        .and_then(|v| v.as_str())
                                        .and_then(|s| s.parse::<i64>().ok());

                                    if let Some(num) = end_fix {
                                        obj.insert("end_line".to_string(), serde_json::json!(num));
                                        needs_fix = true;
                                        print_heart_yellow(&format!("{} Fixed end_line: string → integer {}", "🔧".yellow(), num), true);
                                    }
                                }
                            }
                            "search_files" => {
                                // Check if max_results is a string
                                if let Some(obj) = json_args.as_object_mut() {
                                    let max_fix = obj.get("max_results")
                                        .and_then(|v| v.as_str())
                                        .and_then(|s| s.parse::<i64>().ok());

                                    if let Some(num) = max_fix {
                                        obj.insert("max_results".to_string(), serde_json::json!(num));
                                        needs_fix = true;
                                        print_heart_yellow(&format!("{} Fixed max_results: string → integer {}", "🔧".yellow(), num), true);
                                    }
                                }
                            }
                            _ => {}
                        }

                        if needs_fix {
                            tool_call.function.arguments = serde_json::to_string(&json_args)?;
                            fixed_any = true;
                        }
                    }
                    Err(e) => {
                        // JSON parsing failed - try to fix common issues
                        let mut fixed_args = original_args.clone();

                        // Common issue: trailing quote after number (e.g., "end_line": 60")
                        // Pattern: number followed by quote and closing brace
                        let re = Regex::new(r#":\s*(\d+)"\s*([,}])"#)?;
                        if re.is_match(&fixed_args) {
                            fixed_args = re.replace_all(&fixed_args, ": $1$2").to_string();
                            print_heart_yellow(&format!("{} Fixed malformed JSON: removed trailing quotes after numbers", "🔧".yellow()), true);

                            // Verify the fix worked
                            if serde_json::from_str::<serde_json::Value>(&fixed_args).is_ok() {
                                tool_call.function.arguments = fixed_args;
                                fixed_any = true;
                            } else {
                                print_heart_yellow(&format!("{} Failed to fix malformed JSON for tool {}: {}", "⚠️".red(), tool_call.function.name, e), true);
                            }
                        } else {
                            print_heart_yellow(&format!("{} Malformed JSON for tool {}: {}", "⚠️".red(), tool_call.function.name, e), true);
                            tool_call.function.arguments = "{}".to_string();
                            fixed_any = true;
                        }
                    }
                }
            }
        }
    }

    Ok(fixed_any)
}
