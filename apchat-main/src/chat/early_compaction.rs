//! Early context compaction — removes redundant tool call pairs and summarizes
//! large tool outputs into a context notes system message.
//!
//! Activated by `--context-compact` CLI flag or `APCHAT_CONTEXT_COMPACT=early` env var.
//! Runs every 10 tool call iterations.

use anyhow::Result;
use colored::Colorize;

use crate::APChat;
use apchat_vty::{print_heart_red, print_heart_yellow};
use apchat_models::{Message, ChatRequest, ChatResponse};
use apchat_models::types::ContentPart;
use apchat_logging::log_request_to_file;

use super::history::calculate_conversation_size;

/// How often to run early compaction (every N tool call iterations).
const EARLY_COMPACT_INTERVAL: usize = 10;

/// Minimum number of messages to keep untouched at the end ("recent window").
const RECENT_WINDOW: usize = 8;

/// Minimum tool output size (bytes) to consider for LLM summarization.
const LARGE_OUTPUT_THRESHOLD: usize = 500;

/// Check if early compaction should run at this iteration.
pub fn should_run(tool_call_iterations: usize) -> bool {
    tool_call_iterations > 0 && tool_call_iterations % EARLY_COMPACT_INTERVAL == 0
}

/// Run early compaction on the conversation history.
///
/// Phase 1: Remove error tool call pairs (deterministic, no LLM).
/// Phase 2: Summarize large old tool outputs via LLM, store as context notes.
pub async fn run_early_compaction(chat: &mut APChat, tool_call_iterations: usize) -> Result<()> {
    let size_before = calculate_conversation_size(&chat.messages);
    let msg_count_before = chat.messages.len();

    print_heart_red(&format!(
        "🗜️ {} Early compaction at iteration {}: {:.1} KB, {} messages",
        "EARLY-COMPACT".yellow(),
        tool_call_iterations,
        size_before as f64 / 1024.0,
        msg_count_before,
    ), true);

    // Phase 1: Remove error tool call pairs and superseded reads
    let errors_removed = phase1_remove_errors(chat);
    let dupes_removed = phase1_remove_superseded_reads(chat);

    let size_after_p1 = calculate_conversation_size(&chat.messages);
    if errors_removed > 0 || dupes_removed > 0 {
        print_heart_red(&format!(
            "  Phase 1: removed {} error pairs, {} superseded reads ({:.1} KB -> {:.1} KB)",
            errors_removed, dupes_removed,
            size_before as f64 / 1024.0,
            size_after_p1 as f64 / 1024.0,
        ), true);
    }

    // Phase 2: LLM-assisted summarization of large tool outputs
    let summarized = phase2_summarize_large_outputs(chat).await?;
    if summarized > 0 {
        let size_after_p2 = calculate_conversation_size(&chat.messages);
        print_heart_red(&format!(
            "  Phase 2: summarized {} large tool outputs ({:.1} KB -> {:.1} KB)",
            summarized,
            size_after_p1 as f64 / 1024.0,
            size_after_p2 as f64 / 1024.0,
        ), true);
    }

    Ok(())
}

/// Phase 1: Remove tool call/result pairs where the result was an error.
/// Also removes the assistant message if it only contained that one tool call.
/// Does not touch the recent window.
fn phase1_remove_errors(chat: &mut APChat) -> usize {
    let total = chat.messages.len();
    if total <= RECENT_WINDOW {
        return 0;
    }
    let safe_end = total - RECENT_WINDOW;

    // Collect tool_call_ids of error results outside the recent window
    let error_tool_ids: std::collections::HashSet<String> = chat.messages[..safe_end]
        .iter()
        .filter(|m| m.role == "tool")
        .filter(|m| {
            let text = m.text_only();
            text.starts_with("Error:") || text.starts_with("OPERATION CANCELLED")
                || text.contains("failed") && text.len() < 200
        })
        .filter_map(|m| m.tool_call_id.clone())
        .collect();

    if error_tool_ids.is_empty() {
        return 0;
    }

    let count = error_tool_ids.len();

    // Remove error tool result messages and their corresponding tool calls
    // from assistant messages (outside the recent window)
    let mut indices_to_remove = Vec::new();

    for (i, msg) in chat.messages[..safe_end].iter().enumerate() {
        // Remove tool result messages with error IDs
        if msg.role == "tool" {
            if let Some(ref id) = msg.tool_call_id {
                if error_tool_ids.contains(id) {
                    indices_to_remove.push(i);
                }
            }
        }
        // Remove tool calls from assistant messages
        if msg.role == "assistant" {
            if let Some(ref calls) = msg.tool_calls {
                // If ALL tool calls in this message are errors, remove the whole message
                let all_errors = calls.iter().all(|tc| error_tool_ids.contains(&tc.id));
                if all_errors {
                    indices_to_remove.push(i);
                }
            }
        }
    }

    // Remove in reverse order to preserve indices
    indices_to_remove.sort_unstable();
    indices_to_remove.dedup();
    for &i in indices_to_remove.iter().rev() {
        chat.messages.remove(i);
    }

    count
}

