use anyhow::Result;
use colored::Colorize;

use crate::APChat;
use apchat_vty::{print_heart_red, print_heart_yellow};
use apchat_models::{ModelColor, Message};
use apchat_logging::safe_truncate;

/// Prepare for an LLM call by adding the user message and summarizing history.
///
/// This function extracts the preparation logic that happens before the tool-calling
/// loop starts. It:
/// 1. Adds the user message to the conversation history
/// 2. Performs history summarization/trimming (once, before the loop)
///
/// This is called by the REPL task before entering the tool loop.
pub(crate) async fn prepare_for_llm_call(chat: &mut APChat, user_message: &str) -> Result<()> {
    // Add user message to history
    chat.messages.push(Message {
        role: "user".to_string(),
        content: user_message.to_string(),
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning: None,
    });

    // Summarize ONCE before starting the tool-calling loop, not during it
    // This prevents discarding recent tool results mid-conversation
    crate::chat::history::summarize_and_trim_history(chat).await?;

    Ok(())
}

/// Main chat loop - handles user messages, tool calls, and model interactions
pub(crate) async fn chat(
    chat: &mut APChat,
    user_message: &str,
    cancellation_token: Option<tokio_util::sync::CancellationToken>,
) -> Result<String> {
        chat.messages.push(Message {
            role: "user".to_string(),
            content: user_message.to_string(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning: None,
        });

        // Summarize ONCE before starting the tool-calling loop, not during it
        // This prevents discarding recent tool results mid-conversation
        crate::chat::history::summarize_and_trim_history(chat).await?;

        let mut tool_call_iterations = 0;
        let mut recent_tool_calls: Vec<(String, String)> = Vec::new(); // Track recent tool calls with results
        const MAX_TOOL_ITERATIONS: usize = 250; // Increased limit with intelligent evaluation
        const LOOP_DETECTION_WINDOW: usize = 8; // Check last 8 tool calls
        const PROGRESS_EVAL_INTERVAL: u32 = 50; // Evaluate progress every 50 tool calls
        const CONSECUTIVE_REPEAT_THRESHOLD: usize = 25;
        const SCATTERED_REPEAT_THRESHOLD: usize = 40;

        // Initialize progress evaluator for all operations
        let blu_model_url = crate::config::get_api_url(&chat.client_config, &ModelColor::BluModel);
        let blu_model_key = crate::config::get_api_key(&chat.client_config, &chat.api_key, &ModelColor::BluModel);
        let mut progress_evaluator = Some(apchat_progress::ProgressEvaluator::new(
            std::sync::Arc::new(apchat_llm_api::client::groq::GroqLlmClient::new(
                blu_model_key,
                "kimi".to_string(),
                blu_model_url,
                "progress_evaluator".to_string()
            )),
            0.6, // Minimum confidence threshold
            PROGRESS_EVAL_INTERVAL,
        ));

        // Track tool calls for progress evaluation
        let mut tool_call_history: Vec<apchat_progress::ToolCallInfo> = Vec::new();
        let mut files_changed: std::collections::HashSet<String> = std::collections::HashSet::new();
        let start_time = std::time::Instant::now();
        let mut errors_encountered: Vec<String> = Vec::new();

        loop {
            // Check for cancellation at the start of each iteration
            if let Some(ref token) = cancellation_token {
                if token.is_cancelled() {
                    return Err(anyhow::anyhow!("Chat interrupted by user"));
                }
            }

            // Validate and fix tool calls in the conversation history before sending to API
            // This ensures fixes are permanent and consistent across requests (preserving cache)
            if let Ok(fixed) = crate::tools_execution::validation::validate_and_fix_tool_calls_in_place(chat) {
                if fixed {
                    print_heart_yellow(&format!("{} Tool calls were automatically fixed in conversation history", "✅".green()), true);
                }
            }

            // Race API call against cancellation token
            let (response, usage, current_model, finish_reason, streaming_metrics) = if let Some(ref token) = cancellation_token {
                tokio::select! {
                    result = async {
                        if chat.stream_responses {
                            // Check if this should use the new streaming system
                            // The new system works with all model types now
                            let should_use_anthropic =
                                (chat.client_config.get_api_url(ModelColor::BluModel).as_ref().map(|u| u.contains("anthropic")).unwrap_or(false)) ||
                                (chat.client_config.get_api_url(ModelColor::GrnModel).as_ref().map(|u| u.contains("anthropic")).unwrap_or(false));

                            if should_use_anthropic {
                                // Use the new streaming implementation for Anthropic-compatible APIs
                                if chat.should_show_debug(1) {
                                    print_heart_red("🔧 DEBUG: Using Anthropic-compatible streaming with format translation", true);
                                }
                                crate::api::call_api_streaming_with_llm_client(chat, &chat.messages, &chat.current_model).await
                            } else {
                                // Use old streaming for OpenAI-compatible APIs
                                crate::api::call_api_streaming(chat, &chat.messages).await
                            }
                        } else {
                            // For non-streaming calls, create dummy metrics
                            let (response, usage, current_model, finish_reason) = crate::api::call_api(chat, &chat.messages).await?;
                            let metrics = crate::api::StreamingMetrics {
                                start_time: std::time::Instant::now(),
                                total_tokens: usage.as_ref().map(|u| u.total_tokens).unwrap_or(0),
                                completion_tokens: usage.as_ref().map(|u| u.completion_tokens).unwrap_or(0),
                                prompt_tokens: usage.as_ref().map(|u| Some(u.prompt_tokens)).unwrap_or(None),
                                duration: Some(std::time::Duration::from_millis(100)), // Dummy duration
                            };
                            Ok((response, usage, current_model, finish_reason, metrics))
                        }
                    } => result?,
                    _ = token.cancelled() => {
                        return Err(anyhow::anyhow!("LLM call interrupted by user"));
                    }
                }
            } else {
                // No cancellation token, call normally
                if chat.stream_responses {
                    // Check if this should use the new streaming system
                    // The new system works with all model types now
                    let should_use_anthropic =
                        (chat.client_config.get_api_url(ModelColor::BluModel).as_ref().map(|u| u.contains("anthropic")).unwrap_or(false)) ||
                        (chat.client_config.get_api_url(ModelColor::GrnModel).as_ref().map(|u| u.contains("anthropic")).unwrap_or(false));

                    if should_use_anthropic {
                        // Use the new streaming implementation for Anthropic-compatible APIs
                        if chat.should_show_debug(1) {
                            print_heart_red("🔧 DEBUG: Using Anthropic-compatible streaming with format translation", true);
                        }
                        crate::api::call_api_streaming_with_llm_client(chat, &chat.messages, &chat.current_model).await?
                    } else {
                        // Use old streaming for OpenAI-compatible APIs
                        crate::api::call_api_streaming(chat, &chat.messages).await?
                    }
                } else {
                    // For non-streaming calls, create dummy metrics
                    let (response, usage, current_model, finish_reason) = crate::api::call_api(chat, &chat.messages).await?;
                    let metrics = crate::api::StreamingMetrics {
                        start_time: std::time::Instant::now(),
                        total_tokens: usage.as_ref().map(|u| u.total_tokens).unwrap_or(0),
                        completion_tokens: usage.as_ref().map(|u| u.completion_tokens).unwrap_or(0),
                        prompt_tokens: usage.as_ref().map(|u| Some(u.prompt_tokens)).unwrap_or(None),
                        duration: Some(std::time::Duration::from_millis(100)), // Dummy duration
                    };
                    (response, usage, current_model, finish_reason, metrics)
                }
            };

            if chat.current_model != current_model {
                print_heart_red(&format!("Forced model switch: {:?} -> {:?}", &chat.current_model, &current_model), true);
                chat.current_model = current_model.clone();

                // Removed: Model switch message no longer added to conversation history
            }

            // Display token usage
            if let Some(usage) = &usage {
                chat.total_tokens_used += usage.total_tokens;
                print_heart_red(&format!("======stats for user message: {}", user_message), true);
                print_heart_red(&format!(
                    "{} Prompt: {} | Completion: {} | Total: {} | Session: {} | Finish reason: {:?}",
                    "📊".bright_black(),
                    usage.prompt_tokens.to_string().bright_black(),
                    usage.completion_tokens.to_string().bright_black(),
                    usage.total_tokens.to_string().bright_black(),
                    chat.total_tokens_used.to_string().cyan(),
                    &finish_reason,
                ), true);
            }

            // Display streaming metrics if streaming was used
            if chat.stream_responses {
                streaming_metrics.print_metrics();
            }

            if let Some(tool_calls) = &response.tool_calls {
                tool_call_iterations += 1;

                // Progressive session size management - check periodically during tool execution
                const PROGRESSIVE_CHECK_INTERVAL: usize = 25; // Check every 25 tool calls
                const MID_LOOP_SIZE_THRESHOLD: usize = 400_000; // 400KB for mid-loop compaction
                
                if tool_call_iterations % PROGRESSIVE_CHECK_INTERVAL == 0 {
                    let conversation_size = crate::chat::history::calculate_conversation_size(&chat.messages);
                    if conversation_size > MID_LOOP_SIZE_THRESHOLD {
                        print_heart_red(&format!(
                            "{} Session size reached {:.1} KB during tool execution (iteration {}), performing intelligent compaction...", 
                            "🗜️".yellow(), 
                            conversation_size as f64 / 1024.0,
                            tool_call_iterations
                        ), true);
                        
                        // Perform intelligent compaction that preserves recent tool context
                        if let Err(e) = crate::chat::history::intelligent_compaction(chat, tool_call_iterations).await {
                            print_heart_yellow(&format!("{} Intelligent compaction failed: {}", "⚠️".yellow(), e), true);
                            // Continue without compaction if it fails
                        }
                    }
                }

                // Enhanced loop detection with lower false positive rate
                let tool_signature = tool_calls.iter()
                    .map(|tc| format!("{}:{}", tc.function.name, tc.function.arguments))
                    .collect::<Vec<_>>()
                    .join("|");

                // We'll store the result signature later after execution
                // For now, just track the call signature
                recent_tool_calls.push((tool_signature.clone(), String::new()));

                // Keep only recent tool calls
                if recent_tool_calls.len() > LOOP_DETECTION_WINDOW {
                    recent_tool_calls.remove(0);
                }

                // Check for consecutive identical calls (stronger signal of being stuck)
                let consecutive_count = recent_tool_calls.iter()
                    .rev()
                    .take_while(|(sig, _)| sig == &tool_signature)
                    .count();

                // Check for scattered repetitions in the window
                let total_repetition_count = recent_tool_calls.iter()
                    .filter(|(sig, _)| sig == &tool_signature)
                    .count();

                // Detect if tool is read-only (less likely to be problematic loop)
                let is_read_only = tool_calls.iter().all(|tc|
                    tc.function.name == "read_file" ||
                    tc.function.name == "peek_file_top_10_lines" ||
                    tc.function.name == "list_files" ||
                    tc.function.name == "search_files" ||
                    tc.function.name == "grep_search"
                );

                // More strict threshold for consecutive repeats
                let is_likely_stuck = if is_read_only {
                    // Read-only tools can repeat more before we worry
                    consecutive_count >= CONSECUTIVE_REPEAT_THRESHOLD + 2 ||
                    total_repetition_count >= SCATTERED_REPEAT_THRESHOLD + 2
                } else {
                    // Write operations are more concerning
                    consecutive_count >= CONSECUTIVE_REPEAT_THRESHOLD ||
                    total_repetition_count >= SCATTERED_REPEAT_THRESHOLD
                };

                if is_likely_stuck {
                    let pattern_type = if consecutive_count >= CONSECUTIVE_REPEAT_THRESHOLD {
                        format!("{} consecutive identical calls", consecutive_count)
                    } else {
                        format!("{} identical calls in last {} operations", total_repetition_count, LOOP_DETECTION_WINDOW)
                    };

                    print_heart_yellow(&format!(
                        "{} Detected repeated tool call pattern ({}). Likely stuck in a loop.",
                        "⚠️".red().bold(),
                        pattern_type
                    ), true);
                    chat.messages.push(Message {
                        role: "assistant".to_string(),
                        content: format!(
                            "I apologize, but I'm calling the same tool repeatedly without making progress. \
                            Pattern detected: {}. Please try breaking down your request into smaller, \
                            more specific steps, or provide additional guidance.",
                            pattern_type
                        ),
                        tool_calls: None,
                        tool_call_id: None,
                        name: None,
                        reasoning: None,
                    });
                    return Ok("Repeated tool call pattern detected. Please refine your request.".to_string());
                }

                // Intelligent progress evaluation (replaces hard limit)
                if let Some(ref mut evaluator) = progress_evaluator {
                    // Debug: Show evaluation check
                    if tool_call_iterations % PROGRESS_EVAL_INTERVAL as usize == 0 && tool_call_iterations > 0 {
                        print_heart_yellow(&format!("[DEBUG] Checking if evaluation should trigger at iteration {} (interval: {})",
                                 tool_call_iterations, PROGRESS_EVAL_INTERVAL), true);
                    }

                    if evaluator.should_evaluate(tool_call_iterations as u32) {
                        print_heart_red(&format!("{}", format!("🧠 Evaluating progress after {} tool calls...", tool_call_iterations).bright_blue()), true);
                        print_heart_yellow(&format!("[DEBUG] Progress evaluation triggered at iteration {}", tool_call_iterations), true);

                        // Create tool call summary
                        let mut tool_usage = std::collections::HashMap::new();
                        for call in &tool_call_history {
                            *tool_usage.entry(call.tool_name.clone()).or_insert(0) += 1;
                        }

                        let summary = apchat_progress::ToolCallSummary {
                            total_calls: tool_call_iterations as u32,
                            tool_usage,
                            recent_calls: tool_call_history.iter().rev().take(10).cloned().collect(),
                            current_task: "Executing user request with tools".to_string(),
                            original_request: user_message.to_string(),
                            elapsed_seconds: start_time.elapsed().as_secs(),
                            errors: errors_encountered.clone(),
                            files_changed: files_changed.iter().cloned().collect(),
                        };

                        match evaluator.evaluate_progress(&summary).await {
                            Ok(evaluation) => {
                                print_heart_red(&format!("{}", format!("🎯 Progress Evaluation: {:.0}% complete", evaluation.progress_percentage * 100.0).bright_green()), true);
                                print_heart_red(&format!("{}", format!("📊 Confidence: {:.0}%", evaluation.confidence * 100.0).bright_black()), true);

                                if !evaluation.recommendations.is_empty() {
                                    print_heart_red(&format!("{}", "💡 Recommendations:".bright_cyan()), true);
                                    for rec in &evaluation.recommendations {
                                        print_heart_red(&format!("  • {}", rec), true);
                                    }
                                }

                                if !evaluation.should_continue {
                                    print_heart_red(&format!("{}", "🛑 Agent evaluation suggests stopping or changing strategy".yellow()), true);
                                    chat.messages.push(Message {
                                        role: "assistant".to_string(),
                                        content: format!(
                                            "Based on progress evaluation: {}\n\nRecommendations:\n{}\n\nReasoning: {}",
                                            if evaluation.change_strategy {
                                                "I should change my approach."
                                            } else {
                                                "I should stop and ask for guidance."
                                            },
                                            evaluation.recommendations.join("\n"),
                                            evaluation.reasoning
                                        ),
                                        tool_calls: None,
                                        tool_call_id: None,
                                        name: None,
                                        reasoning: None,
                                    });
                                    return Ok("Intelligent progress evaluation suggested stopping this approach.".to_string());
                                }

                                if evaluation.change_strategy {
                                    print_heart_red(&format!("{}", "🔄 Agent evaluation suggests changing strategy".bright_yellow()), true);
                                    // Remove the progress evaluation system message to avoid breaking conversation flow
                                    // The evaluation is logged but not added to the conversation history
                                    print_heart_red(&format!("{} Progress evaluation: {}", "📊".bright_cyan(), evaluation.reasoning), true);
                                    if !evaluation.recommendations.is_empty() {
                                        print_heart_red(&format!("{} Recommendations: {}", "💡".bright_yellow(), evaluation.recommendations.join(", ")), true);
                                    }
                                } else {
                                    // should_continue is true and no strategy change needed
                                    print_heart_red(&format!("{}", "✅ Progress evaluation: continuing execution with current approach".bright_green()), true);
                                }
                            }
                            Err(e) => {
                                print_heart_yellow(&format!("{} Progress evaluation failed: {}", "⚠️".yellow(), e), true);
                                // Continue with conservative fallback
                            }
                        }
                    }
                }
                /// Normal exit
                let STOP = "stop".to_string();
                if let Some(reason) = finish_reason {
                    if reason == STOP {
                        print_heart_red(&format!("LLM has finished evaluation after {} iterations", tool_call_iterations), true);
                        return Ok("LLM has finished the execution loop.".to_string());
                    }
                }

                // Conservative hard limit as final fallback
                if tool_call_iterations > MAX_TOOL_ITERATIONS {
                    print_heart_yellow(&format!(
                        "{} Reached maximum tool call limit ({} iterations).",
                        "⚠️".yellow(),
                        MAX_TOOL_ITERATIONS
                    ), true);
                    chat.messages.push(Message {
                        role: "assistant".to_string(),
                        content: format!(
                            "I've made {} tool calls for this request. Despite intelligent progress evaluation, \
                            I've reached the safety limit. Please break this down into smaller tasks or provide more specific direction.",
                            tool_call_iterations
                        ),
                        tool_calls: None,
                        tool_call_id: None,
                        name: None,
                        reasoning: None,
                    });
                    return Ok(format!(
                        "Reached maximum tool call limit ({} iterations). Please simplify your request.",
                        tool_call_iterations
                    ));
                }

		let rollback_len = chat.messages.len();

                chat.messages.push(response.clone());

                // Log assistant message with tool calls
                if let Some(logger) = &mut chat.logger {
                    let tool_call_info: Vec<(String, String, String)> = tool_calls
                        .iter()
                        .map(|tc| (
                            tc.id.clone(),
                            tc.function.name.clone(),
                            tc.function.arguments.clone()
                        ))
                        .collect();

                    if std::env::var("DEBUG_LOG").is_ok() {
                        print_heart_yellow(&format!("[DEBUG] Logging {} tool calls", tool_call_info.len()), true);
                    }

                    let model_name = chat.current_model.as_str_default();
                    logger.log_with_tool_calls(
                        "assistant",
                        &response.content,
                        Some(&model_name),
                        tool_call_info,
                    ).await;
                }
                // rollback assistant message with tool calls and the tool results
                let mut rollback = false;

                for tool_call in tool_calls {
                    print_heart_red(&format!(
                        "{} {} with args: {} (iteration {}/{})",
                        "🔧 Calling tool:".yellow(),
                        tool_call.function.name.cyan(),
                        tool_call.function.arguments.bright_black(),
                        tool_call_iterations,
                        MAX_TOOL_ITERATIONS
                    ), true);

                    let tool_start_time = std::time::Instant::now();
                    let result = match chat.execute_tool(
                        &tool_call.function.name,
                        &tool_call.function.arguments,
                    ).await {
                        Ok(r) => r,
                        Err(e) => {
                            let error_msg = e.to_string();
			    if error_msg.contains("Failed to parse") {
                                rollback = true;
                            }

                            // Track error for progress evaluation
                            errors_encountered.push(format!("{}: {}", tool_call.function.name, error_msg));
                            // Make cancellation errors very explicit to the model
                            if error_msg.contains("cancelled by user") ||
                               error_msg.contains("Edit cancelled") ||
                               error_msg.contains("Command cancelled") {
                                // Extract user's comment if present
                                let user_feedback = if error_msg.contains(" - ") {
                                    error_msg.split(" - ").skip(1).collect::<Vec<_>>().join(" - ")
                                } else {
                                    String::new()
                                };

                                let feedback_section = if !user_feedback.is_empty() {
                                    format!("\n\nUSER'S FEEDBACK: {}\nThis feedback explains why the operation was cancelled. Address this concern in your next approach.", user_feedback)
                                } else {
                                    String::new()
                                };

                                format!(
                                    "OPERATION CANCELLED BY USER. The user explicitly cancelled this operation. \
                                    DO NOT retry this same approach. Please acknowledge the cancellation and either:\n\
                                    1. Ask the user what they would like to do instead\n\
                                    2. Try a completely different approach that addresses the user's concerns\n\
                                    3. Stop if this was the only viable option\
                                    {}\n\
                                    \nOriginal message: {}",
                                    feedback_section,
                                    error_msg
                                )
                            } else {
                                format!("Error: {}", error_msg)
                            }
                        }
                    };

                    // Display result to user (truncate for file reading tools)
                    let display_result = if tool_call.function.name == "read_file" || tool_call.function.name == "peek_file_top_10_lines" {
                        let lines: Vec<&str> = result.lines().collect();
                        if lines.len() > 10 {
                            let first_10 = lines[..10].join("\n");
                            let remaining = lines.len() - 10;
                            format!("{}\n\n...and {} more lines", first_10, remaining)
                        } else {
                            result.clone()
                        }
                    } else {
                        result.clone()
                    };

                    print_heart_red(&format!("{} {}", "📋 Result:".green(), display_result.bright_black()), true);

                    // Log tool result
                    if let Some(logger) = &mut chat.logger {
                        if std::env::var("DEBUG_LOG").is_ok() {
                            print_heart_yellow(&format!("[DEBUG] Logging tool result for {}", tool_call.function.name), true);
                        }
                        logger.log_tool_result(
                            &result,
                            &tool_call.id,
                            &tool_call.function.name,
                        ).await;
                    }

                    // Track tool call for progress evaluation
                    let duration = tool_start_time.elapsed();
                    let result_summary = if result.chars().count() > 200 {
                        format!("{} (truncated)", safe_truncate(&result, 200))
                    } else {
                        result.clone()
                    };

                    // Track files that were changed
                    if tool_call.function.name.contains("write_file") ||
                       tool_call.function.name.contains("edit_file") {
                        if let Ok(args) = serde_json::from_str::<serde_json::Value>(&tool_call.function.arguments) {
                            if let Some(file_path) = args.get("file_path").and_then(|v| v.as_str()) {
                                files_changed.insert(file_path.to_string());
                            }
                        }
                    }

                    let call_info = apchat_progress::ToolCallInfo {
                        tool_name: tool_call.function.name.clone(),
                        parameters: tool_call.function.arguments.clone(),
                        success: !result.contains("failed") && !result.contains("cancelled"),
                        duration_ms: duration.as_millis() as u64,
                        result_summary: Some(result_summary),
                    };
                    tool_call_history.push(call_info);

                    chat.messages.push(Message {
                        role: "tool".to_string(),
                        content: result,
                        tool_calls: None,
                        tool_call_id: Some(tool_call.id.clone()),
                        name: Some(tool_call.function.name.clone()),
                        reasoning: None,
                    });
		    if rollback {
                       print_heart_red(&format!("{} {}", "📋 FATAL ERROR - ROLLBACK".red(), "parsing failed"), true);
                       chat.messages.truncate(rollback_len);
                    }
                }

                // Apply any pending context edits from self-edit tools
                crate::chat::context_edit::apply_pending_context_edits(chat, rollback_len);
            } else {
                chat.messages.push(response.clone());
                // If finish_reason is "stop", the agent has naturally finished
                // This explicit check allows agents to signal completion
                if finish_reason.as_deref() == Some("stop") {
                    if chat.should_show_debug(1) {
                        print_heart_red("🔧 DEBUG: Agent signaled completion via finish_reason: stop", true);
                    }
                    return Ok(response.content);
                }
                return Ok(response.content);
            }
        }
}
