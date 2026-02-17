use crate::APChat;
use apchat_toolcore::ContextEdit;
use apchat_vty::{print_heart_red, print_heart_yellow};
use colored::Colorize;
use std::collections::BTreeSet;

/// Apply pending context edits to the conversation history.
///
/// `pre_batch_len` is the number of messages that existed before the current
/// tool-calling batch (i.e., before the assistant message with tool_calls and
/// its tool results were added). Edits can only target indices < pre_batch_len.
///
/// Returns the number of edits applied.
pub(crate) fn apply_pending_context_edits(chat: &mut APChat, pre_batch_len: usize) -> usize {
    let pending: Vec<ContextEdit> = {
        match chat.context_edits.lock() {
            Ok(mut guard) => guard.drain(..).collect(),
            Err(e) => {
                print_heart_yellow(&format!("{} Failed to read context edits: {}", "⚠️".yellow(), e), true);
                return 0;
            }
        }
    };

    if pending.is_empty() {
        return 0;
    }

    let mut edits_applied = 0;

    // First pass: apply EditItem operations (these don't change indices)
    for edit in &pending {
        if let ContextEdit::EditItem { index, new_content } = edit {
            if *index >= pre_batch_len {
                print_heart_yellow(&format!(
                    "{} edit_item: index {} is beyond the pre-batch boundary ({}), skipping",
                    "⚠️".yellow(), index, pre_batch_len
                ), true);
                continue;
            }
            if *index >= chat.messages.len() {
                print_heart_yellow(&format!(
                    "{} edit_item: index {} is out of range ({}), skipping",
                    "⚠️".yellow(), index, chat.messages.len()
                ), true);
                continue;
            }
            if !chat.messages[*index].content.contains("**EDITABLE**") {
                print_heart_yellow(&format!(
                    "{} edit_item: message at index {} does not contain **EDITABLE** marker, skipping",
                    "⚠️".yellow(), index
                ), true);
                continue;
            }
            print_heart_red(&format!(
                "{} Editing context item at index {} (role: {})",
                "✏️".green(), index, chat.messages[*index].role
            ), true);
            chat.messages[*index].content = new_content.clone();
            edits_applied += 1;
        }
    }

    // Second pass: collect all DeleteItems indices, expand to include related messages
    let mut all_delete_indices = BTreeSet::new();
    for edit in &pending {
        if let ContextEdit::DeleteItems { indices } = edit {
            for &idx in indices {
                if idx >= pre_batch_len {
                    print_heart_yellow(&format!(
                        "{} delete_items: index {} is beyond the pre-batch boundary ({}), skipping",
                        "⚠️".yellow(), idx, pre_batch_len
                    ), true);
                    continue;
                }
                if idx >= chat.messages.len() {
                    print_heart_yellow(&format!(
                        "{} delete_items: index {} is out of range ({}), skipping",
                        "⚠️".yellow(), idx, chat.messages.len()
                    ), true);
                    continue;
                }
                if chat.messages[idx].role == "system" {
                    print_heart_yellow(&format!(
                        "{} delete_items: cannot delete system message at index {}, skipping",
                        "⚠️".yellow(), idx
                    ), true);
                    continue;
                }
                all_delete_indices.insert(idx);
            }
        }
    }

    if all_delete_indices.is_empty() {
        return edits_applied;
    }

    // Expand indices to include related assistant+tool groups to preserve role ordering
    let expanded = expand_to_groups(&chat.messages, &all_delete_indices, pre_batch_len);

    if expanded.is_empty() {
        return edits_applied;
    }

    // Delete in reverse order to maintain index validity
    let mut sorted: Vec<usize> = expanded.into_iter().collect();
    sorted.sort_unstable_by(|a, b| b.cmp(a));

    for idx in &sorted {
        print_heart_red(&format!(
            "{} Deleting context item at index {} (role: {}, name: {:?})",
            "🗑️".red(), idx, chat.messages[*idx].role,
            chat.messages[*idx].name
        ), true);
        chat.messages.remove(*idx);
        edits_applied += 1;
    }

    edits_applied
}

