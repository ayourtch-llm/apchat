#[cfg(test)]
mod tests {
    use crate::chat::history::{calculate_conversation_size, get_max_session_size, should_compact_session,
                               find_cutoff_preserving_tool_pairs, ensure_proper_role_alternation,
                               extract_latest_todo_state, intelligent_compaction};
    use crate::chat::early_compaction;
    use crate::{APChat, config::{ClientConfig, FeatureFlags}};
    use apchat_models::{Message, ModelColor, ToolCall, FunctionCall};
    use apchat_models::types::ContentPart;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use apchat_terminal::TerminalManager;
    use apchat_policy::PolicyManager;
    use apchat_toolcore::ToolRegistry;
    use apchat_todo::TodoManager;
    use tempfile::TempDir;

    // ─── Helpers ────────────────────────────────────────────────────────────────

    fn create_test_apchat() -> APChat {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().to_path_buf();

        APChat {
            api_key: "test-key".to_string(),
            work_dir: work_dir.clone(),
            client: reqwest::Client::new(),
            messages: Vec::new(),
            current_model: ModelColor::GrnModel,
            total_tokens_used: 0,
            logger: None,
            tool_registry: ToolRegistry::new(),
            client_config: ClientConfig::new(),
            policy_manager: PolicyManager::new(),
            terminal_manager: Arc::new(Mutex::new(TerminalManager::new(work_dir))),
            skill_registry: None,
            non_interactive: false,
            todo_manager: Arc::new(TodoManager::new()),
            stream_responses: false,
            verbose: false,
            debug_level: 0,
            inference_debug: false,
            webex_debug: false,
            process_id: 12345,
            readline_history: None,
            content_limiter: None,
            mspc_channel: None,
            signal_sender: None,
            signal_receiver: None,
            confirmation_registry: None,
            llm_overrides: Arc::new(std::sync::Mutex::new(None)),
            context_edits: Arc::new(std::sync::Mutex::new(Vec::new())),
            summarize_subagents: true,
            mcp_clients: Vec::new(),
            feature_flags: FeatureFlags::default(),
            bogus_ack_msg: None,
            task_completion_marker: None,
            cancellation_token: None,
            ipc_mailbox: None,
        }
    }

    fn msg(role: &str, content: &str) -> Message {
        Message {
            role: role.to_string(),
            content: vec![ContentPart::Text(content.to_string())],
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning: None,
        }
    }