/// Phase 1b: Remove superseded read_file calls — if the same file was read
/// multiple times, remove earlier reads (outside the recent window).
fn phase1_remove_superseded_reads(chat: &mut APChat) -> usize {
    let total = chat.messages.len();
    if total <= RECENT_WINDOW {
        return 0;
    }
    let safe_end = total - RECENT_WINDOW;

    // Track the latest read_file for each file path (by tool_call_id)
    let mut latest_read: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut superseded_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Scan forward to find reads, keeping track of the latest one per file
    for (i, msg) in chat.messages[..safe_end].iter().enumerate() {
        if msg.role == "tool" && msg.name.as_deref() == Some("read_file") {
            if let Some(ref id) = msg.tool_call_id {
                // Find the file_path from the corresponding assistant tool call
                let file_path = find_tool_call_arg(&chat.messages[..i], id, "file_path");
                if let Some(path) = file_path {
                    if let Some(prev_idx) = latest_read.insert(path, i) {
                        // Previous read of same file is now superseded
                        if let Some(prev_id) = chat.messages[prev_idx].tool_call_id.clone() {
                            superseded_ids.insert(prev_id);
                        }
                    }
                }
            }
        }
    }

    if superseded_ids.is_empty() {
        return 0;
    }

    let count = superseded_ids.len();

    // Remove superseded tool results and their assistant tool call entries
    let mut indices_to_remove = Vec::new();
    for (i, msg) in chat.messages[..safe_end].iter().enumerate() {
        if msg.role == "tool" {
            if let Some(ref id) = msg.tool_call_id {
                if superseded_ids.contains(id) {
                    indices_to_remove.push(i);
                }
            }
        }
        if msg.role == "assistant" {
            if let Some(ref calls) = msg.tool_calls {
                let all_superseded = calls.iter().all(|tc| superseded_ids.contains(&tc.id));
                if all_superseded {
                    indices_to_remove.push(i);
                }
            }
        }
    }

    indices_to_remove.sort_unstable();
    indices_to_remove.dedup();
    for &i in indices_to_remove.iter().rev() {
        chat.messages.remove(i);
    }

    count
}