/// Expand deletion indices to include related messages to preserve role ordering.
///
/// If an assistant message with tool_calls is marked for deletion, all its
/// tool result messages are also included. If a tool result message is marked
/// for deletion, the corresponding assistant message and all other tool
/// results for that batch are also included.
fn expand_to_groups(
    messages: &[apchat_models::Message],
    indices: &BTreeSet<usize>,
    pre_batch_len: usize,
) -> BTreeSet<usize> {
    let mut expanded = BTreeSet::new();

    for &idx in indices {
        if idx >= pre_batch_len || idx >= messages.len() {
            continue;
        }

        let msg = &messages[idx];

        if msg.role == "assistant" && msg.tool_calls.is_some() {
            // Include this assistant message and all its tool results
            expanded.insert(idx);
            let tool_calls = msg.tool_calls.as_ref().unwrap();
            let tool_call_ids: std::collections::HashSet<&str> = tool_calls
                .iter()
                .map(|tc| tc.id.as_str())
                .collect();

            // Find tool result messages that follow this assistant message
            for j in (idx + 1)..pre_batch_len.min(messages.len()) {
                if messages[j].role == "tool" {
                    if let Some(ref tc_id) = messages[j].tool_call_id {
                        if tool_call_ids.contains(tc_id.as_str()) {
                            expanded.insert(j);
                        }
                    }
                } else {
                    break;
                }
            }
        } else if msg.role == "tool" {
            // Find the corresponding assistant message with tool_calls
            expanded.insert(idx);
            if let Some(ref tc_id) = msg.tool_call_id {
                // Search backwards for the assistant message
                for j in (0..idx).rev() {
                    if messages[j].role == "assistant" {
                        if let Some(ref tool_calls) = messages[j].tool_calls {
                            if tool_calls.iter().any(|tc| tc.id == *tc_id) {
                                expanded.insert(j);
                                // Also include all other tool results for this group
                                let all_ids: std::collections::HashSet<&str> = tool_calls
                                    .iter()
                                    .map(|tc| tc.id.as_str())
                                    .collect();
                                for k in (j + 1)..pre_batch_len.min(messages.len()) {
                                    if messages[k].role == "tool" {
                                        if let Some(ref kid) = messages[k].tool_call_id {
                                            if all_ids.contains(kid.as_str()) {
                                                expanded.insert(k);
                                            }
                                        }
                                    } else {
                                        break;
                                    }
                                }
                                break;
                            }
                        }
                        break;
                    }
                }
            }
        } else {
            // User or other messages: just include directly
            expanded.insert(idx);
        }
    }

    expanded
}

#[cfg(test)]
mod tests {
    use super::*;
    use apchat_models::Message;
    use std::collections::BTreeSet;

    fn make_msg(role: &str, content: &str) -> Message {
        Message {
            role: role.to_string(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning: None,
        }
    }

    fn make_assistant_with_tools(tool_call_ids: Vec<&str>) -> Message {
        use apchat_models::{ToolCall, FunctionCall};
        Message {
            role: "assistant".to_string(),
            content: String::new(),
            tool_calls: Some(tool_call_ids.iter().map(|id| ToolCall {
                id: id.to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "test_tool".to_string(),
                    arguments: "{}".to_string(),
                },
            }).collect()),
            tool_call_id: None,
            name: None,
            reasoning: None,
        }
    }

    fn make_tool_result(tool_call_id: &str, content: &str) -> Message {
        Message {
            role: "tool".to_string(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.to_string()),
            name: Some("test_tool".to_string()),
            reasoning: None,
        }
    }

    #[test]
    fn test_expand_assistant_with_tools() {
        let messages = vec![
            make_msg("system", "system prompt"),
            make_msg("user", "hello"),
            make_assistant_with_tools(vec!["call_1", "call_2"]),
            make_tool_result("call_1", "result 1"),
            make_tool_result("call_2", "result 2"),
            make_msg("assistant", "final response"),
        ];

        // Marking assistant at index 2 should expand to include tool results at 3 and 4
        let mut indices = BTreeSet::new();
        indices.insert(2);
        let expanded = expand_to_groups(&messages, &indices, 6);
        assert_eq!(expanded, BTreeSet::from([2, 3, 4]));
    }

    #[test]
    fn test_expand_tool_result_includes_group() {
        let messages = vec![
            make_msg("system", "system prompt"),
            make_msg("user", "hello"),
            make_assistant_with_tools(vec!["call_1", "call_2"]),
            make_tool_result("call_1", "result 1"),
            make_tool_result("call_2", "result 2"),
            make_msg("assistant", "final response"),
        ];

        // Marking tool result at index 3 should expand to include assistant at 2 and tool at 4
        let mut indices = BTreeSet::new();
        indices.insert(3);
        let expanded = expand_to_groups(&messages, &indices, 6);
        assert_eq!(expanded, BTreeSet::from([2, 3, 4]));
    }

    #[test]
    fn test_expand_user_message() {
        let messages = vec![
            make_msg("system", "system prompt"),
            make_msg("user", "hello"),
            make_msg("assistant", "hi"),
            make_msg("user", "bye"),
        ];

        let mut indices = BTreeSet::new();
        indices.insert(1);
        let expanded = expand_to_groups(&messages, &indices, 4);
        assert_eq!(expanded, BTreeSet::from([1]));
    }

    #[test]
    fn test_expand_respects_pre_batch_len() {
        let messages = vec![
            make_msg("system", "system prompt"),
            make_msg("user", "hello"),
            make_assistant_with_tools(vec!["call_1"]),
            make_tool_result("call_1", "result 1"),
        ];

        // Index 3 is at/beyond pre_batch_len of 3, should be skipped
        let mut indices = BTreeSet::new();
        indices.insert(3);
        let expanded = expand_to_groups(&messages, &indices, 3);
        // Tool result at index 3 is beyond pre_batch_len, skipped
        assert!(expanded.is_empty());
    }
}
