use anyhow::Result;
use colored::Colorize;
use std::collections::HashSet;

use crate::APChat;
use apchat_models::{ModelColor, Message, ChatRequest, ChatResponse};
use apchat_logging::{log_request_to_file, safe_truncate};

/// Calculate the current conversation size in bytes by serializing to JSON
pub fn calculate_conversation_size(messages: &[Message]) -> usize {
    match serde_json::to_string(messages) {
        Ok(json) => json.len(),
        Err(_) => 0, // If serialization fails, return 0
    }
}

/// Get maximum session size based on model type (different models have different context limits)
pub fn get_max_session_size(model: &ModelColor) -> usize {
    match model {
        ModelColor::GrnModel => 150_000,  // Conservative for Groq (~8K tokens)
        ModelColor::BluModel => 400_000,  // Moderate for Claude (~100K tokens)
        ModelColor::RedModel => 600_000,  // Larger for local models
    }
}

/// Determine if a session should be compacted based on its current size and model type
pub fn should_compact_session(chat: &APChat, model: &ModelColor) -> bool {
    let conversation_size = calculate_conversation_size(&chat.messages);
    let max_size = get_max_session_size(model);

    // Add a 25% buffer before triggering compaction
    conversation_size > (max_size * 125) / 100
}

/// Find the cutoff point for keeping recent messages while preserving tool call/result pairs
///
/// Returns the index in the messages array where we should start keeping messages.
/// All messages from this index onwards will be kept (not summarized).
///
/// This function ensures that:
/// 1. If we keep an assistant message with tool_calls, we also keep all corresponding tool results
/// 2. If we keep a tool result, we also keep the assistant message with the tool_call
/// 3. We don't orphan any tool calls or tool results
fn find_cutoff_preserving_tool_pairs(
    messages: &[Message],
    target_keep_count: usize,
) -> usize {
    if messages.is_empty() || messages.len() <= target_keep_count {
        return 0; // Keep everything
    }

    // Start with the naive cutoff
    let naive_cutoff = messages.len() - target_keep_count;

    // Track which indices we must keep
    let mut must_keep = HashSet::new();
    for i in naive_cutoff..messages.len() {
        must_keep.insert(i);
    }

    // Expand to include complete tool call/result pairs
    let mut needs_processing = true;
    while needs_processing {
        needs_processing = false;
        let current_must_keep: Vec<usize> = must_keep.iter().copied().collect();

        for &idx in &current_must_keep {
            let message = &messages[idx];

            // If this is an assistant message with tool calls, ensure all results are included
            if message.role == "assistant" {
                if let Some(tool_calls) = &message.tool_calls {
                    let tool_call_ids: HashSet<String> = tool_calls
                        .iter()
                        .map(|tc| tc.id.clone())
                        .collect();

                    // Find all corresponding tool results after this message
                    for j in (idx + 1)..messages.len() {
                        if messages[j].role == "tool" {
                            if let Some(tool_call_id) = &messages[j].tool_call_id {
                                if tool_call_ids.contains(tool_call_id) {
                                    if !must_keep.contains(&j) {
                                        must_keep.insert(j);
                                        needs_processing = true;
                                    }
                                }
                            }
                        } else if messages[j].role == "assistant" || messages[j].role == "user" {
                            // Stop looking for results after we hit another assistant/user message
                            break;
                        }
                    }
                }
            }

            // If this is a tool result, ensure the assistant message with the call is included
            if message.role == "tool" {
                if let Some(tool_call_id) = &message.tool_call_id {
                    // Search backwards for the assistant message with this tool call
                    for j in (0..idx).rev() {
                        if messages[j].role == "assistant" {
                            if let Some(tool_calls) = &messages[j].tool_calls {
                                let has_matching_call = tool_calls
                                    .iter()
                                    .any(|tc| &tc.id == tool_call_id);

                                if has_matching_call {
                                    if !must_keep.contains(&j) {
                                        must_keep.insert(j);
                                        needs_processing = true;
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Find the minimum index that must be kept
    must_keep.into_iter().min().unwrap_or(naive_cutoff)
}

/// Ensure proper role alternation after system messages
/// After system messages, conversation must alternate between user and assistant roles
/// This function ensures that after system messages, the first non-system message is user
fn ensure_proper_role_alternation(messages: &mut Vec<Message>) {
    println!("in ensure_proper_role_alternation");
    if messages.len() <= 2 {
        return; // Not enough messages to have an issue
    }
    
    let mut i = 0;
    
    // Find the first non-system message after system messages
    let mut first_non_system_idx = 0;
    for (i, msg) in messages.iter().enumerate() {
        if msg.role != "system" {
            println!("Found non system message at {} => {}", i, &msg.role);
            first_non_system_idx = i;
            break;
        }
    }
    
    // If the first non-system message is not user, we need to fix it
    if first_non_system_idx > 0 && first_non_system_idx < messages.len() {
        let first_non_system = &messages[first_non_system_idx];
        
        // If it's assistant (not a tool call/result), convert it to user
        if first_non_system.role == "assistant" {
            // Check if it's actually a tool call message
            let is_tool_call = first_non_system.tool_calls.is_some();
            
            if !is_tool_call {
                // Convert assistant to user message to maintain alternation
                let mut fixed_msg = first_non_system.clone();
                fixed_msg.role = "user".to_string();
                messages[first_non_system_idx] = fixed_msg;
            } else {
/*
                // Insert new empty user message before tool call to maintain proper role alternation
                // Tool calls should follow user messages, not system messages
                let tool_call_msg = messages[first_non_system_idx].clone();
                let new_user_msg = Message {
                    role: "user".to_string(),
                    content: String::new(),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                    reasoning: None,
                };
                
                // Replace the tool call message with user message at this position
                messages[first_non_system_idx] = new_user_msg;
                
                // Insert the original tool call message after the new user message
                messages.insert(first_non_system_idx + 1, tool_call_msg);
*/
            }
        }
    }
}

/// Intelligent compaction that preserves recent tool call context while summarizing older messages
/// This is designed to work during tool-calling loops without losing recent context
pub async fn intelligent_compaction(chat: &mut APChat, current_tool_iteration: usize) -> Result<()> {
    const MIN_COMPACT_SIZE: usize = 100_000; // Only compact if above 100KB
    const PRESERVE_RECENT_MESSAGES: usize = 15; // Keep more recent messages than regular summarization
    const PRESERVE_RECENT_TOOL_CALLS: usize = 10; // Keep last 10 tool calls
    
    let conversation_size = calculate_conversation_size(&chat.messages);
    
    // Don't compact small conversations
    if conversation_size <= MIN_COMPACT_SIZE || chat.messages.len() <= PRESERVE_RECENT_MESSAGES * 2 {
        return Ok(());
    }
    
    println!("🗜️ {} Starting intelligent compaction: {:.1} KB, {} messages", 
             "COMPACT".yellow(), 
             conversation_size as f64 / 1024.0, 
             chat.messages.len());
    
    // Find recent tool calls to preserve context
    let mut recent_tool_call_indices = Vec::new();
    let mut tool_call_count = 0;
    
    // Scan from the end to find recent tool calls
    for (i, message) in chat.messages.iter().enumerate().rev() {
        if message.tool_calls.is_some() {
            recent_tool_call_indices.push(i);
            tool_call_count += 1;
            
            if tool_call_count >= PRESERVE_RECENT_TOOL_CALLS {
                break;
            }
        }
    }
    
    // Determine the cutoff point for preserving recent messages
    let preserve_cutoff = if chat.messages.len() > PRESERVE_RECENT_MESSAGES {
        chat.messages.len() - PRESERVE_RECENT_MESSAGES
    } else {
        0
    };
    
    // Determine the tool call cutoff (preserve messages around recent tool calls)
    let tool_call_cutoff = if let Some(&earliest_recent_tool) = recent_tool_call_indices.last() {
        // Preserve some context before the earliest recent tool call
        if earliest_recent_tool > 5 {
            earliest_recent_tool - 5
        } else {
            0
        }
    } else {
        preserve_cutoff
    };
    
    // Use the more conservative cutoff (preserve more)
    let cutoff = std::cmp::min(preserve_cutoff, tool_call_cutoff);
    
    // Don't compact if we don't have enough older messages
    if cutoff <= 1 {
        return Ok(());
    }
    
    // Keep system message and very recent messages
    let system_message = chat.messages.first().cloned();

    // Find the cutoff point that preserves tool call/result pairs
    // Start from the rough cutoff and adjust to avoid breaking tool pairs
    let final_cutoff = find_cutoff_preserving_tool_pairs(&chat.messages, PRESERVE_RECENT_MESSAGES);

    // Extract recent messages (from cutoff to end)
    let recent_messages: Vec<Message> = chat.messages
        .iter()
        .skip(final_cutoff)
        .cloned()
        .collect();

    // Get messages to summarize (everything between system and recent)
    let to_summarize: Vec<Message> = chat.messages
        .iter()
        .skip(1) // Skip system
        .take(final_cutoff.saturating_sub(1))
        .cloned()
        .collect();
    
    if to_summarize.is_empty() {
        return Ok(());
    }
    
    // Use the "other" model for summarization
    let summary_model = match chat.current_model {
        ModelColor::BluModel => ModelColor::GrnModel,
        ModelColor::GrnModel => ModelColor::BluModel,
        ModelColor::RedModel => ModelColor::BluModel,
    };
    
    // Build summary request
    let mut summary_history = vec![Message {
        role: "system".to_string(),
        content: format!(
            "You are {} summarizing a conversation to reduce session size. \
            The conversation is at tool call iteration {}. Focus on preserving \
            key context, decisions, file changes, and task progress. This summary \
            will be used to continue the current work without losing important context.",
            summary_model.display_name(),
            current_tool_iteration
        ),
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning: None,
    }];
    
    // Format the conversation to summarize (more concise during tool execution)
    let conversation_text = to_summarize.iter()
        .map(|m| {
            let role = &m.role;
            let content = if m.content.chars().count() > 300 {
                format!("{}... [truncated]", safe_truncate(&m.content, 300))
            } else {
                m.content.clone()
            };
            
            // Include tool call information if present
            let tool_info = if let Some(tool_calls) = &m.tool_calls {
                let tool_names: Vec<String> = tool_calls.iter()
                    .map(|tc| format!("{}({})", tc.function.name, tc.function.arguments.len()))
                    .collect();
                format!(" [TOOLS: {}]", tool_names.join(", "))
            } else {
                String::new()
            };
            
            format!("{}:{} {}", role, tool_info, content)
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    
    summary_history.push(Message {
        role: "user".to_string(),
        content: format!(
            "Create a concise summary of this conversation segment (tool iteration {}). \
            Focus on: 1) Key decisions made 2) Files modified 3) Current task status 4) \
            Important context needed to continue. Keep it under 200 words.\n\n{}",
            current_tool_iteration,
            conversation_text
        ),
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning: None,
    });
    
    // Call API to get summary using the OTHER model
    let request = ChatRequest {
        model: summary_model.as_str(
            chat.client_config.get_model_override(ModelColor::BluModel).as_deref().map(|s| s.as_str()),
            chat.client_config.get_model_override(ModelColor::GrnModel).as_deref().map(|s| s.as_str()),
            chat.client_config.get_model_override(ModelColor::RedModel).as_deref().map(|s| s.as_str())
        ).to_string(),
        messages: summary_history,
        tools: vec![],
        tool_choice: "none".to_string(),
        stream: None,
    };
    
    // Get the appropriate API URL for the summary model
    let api_url = crate::config::get_api_url(&chat.client_config, &summary_model);
    
    // Log request to file for persistent debugging
    let _ = log_request_to_file(&api_url, &request, &summary_model, &chat.api_key);
    
    let api_key = crate::config::get_api_key(&chat.client_config, &chat.api_key, &summary_model);
    let response = chat.client
        .post(&api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;
    
    if !response.status().is_success() {
        // If summarization fails, do simple trimming
        println!("{} Intelligent compaction failed, doing simple trim", "⚠️".yellow());
        let mut new_history = vec![system_message.unwrap()];
        new_history.extend(recent_messages);
        
        // Ensure proper role alternation after system messages
        ensure_proper_role_alternation(&mut new_history);
        
        chat.messages = new_history;
        return Ok(());
    }
    
    let response_text = response.text().await?;
    let chat_response: ChatResponse = serde_json::from_str(&response_text)?;
    
    if let Some(summary_msg) = chat_response.choices.into_iter().next().map(|c| c.message) {
        let summary = summary_msg.content;
        
        // Rebuild history WITHOUT adding a new system message in the middle
        let mut new_history = vec![];
        
        if let Some(sys_msg) = system_message {
            new_history.push(sys_msg);
        }
        
        // Add recent messages (including recent tool context) - NO system message added!
        new_history.extend(recent_messages);
        
        // Ensure proper role alternation after system messages
        ensure_proper_role_alternation(&mut new_history);
        
        chat.messages = new_history;
        
        // Calculate new size
        let new_size = calculate_conversation_size(&chat.messages);
        
        println!(
            "{} Intelligent compaction complete: {} messages ({:.1} KB) → {} messages ({:.1} KB)",
            "✅".green(),
            conversation_size / 1000, // Approximate original message count
            conversation_size as f64 / 1024.0,
            chat.messages.len(),
            new_size as f64 / 1024.0
        );
    }
    
    Ok(())
}

/// Summarize and trim conversation history when it gets too long
/// Uses another model to summarize the middle portion of the conversation
pub(crate) async fn summarize_and_trim_history(chat: &mut APChat) -> Result<()> {
    // Use dynamic size limits based on model type
    let max_size = get_max_session_size(&chat.current_model);
    const KEEP_RECENT_MESSAGES: usize = 5;

    // Calculate conversation size by serializing to JSON
    let conversation_size = calculate_conversation_size(&chat.messages);

    // Only summarize if conversation exceeds size limit
    if conversation_size <= max_size {
        return Ok(());
    }

    // Use the "other" model for summarization
    let summary_model = match chat.current_model {
        ModelColor::BluModel => ModelColor::GrnModel,
        ModelColor::GrnModel => ModelColor::BluModel,
        ModelColor::RedModel => ModelColor::BluModel, // Use BluModel for summarization when using RedModel
    };

    println!(
        "{} History getting large ({:.1} KB, {} messages) - exceeds {} KB limit for {}. Asking {} to summarize...",
        "📝".yellow(),
        conversation_size as f64 / 1024.0,
        chat.messages.len(),
        max_size as f64 / 1024.0,
        chat.current_model.display_name(),
        summary_model.display_name()
    );

    // Keep system message and recent messages
    let system_message = chat.messages.first().cloned();

    // Find the cutoff point that preserves tool call/result pairs
    let cutoff = find_cutoff_preserving_tool_pairs(&chat.messages, KEEP_RECENT_MESSAGES);

    // Extract recent messages (from cutoff to end)
    let recent_messages: Vec<Message> = chat.messages
        .iter()
        .skip(cutoff)
        .cloned()
        .collect();

    // Get messages to summarize (everything except system and recent)
    let to_summarize: Vec<Message> = chat.messages
        .iter()
        .skip(1) // Skip system
        .take(cutoff.saturating_sub(1))
        .cloned()
        .collect();

    // Build summary request
    let mut summary_history = vec![Message {
        role: "system".to_string(),
        content: format!(
            "You are {}. You are being asked to summarize a conversation that was handled by {}. \
            After summarizing, you may recommend switching to yourself if you believe you would be \
            better suited for the ongoing work based on the context.",
            summary_model.display_name(),
            chat.current_model.display_name()
        ),
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning: None,
    }];

    // Format the conversation to summarize
    let conversation_text = to_summarize.iter()
        .map(|m| {
            let role = &m.role;
            let content = if m.content.chars().count() > 500 {
                format!("{}... [truncated]", safe_truncate(&m.content, 500))
            } else {
                m.content.clone()
            };
            format!("{}: {}", role, content)
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    summary_history.push(Message {
        role: "user".to_string(),
        content: format!(
            "Summarize this conversation history in 2-3 concise sentences, focusing on key context, decisions, and file changes:\n\n{}\n\n\
            Then, based on the recent context and what seems to be the ongoing work, add a separate line starting with 'RECOMMENDATION: ' \
            followed by either 'STAY' (keep current model) or 'SWITCH' (switch to you) and briefly explain why in one sentence.",
            conversation_text
        ),
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning: None,
    });

    // Call API to get summary using the OTHER model
    let request = ChatRequest {
        model: summary_model.as_str(
            chat.client_config.get_model_override(ModelColor::BluModel).as_deref().map(|s| s.as_str()),
            chat.client_config.get_model_override(ModelColor::GrnModel).as_deref().map(|s| s.as_str()),
            chat.client_config.get_model_override(ModelColor::RedModel).as_deref().map(|s| s.as_str())
        ).to_string(),
        messages: summary_history,
        tools: vec![],
        tool_choice: "none".to_string(),
        stream: None,
    };

    // Get the appropriate API URL for the summary model
    let api_url = crate::config::get_api_url(&chat.client_config, &summary_model);

    // Log request to file for persistent debugging
    let _ = log_request_to_file(&api_url, &request, &summary_model, &chat.api_key);

    let api_key = crate::config::get_api_key(&chat.client_config, &chat.api_key, &summary_model);
    let response = chat.client
        .post(&api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        // If summarization fails, just trim without summarizing
        println!("{} Summarization failed, doing simple trim", "⚠️".yellow());
        let mut new_history = vec![system_message.unwrap()];
        new_history.extend(recent_messages);
        
        // Ensure proper role alternation after system messages
        ensure_proper_role_alternation(&mut new_history);
        
        chat.messages = new_history;
        return Ok(());
    }

    let response_text = response.text().await?;
    let chat_response: ChatResponse = serde_json::from_str(&response_text)?;

    if let Some(summary_msg) = chat_response.choices.into_iter().next().map(|c| c.message) {
        let full_response = summary_msg.content;

        // Parse recommendation
        let (summary, recommendation_text) = if let Some(rec_pos) = full_response.find("RECOMMENDATION:") {
            let summary = full_response[..rec_pos].trim().to_string();
            let recommendation = full_response[rec_pos..].trim().to_string();

            println!("{} {}", "💡".bright_cyan(), recommendation);
            (summary, Some(recommendation))
        } else {
            (full_response, None)
        };

        // Rebuild history WITHOUT adding a new system message in the middle
        let mut new_history = vec![];

        if let Some(sys_msg) = system_message {
            new_history.push(sys_msg);
        }

        // Add recent messages - NO summary system message added!
        new_history.extend(recent_messages);
        
        // Ensure proper role alternation after system messages
        ensure_proper_role_alternation(&mut new_history);

        chat.messages = new_history;

        // Calculate new size
        let new_size = serde_json::to_string(&chat.messages)
            .map(|json| json.len())
            .unwrap_or(0);

        println!(
            "{} History trimmed to {} messages ({:.1} KB)",
            "✅".green(),
            chat.messages.len(),
            new_size as f64 / 1024.0
        );

        // If there's a SWITCH recommendation, ask the current model to decide
        if let Some(rec_text) = recommendation_text {
            if rec_text.contains("SWITCH") {
                println!(
                    "{} {} suggests switching. Asking {} to decide...",
                    "🤔".yellow(),
                    summary_model.display_name(),
                    chat.current_model.display_name()
                );

                // Ask current model to decide
                let decision_prompt = vec![
                    Message {
                        role: "system".to_string(),
                        content: format!(
                            "You are {}. You have been handling this conversation.",
                            chat.current_model.display_name()
                        ),
                        tool_calls: None,
                        tool_call_id: None,
                        name: None,
                        reasoning: None,
                    },
                    Message {
                        role: "user".to_string(),
                        content: format!(
                            "{} has reviewed the conversation history and made the following recommendation:\n\n{}\n\n\
                            Based on this recommendation and your understanding of the current context, do you agree to switch to {}? \
                            Respond with only 'AGREE' or 'DECLINE' followed by a brief one-sentence explanation.",
                            summary_model.display_name(),
                            rec_text,
                            summary_model.display_name()
                        ),
                        tool_calls: None,
                        tool_call_id: None,
                        name: None,
                        reasoning: None,
                    },
                ];

                let decision_request = ChatRequest {
                    model: chat.current_model.as_str(
                        chat.client_config.get_model_override(ModelColor::BluModel).as_deref().map(|s| s.as_str()),
                        chat.client_config.get_model_override(ModelColor::GrnModel).as_deref().map(|s| s.as_str()),
                        chat.client_config.get_model_override(ModelColor::RedModel).as_deref().map(|s| s.as_str())
                    ).to_string(),
                    messages: decision_prompt,
                    tools: vec![],
                    tool_choice: "none".to_string(),
                    stream: None,
                };

                // Get the appropriate API URL for the current model
                let decision_api_url = crate::config::get_api_url(&chat.client_config, &chat.current_model);

                // Log request to file for persistent debugging
                let _ = log_request_to_file(&decision_api_url, &decision_request, &chat.current_model, &chat.api_key);

                let api_key = crate::config::get_api_key(&chat.client_config, &chat.api_key, &chat.current_model);
                let decision_response = chat.client
                    .post(&decision_api_url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Content-Type", "application/json")
                    .json(&decision_request)
                    .send()
                    .await?;

                if decision_response.status().is_success() {
                    let decision_text = decision_response.text().await?;
                    if let Ok(decision_chat) = serde_json::from_str::<ChatResponse>(&decision_text) {
                        if let Some(decision_msg) = decision_chat.choices.into_iter().next().map(|c| c.message) {
                            let decision = decision_msg.content;
                            println!("{} {} says: {}", "💬".bright_green(), chat.current_model.display_name(), decision);

                            if decision.to_uppercase().contains("AGREE") {
                                println!(
                                    "{} Switching to {} by mutual agreement",
                                    "🔄".bright_cyan(),
                                    summary_model.display_name()
                                );
                                chat.current_model = summary_model.clone();

                                // Removed: Model switch message no longer added to conversation history
                            } else {
                                println!(
                                    "{} Staying with {}",
                                    "✋".yellow(),
                                    chat.current_model.display_name()
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