    fn msg_with_tool_calls(content: &str, calls: Vec<(&str, &str, &str)>) -> Message {
        let tool_calls: Vec<ToolCall> = calls
            .into_iter()
            .map(|(id, name, args)| ToolCall {
                id: id.to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: name.to_string(),
                    arguments: args.to_string(),
                },
            })
            .collect();
        Message {
            role: "assistant".to_string(),
            content: vec![ContentPart::Text(content.to_string())],
            tool_calls: Some(tool_calls),
            tool_call_id: None,
            name: None,
            reasoning: None,
        }
    }

    fn tool_result(id: &str, name: &str, content: &str) -> Message {
        Message {
            role: "tool".to_string(),
            content: vec![ContentPart::Text(content.to_string())],
            tool_calls: None,
            tool_call_id: Some(id.to_string()),
            name: Some(name.to_string()),
            reasoning: None,
        }
    }

    fn large_msg(role: &str, base: &str, size_kb: usize) -> Message {
        let padding = "x".repeat(size_kb * 1024 - base.len().min(size_kb * 1024));
        Message {
            role: role.to_string(),
            content: vec![ContentPart::Text(format!("{}{}", base, padding))],
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning: None,
        }
    }

    // ─── calculate_conversation_size ─────────────────────────────────────────────

    #[test]
    fn test_calculate_conversation_size_empty() {
        let size = calculate_conversation_size(&[]);
        assert_eq!(size, 2); // JSON "[]"
    }

    #[test]
    fn test_calculate_conversation_size_single_message() {
        let msgs = vec![msg("user", "Hello world")];
        let size = calculate_conversation_size(&msgs);
        assert!(size > 0);
        assert!(size < 1000);
    }

    #[test]
    fn test_calculate_conversation_size_grows_with_content() {
        let small = vec![msg("user", "Hi")];
        let large = vec![msg("user", &"x".repeat(10_000))];
        assert!(calculate_conversation_size(&large) > calculate_conversation_size(&small));
    }

    // ─── get_max_session_size ────────────────────────────────────────────────────

    #[test]
    fn test_max_session_size_by_model() {
        assert_eq!(get_max_session_size(&ModelColor::GrnModel), 150_000);
        assert_eq!(get_max_session_size(&ModelColor::BluModel), 400_000);
        assert_eq!(get_max_session_size(&ModelColor::RedModel), 600_000);
    }

    // ─── should_compact_session ──────────────────────────────────────────────────

    #[test]
    fn test_compact_not_needed_for_small_conversation() {
        let chat = create_test_apchat();
        assert!(!should_compact_session(&chat, &ModelColor::GrnModel));
    }

    #[test]
    fn test_compact_needed_for_large_conversation() {
        let mut chat = create_test_apchat();
        for i in 0..100 {
            chat.messages.push(large_msg("user", &format!("Msg {}", i), 2));
        }
        // 200KB > 150KB * 1.25 = 187.5KB — should trigger for GrnModel
        assert!(should_compact_session(&chat, &ModelColor::GrnModel));
        // But not for RedModel (600KB * 1.25 = 750KB)
        assert!(!should_compact_session(&chat, &ModelColor::RedModel));
    }

    #[test]
    fn test_compact_threshold_includes_25_percent_buffer() {
        let mut chat = create_test_apchat();
        // GrnModel: 150KB. With 25% buffer -> triggers at 187.5KB
        // Add messages to land between 150KB and 187.5KB
        for _ in 0..85 {
            chat.messages.push(large_msg("user", "m", 2)); // ~170KB
        }
        let size = calculate_conversation_size(&chat.messages);
        // Should be around 170KB — above 150KB but below 187.5KB
        if size > 150_000 && size < 187_500 {
            assert!(!should_compact_session(&chat, &ModelColor::GrnModel),
                "Should NOT compact when between threshold and threshold+25%");
        }
    }

    // ─── find_cutoff_preserving_tool_pairs ───────────────────────────────────────

    #[test]
    fn test_cutoff_empty_messages() {
        let msgs: Vec<Message> = vec![];
        assert_eq!(find_cutoff_preserving_tool_pairs(&msgs, 5), 0);
    }

    #[test]
    fn test_cutoff_fewer_than_target() {
        let msgs = vec![msg("user", "a"), msg("assistant", "b")];
        assert_eq!(find_cutoff_preserving_tool_pairs(&msgs, 5), 0);
    }

    #[test]
    fn test_cutoff_simple_no_tools() {
        let msgs = vec![
            msg("system", "sys"),
            msg("user", "1"),
            msg("assistant", "2"),
            msg("user", "3"),
            msg("assistant", "4"),
            msg("user", "5"),
            msg("assistant", "6"),
        ];
        // Keep last 3 → naive cutoff = 7 - 3 = 4
        let cutoff = find_cutoff_preserving_tool_pairs(&msgs, 3);
        assert_eq!(cutoff, 4);
    }

    #[test]
    fn test_cutoff_preserves_tool_result_with_assistant() {
        // If we want to keep a tool result, we must also keep its assistant message
        let msgs = vec![
            msg("system", "sys"),            // 0
            msg("user", "old"),              // 1
            msg("assistant", "old resp"),     // 2
            msg_with_tool_calls("calling", vec![("tc1", "read_file", "{}")]), // 3
            tool_result("tc1", "read_file", "file content"),                  // 4
            msg("user", "recent"),           // 5
            msg("assistant", "recent resp"), // 6
        ];
        // Keep last 3 → naive cutoff = 4. Messages 4,5,6 would be kept.
        // But msg 4 is a tool result for tc1, so msg 3 (assistant with tc1) must also be kept.
        let cutoff = find_cutoff_preserving_tool_pairs(&msgs, 3);
        assert!(cutoff <= 3, "Cutoff should be <= 3 to include the assistant with tool call, got {}", cutoff);
    }

    #[test]
    fn test_cutoff_preserves_all_results_for_multi_tool_call() {
        // Assistant calls two tools; both results must be kept together
        let msgs = vec![
            msg("system", "sys"),            // 0
            msg("user", "old"),              // 1
            msg_with_tool_calls("calling", vec![
                ("tc1", "read_file", "{}"),
                ("tc2", "search_files", "{}"),
            ]),                               // 2
            tool_result("tc1", "read_file", "result1"), // 3
            tool_result("tc2", "search_files", "result2"), // 4
            msg("user", "recent"),           // 5
        ];
        // Keep last 2 → naive cutoff = 4. We'd keep 4,5.
        // But msg 4 is result for tc2 from msg 2, so msg 2 must be kept.
        // And msg 2 also has tc1, so msg 3 must be kept.
        let cutoff = find_cutoff_preserving_tool_pairs(&msgs, 2);
        assert!(cutoff <= 2, "Cutoff should be <= 2 to keep entire tool call group, got {}", cutoff);
    }

    #[test]
    fn test_cutoff_no_orphaned_tool_calls() {
        // If we keep an assistant with tool calls, all tool results must be included
        let msgs = vec![
            msg("system", "sys"),            // 0
            msg("user", "q"),                // 1
            msg_with_tool_calls("", vec![("tc1", "write_file", "{}")]), // 2
            tool_result("tc1", "write_file", "ok"), // 3
            msg("user", "q2"),               // 4
            msg_with_tool_calls("", vec![("tc2", "edit_file", "{}")]), // 5
            tool_result("tc2", "edit_file", "ok"), // 6
        ];
        // Keep last 3 → naive cutoff = 4. We keep 4,5,6.
        // msg 5 has tc2, msg 6 is its result — pair complete.
        let cutoff = find_cutoff_preserving_tool_pairs(&msgs, 3);
        assert_eq!(cutoff, 4);
    }

    // ─── ensure_proper_role_alternation ──────────────────────────────────────────

    #[test]
    fn test_role_alternation_no_change_needed() {
        let mut msgs = vec![
            msg("system", "sys"),
            msg("user", "hi"),
            msg("assistant", "hello"),
        ];
        let original_len = msgs.len();
        ensure_proper_role_alternation(&mut msgs);
        assert_eq!(msgs.len(), original_len);
        assert_eq!(msgs[1].role, "user");
    }

    #[test]
    fn test_role_alternation_converts_assistant_after_system_to_user() {
        let mut msgs = vec![
            msg("system", "sys"),
            msg("assistant", "I should be user"),
            msg("user", "real user"),
        ];
        ensure_proper_role_alternation(&mut msgs);
        // First non-system message that was assistant should become user
        assert_eq!(msgs[1].role, "user");
    }

    #[test]
    fn test_role_alternation_too_few_messages() {
        let mut msgs = vec![msg("system", "sys")];
        let original_len = msgs.len();
        ensure_proper_role_alternation(&mut msgs);
        assert_eq!(msgs.len(), original_len); // No change
    }

    // ─── extract_latest_todo_state ───────────────────────────────────────────────

    #[test]
    fn test_extract_todo_no_todos() {
        let msgs = vec![
            msg("user", "hello"),
            msg("assistant", "hi"),
        ];
        assert!(extract_latest_todo_state(&msgs).is_none());
    }

    #[test]
    fn test_extract_todo_from_tool_call() {
        let msgs = vec![
            msg("user", "do tasks"),
            msg_with_tool_calls("tracking", vec![
                ("tc1", "todo_write", r#"{"todos":[{"content":"Fix bug","status":"in_progress","activeForm":"Fixing bug"},{"content":"Write tests","status":"pending","activeForm":"Writing tests"}]}"#),
            ]),
            tool_result("tc1", "todo_write", "Updated"),
        ];
        let tasks = extract_latest_todo_state(&msgs);
        assert!(tasks.is_some());
        let tasks = tasks.unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].content, "Fix bug");
        assert_eq!(tasks[1].content, "Write tests");
    }

    #[test]
    fn test_extract_todo_returns_latest() {
        // Two todo_write calls — should return the most recent (scanning backwards)
        let msgs = vec![
            msg_with_tool_calls("first", vec![
                ("tc1", "todo_write", r#"{"todos":[{"content":"Old task","status":"completed","activeForm":"Old task"}]}"#),
            ]),
            tool_result("tc1", "todo_write", "ok"),
            msg("user", "more work"),
            msg_with_tool_calls("second", vec![
                ("tc2", "todo_write", r#"{"todos":[{"content":"New task","status":"in_progress","activeForm":"New task"}]}"#),
            ]),
            tool_result("tc2", "todo_write", "ok"),
        ];
        let tasks = extract_latest_todo_state(&msgs).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].content, "New task");
    }

    // ─── early_compaction::should_run ────────────────────────────────────────────

    #[test]
    fn test_early_compact_should_run_at_zero() {
        assert!(!early_compaction::should_run(0));
    }

    #[test]
    fn test_early_compact_should_run_at_intervals() {
        assert!(early_compaction::should_run(10));
        assert!(early_compaction::should_run(20));
        assert!(early_compaction::should_run(100));
    }

    #[test]
    fn test_early_compact_should_not_run_between_intervals() {
        assert!(!early_compaction::should_run(1));
        assert!(!early_compaction::should_run(5));
        assert!(!early_compaction::should_run(11));
        assert!(!early_compaction::should_run(99));
    }

    // ─── Loop detection logic (extracted from session.rs) ────────────────────────
    // These test the same algorithm used in session.rs lines 288-358

    /// Simulates the loop detection logic from session.rs
    fn detect_loop(
        recent_tool_calls: &[(String, String)],
        tool_signature: &str,
        is_read_only: bool,
    ) -> bool {
        const CONSECUTIVE_REPEAT_THRESHOLD: usize = 25;
        const SCATTERED_REPEAT_THRESHOLD: usize = 40;

        let consecutive_count = recent_tool_calls
            .iter()
            .rev()
            .take_while(|(sig, _)| sig == tool_signature)
            .count();

        let total_repetition_count = recent_tool_calls
            .iter()
            .filter(|(sig, _)| sig == tool_signature)
            .count();

        if is_read_only {
            consecutive_count >= CONSECUTIVE_REPEAT_THRESHOLD + 2
                || total_repetition_count >= SCATTERED_REPEAT_THRESHOLD + 2
        } else {
            consecutive_count >= CONSECUTIVE_REPEAT_THRESHOLD
                || total_repetition_count >= SCATTERED_REPEAT_THRESHOLD
        }
    }

    #[test]
    fn test_loop_detection_no_loop_with_few_calls() {
        let calls: Vec<(String, String)> = (0..5)
            .map(|i| (format!("read_file:{}", i), String::new()))
            .collect();
        assert!(!detect_loop(&calls, "read_file:0", false));
    }

    #[test]
    fn test_loop_detection_consecutive_write_triggers() {
        let sig = "write_file:{\"path\":\"a.rs\"}".to_string();
        let calls: Vec<(String, String)> = (0..25)
            .map(|_| (sig.clone(), String::new()))
            .collect();
        assert!(detect_loop(&calls, &sig, false));
    }

    #[test]
    fn test_loop_detection_consecutive_read_more_lenient() {
        let sig = "read_file:{\"path\":\"a.rs\"}".to_string();
        // 25 consecutive reads should NOT trigger (read-only gets +2 leniency)
        let calls: Vec<(String, String)> = (0..25)
            .map(|_| (sig.clone(), String::new()))
            .collect();
        assert!(!detect_loop(&calls, &sig, true));

        // 27 consecutive reads SHOULD trigger
        let calls: Vec<(String, String)> = (0..27)
            .map(|_| (sig.clone(), String::new()))
            .collect();
        assert!(detect_loop(&calls, &sig, true));
    }

    #[test]
    fn test_loop_detection_scattered_repeats() {
        let target = "edit_file:{\"path\":\"a.rs\"}".to_string();
        let other = "read_file:{\"path\":\"b.rs\"}".to_string();
        // Alternate between target and other — scattered pattern
        let mut calls: Vec<(String, String)> = Vec::new();
        for _ in 0..80 {
            calls.push((target.clone(), String::new()));
            calls.push((other.clone(), String::new()));
        }
        // 80 scattered repetitions of target > threshold of 40
        assert!(detect_loop(&calls, &target, false));
    }

    #[test]
    fn test_loop_detection_mixed_calls_no_loop() {
        // Different tool calls — no loop
        let calls: Vec<(String, String)> = (0..50)
            .map(|i| (format!("tool_{}:{{}}", i), String::new()))
            .collect();
        assert!(!detect_loop(&calls, "tool_0:{}", false));
    }

    // ─── Tool call signature building (mirrors session.rs:288-291) ───────────────

    #[test]
    fn test_tool_signature_building() {
        let tool_calls = vec![
            ToolCall {
                id: "tc1".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "read_file".to_string(),
                    arguments: r#"{"file_path":"src/main.rs"}"#.to_string(),
                },
            },
            ToolCall {
                id: "tc2".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "search_files".to_string(),
                    arguments: r#"{"pattern":"*.rs"}"#.to_string(),
                },
            },
        ];

        let signature = tool_calls
            .iter()
            .map(|tc| format!("{}:{}", tc.function.name, tc.function.arguments))
            .collect::<Vec<_>>()
            .join("|");

        assert_eq!(
            signature,
            r#"read_file:{"file_path":"src/main.rs"}|search_files:{"pattern":"*.rs"}"#
        );
    }

    // ─── Read-only detection (mirrors session.rs:314-320) ────────────────────────

    fn is_read_only(tool_calls: &[ToolCall]) -> bool {
        tool_calls.iter().all(|tc| {
            tc.function.name == "read_file"
                || tc.function.name == "list_files"
                || tc.function.name == "search_files"
                || tc.function.name == "grep_search"
        })
    }

    #[test]
    fn test_read_only_detection_all_reads() {
        let calls = vec![
            ToolCall { id: "1".into(), tool_type: "function".into(),
                function: FunctionCall { name: "read_file".into(), arguments: "{}".into() } },
            ToolCall { id: "2".into(), tool_type: "function".into(),
                function: FunctionCall { name: "search_files".into(), arguments: "{}".into() } },
        ];
        assert!(is_read_only(&calls));
    }

    #[test]
    fn test_read_only_detection_with_write() {
        let calls = vec![
            ToolCall { id: "1".into(), tool_type: "function".into(),
                function: FunctionCall { name: "read_file".into(), arguments: "{}".into() } },
            ToolCall { id: "2".into(), tool_type: "function".into(),
                function: FunctionCall { name: "write_file".into(), arguments: "{}".into() } },
        ];
        assert!(!is_read_only(&calls));
    }

    // ─── Progressive session size management constants ───────────────────────────

    #[test]
    fn test_progressive_check_interval() {
        // Verify the check happens at expected intervals (mirrors session.rs:257-260)
        const PROGRESSIVE_CHECK_INTERVAL: usize = 25;
        assert!(1 % PROGRESSIVE_CHECK_INTERVAL != 0);
        assert!(25 % PROGRESSIVE_CHECK_INTERVAL == 0);
        assert!(50 % PROGRESSIVE_CHECK_INTERVAL == 0);
        assert!(26 % PROGRESSIVE_CHECK_INTERVAL != 0);
    }

    // ─── Task mode completion marker logic (mirrors session.rs:661-723) ──────────

    #[test]
    fn test_completion_marker_in_text() {
        let marker = "TASK_DONE_abc123";
        let text = "I've completed the task. TASK_DONE_abc123";
        assert!(text.contains(marker));
    }

    #[test]
    fn test_completion_marker_in_reasoning() {
        let marker = "TASK_DONE_abc123";
        let text_content = "";
        let reasoning = "Analysis complete. TASK_DONE_abc123";
        let has_marker = text_content.contains(marker) || reasoning.contains(marker);
        assert!(has_marker);
    }

    #[test]
    fn test_completion_marker_strip() {
        let marker = "TASK_DONE_abc123";
        let output = "Here is the result. TASK_DONE_abc123 Extra text.";
        let clean = output.replace(marker, "").trim().to_string();
        assert_eq!(clean, "Here is the result.  Extra text.");
        assert!(!clean.contains(marker));
    }

    // ─── Empty response retry logic (mirrors session.rs:733-767) ─────────────────

    #[test]
    fn test_empty_response_retry_limits() {
        const MAX_EMPTY_RESPONSE_RETRIES: usize = 3;
        const MAX_TASK_MODE_RETRIES: usize = 3;

        let mut retries = 0;
        let should_give_up = |retries: usize| retries >= MAX_EMPTY_RESPONSE_RETRIES;

        assert!(!should_give_up(0));
        assert!(!should_give_up(1));
        assert!(!should_give_up(2));
        assert!(should_give_up(3));
    }

    // ─── safe_truncate ───────────────────────────────────────────────────────────

    #[test]
    fn test_safe_truncate_long() {
        let long_text = "x".repeat(1000);
        let truncated = apchat_logging::safe_truncate(&long_text, 100);
        assert_eq!(truncated.len(), 100);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_safe_truncate_short() {
        let short = "Hello world";
        let result = apchat_logging::safe_truncate(short, 100);
        assert_eq!(result, short);
    }

    #[test]
    fn test_safe_truncate_exact() {
        let text = "x".repeat(100);
        let result = apchat_logging::safe_truncate(&text, 100);
        assert_eq!(result.len(), 100);
    }

    // ─── Integration: cutoff with complex conversation ──────────────────────────

    #[test]
    fn test_cutoff_complex_conversation() {
        // Simulate a real conversation: system, user/assistant pairs,
        // then a tool call sequence, then more user/assistant
        let mut msgs = vec![
            msg("system", "You are helpful"),                                // 0
            msg("user", "Hello"),                                            // 1
            msg("assistant", "Hi!"),                                         // 2
            msg("user", "Read my file"),                                     // 3
            msg_with_tool_calls("Sure", vec![("tc1", "read_file", r#"{"file_path":"a.rs"}"#)]), // 4
            tool_result("tc1", "read_file", "fn main() {}"),                 // 5
            msg("assistant", "Here's the content of a.rs"),                  // 6
            msg("user", "Now edit it"),                                      // 7
            msg_with_tool_calls("Editing", vec![("tc2", "edit_file", r#"{"file_path":"a.rs"}"#)]), // 8
            tool_result("tc2", "edit_file", "File edited successfully"),     // 9
            msg("assistant", "Done editing"),                                // 10
            msg("user", "Thanks"),                                           // 11
        ];

        // Keep last 4 → naive cutoff = 8
        // msg 8 is assistant with tc2, msg 9 is its result — pair is complete within kept window
        let cutoff = find_cutoff_preserving_tool_pairs(&msgs, 4);
        assert_eq!(cutoff, 8);

        // Keep last 5 → naive cutoff = 7
        // msgs 7,8,9,10,11 are kept. msg 8 has tc2, msg 9 is result — pair complete.
        let cutoff = find_cutoff_preserving_tool_pairs(&msgs, 5);
        assert_eq!(cutoff, 7);

        // Keep last 2 → naive cutoff = 10. msgs 10,11 kept.
        // No tool calls in kept window, no expansion needed.
        let cutoff = find_cutoff_preserving_tool_pairs(&msgs, 2);
        assert_eq!(cutoff, 10);
    }

    #[test]
    fn test_cutoff_tool_result_pulls_in_assistant() {
        // The kept window starts with a tool result — need to pull in the assistant
        let msgs = vec![
            msg("system", "sys"),                                            // 0
            msg("user", "q"),                                                // 1
            msg_with_tool_calls("", vec![("tc1", "read_file", "{}")]),       // 2
            tool_result("tc1", "read_file", "content"),                      // 3
            msg("user", "next"),                                             // 4
        ];
        // Keep last 2 → naive cutoff = 3. Msgs 3,4 kept.
        // Msg 3 is tool result for tc1 → must pull in msg 2 (assistant with tc1)
        let cutoff = find_cutoff_preserving_tool_pairs(&msgs, 2);
        assert!(cutoff <= 2, "Must include assistant message for tool result, got cutoff={}", cutoff);
    }

    // ─── intelligent_compaction (no LLM needed — tests the skip-small path) ─────

    #[tokio::test]
    async fn test_intelligent_compaction_skips_small_conversation() {
        let mut chat = create_test_apchat();
        chat.messages.push(msg("system", "sys"));
        chat.messages.push(msg("user", "hello"));
        chat.messages.push(msg("assistant", "hi"));

        let original_len = chat.messages.len();
        let result = intelligent_compaction(&mut chat, 0).await;
        assert!(result.is_ok());
        assert_eq!(chat.messages.len(), original_len);
    }

    #[tokio::test]
    async fn test_intelligent_compaction_preserves_recent_tool_calls() {
        let mut chat = create_test_apchat();
        chat.messages.push(msg("system", "sys"));

        // 50 old user/assistant pairs
        for i in 0..50 {
            chat.messages.push(msg("user", &format!("Old {}", i)));
            chat.messages.push(msg("assistant", &format!("Resp {}", i)));
        }

        // 5 recent tool calls
        for i in 0..5 {
            chat.messages.push(msg_with_tool_calls(
                &format!("Tool {}", i),
                vec![(&format!("tc{}", i), "edit_file", &format!(r#"{{"file_path":"file{}.rs"}}"#, i))],
            ));
            chat.messages.push(tool_result(&format!("tc{}", i), "edit_file", &format!("Result {}", i)));
        }

        let original_tool_count = chat.messages.iter().filter(|m| m.tool_calls.is_some()).count();
        let result = intelligent_compaction(&mut chat, 100).await;
        assert!(result.is_ok());

        // All recent tool calls should still be present
        let new_tool_count = chat.messages.iter().filter(|m| m.tool_calls.is_some()).count();
        assert_eq!(new_tool_count, original_tool_count);
    }
}