/// Find a tool call argument value from the assistant message that contains the tool call.
fn find_tool_call_arg(messages: &[Message], tool_call_id: &str, arg_name: &str) -> Option<String> {
    for msg in messages.iter().rev() {
        if let Some(ref calls) = msg.tool_calls {
            for call in calls {
                if call.id == tool_call_id {
                    if let Ok(args) = serde_json::from_str::<serde_json::Value>(&call.function.arguments) {
                        return args.get(arg_name).and_then(|v| v.as_str()).map(|s| s.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Phase 2: Ask the LLM to extract key facts from large old tool outputs
/// and replace them with compact summaries.
async fn phase2_summarize_large_outputs(chat: &mut APChat) -> Result<usize> {
    let total = chat.messages.len();
    if total <= RECENT_WINDOW {
        return Ok(0);
    }
    let safe_end = total - RECENT_WINDOW;

    // Find large tool results outside the recent window
    let mut large_outputs: Vec<(usize, String, usize)> = Vec::new(); // (index, tool_name, size)
    for (i, msg) in chat.messages[..safe_end].iter().enumerate() {
        if msg.role == "tool" {
            let text = msg.text_only();
            if text.len() > LARGE_OUTPUT_THRESHOLD {
                let tool_name = msg.name.clone().unwrap_or_else(|| "unknown".to_string());
                large_outputs.push((i, tool_name, text.len()));
            }
        }
    }

    if large_outputs.is_empty() {
        return Ok(0);
    }

    // Build a prompt for the LLM to extract key facts
    let mut extraction_prompt = String::from(
        "You are a context compaction assistant. Below are tool call results from an ongoing conversation. \
        For each one, extract ONLY the key facts, decisions, or information that would be needed later. \
        Output a JSON array of objects with fields: \"index\" (the result number), \"summary\" (concise facts, max 2-3 sentences).\n\
        If a result contains no useful information for future reference, set summary to null.\n\n"
    );

    for (result_num, (idx, tool_name, _size)) in large_outputs.iter().enumerate() {
        let text = chat.messages[*idx].text_only();
        // Truncate very large outputs to avoid overwhelming the summarizer
        let truncated = if text.len() > 4000 {
            format!("{}... [truncated, {} total chars]", &text[..4000], text.len())
        } else {
            text
        };
        extraction_prompt.push_str(&format!(
            "--- Result {} (tool: {}) ---\n{}\n\n",
            result_num, tool_name, truncated
        ));
    }

    extraction_prompt.push_str("Output ONLY the JSON array, no other text.");

    // Use the cheapest model (BluModel) for summarization
    use apchat_models::ModelColor;
    let summary_model = ModelColor::BluModel;

    let summary_history = vec![Message {
        role: "user".to_string(),
        content: vec![ContentPart::Text(extraction_prompt)],
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning: None,
    }];

    let request = ChatRequest {
        model: summary_model.as_str(
            chat.client_config.get_model_override(ModelColor::BluModel).as_deref().map(|s| s.as_str()),
            chat.client_config.get_model_override(ModelColor::GrnModel).as_deref().map(|s| s.as_str()),
            chat.client_config.get_model_override(ModelColor::RedModel).as_deref().map(|s| s.as_str()),
        ).to_string(),
        messages: summary_history,
        tools: vec![],
        tool_choice: "none".to_string(),
        stream: None,
    };

    let api_url = crate::config::get_api_url(&chat.client_config, &summary_model);
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
        print_heart_yellow(&format!("⚠️ Early compaction Phase 2 LLM call failed: {}", response.status()), true);
        return Ok(0);
    }

    let response_text = response.text().await?;
    let chat_response: ChatResponse = serde_json::from_str(&response_text)?;

    let summary_text = chat_response.choices
        .into_iter()
        .next()
        .map(|c| c.message.text_only())
        .unwrap_or_default();

    // Parse the JSON response
    // Try to extract JSON array from the response (may have markdown fences)
    let json_text = extract_json_array(&summary_text);
    let summaries: Vec<serde_json::Value> = match serde_json::from_str(&json_text) {
        Ok(v) => v,
        Err(e) => {
            print_heart_yellow(&format!("⚠️ Failed to parse compaction summaries: {}", e), true);
            return Ok(0);
        }
    };

    // Collect facts for the context notes system message
    let mut context_notes = Vec::new();
    let mut compacted_count = 0;

    for summary_obj in &summaries {
        let result_num = summary_obj.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let summary = summary_obj.get("summary").and_then(|v| v.as_str());

        if let Some(summary_text) = summary {
            if result_num < large_outputs.len() {
                let (msg_idx, tool_name, original_size) = &large_outputs[result_num];
                // Replace the large output with a compact summary
                if *msg_idx < chat.messages.len() {
                    let compact = format!("[Compacted from {} chars] {}", original_size, summary_text);
                    chat.messages[*msg_idx].content = vec![ContentPart::Text(compact)];
                    context_notes.push(format!("- {}: {}", tool_name, summary_text));
                    compacted_count += 1;
                }
            }
        }
    }

    // Append to or create the context notes system message
    if !context_notes.is_empty() {
        let notes_text = format!(
            "\n\n## Context Notes (from compacted tool outputs)\n{}",
            context_notes.join("\n")
        );

        // Find existing context notes system message or create one
        let mut found = false;
        for msg in chat.messages.iter_mut() {
            if msg.role == "system" && msg.text_only().contains("## Context Notes") {
                // Append to existing
                if let Some(ContentPart::Text(ref mut text)) = msg.content.first_mut() {
                    text.push_str(&format!("\n{}", context_notes.join("\n")));
                }
                found = true;
                break;
            }
        }

        if !found {
            // Find the last system message and insert after it
            let insert_pos = chat.messages.iter()
                .take_while(|m| m.role == "system")
                .count();

            chat.messages.insert(insert_pos, Message {
                role: "system".to_string(),
                content: vec![ContentPart::Text(notes_text)],
                tool_calls: None,
                tool_call_id: None,
                name: None,
                reasoning: None,
            });
        }
    }

    Ok(compacted_count)
}

/// Extract a JSON array from text that may contain markdown fences or other wrapping.
fn extract_json_array(text: &str) -> String {
    // Try to find JSON array directly
    if let Some(start) = text.find('[') {
        if let Some(end) = text.rfind(']') {
            return text[start..=end].to_string();
        }
    }
    text.to_string()
}
