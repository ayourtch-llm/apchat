use anyhow::{Context, Result}

#[cfg(test)]
mod auto_save_tests {
    use crate::APChat;
    use apchat_models::{Message, ModelColor};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tempfile::TempDir;
    use apchat_policy::PolicyManager;
    use apchat_toolcore::ToolRegistry;
    use apchat_terminal::TerminalManager;
    use apchat_todo::TodoManager;

    async fn create_test_chat() -> APChat {
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
            agent_coordinator: None,
            use_agents: false,
            client_config: crate::config::ClientConfig::new(),
            policy_manager: PolicyManager::new(),
            terminal_manager: Arc::new(Mutex::new(TerminalManager::new(work_dir))),
            skill_registry: None,
            non_interactive: false,
            todo_manager: Arc::new(TodoManager::new()),
            stream_responses: false,
            verbose: false,
            debug_level: 0,
            process_id: 12345, // Fixed for testing
        }
    }

    #[tokio::test]
    async fn test_auto_save_creates_valid_file() {
        use std::fs;
        
        let mut chat = create_test_chat().await;
        
        // Add test messages
        chat.messages.push(Message {
            role: "user".to_string(),
            content: "Test message".to_string(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning: None,
        });
        
        let temp_dir = TempDir::new().unwrap();
        let test_logs_dir = temp_dir.path().join("logs");
        fs::create_dir_all(&test_logs_dir).unwrap();
        
        // Mock the logs directory for testing
        let file_path = test_logs_dir.join("history-12345.json");
        let result = crate::chat::state::save_state(
            &chat.messages,
            &chat.current_model,
            chat.total_tokens_used,
            file_path.to_str().unwrap()
        );
        
        assert!(result.is_ok(), "Auto-save should succeed");
        
        // Verify file was created
        assert!(file_path.exists(), "History file should exist");
        
        // Verify file contains valid JSON
        let content = fs::read_to_string(&file_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        
        assert_eq!(parsed["messages"].as_array().unwrap().len(), 2); // 1 user + 1 system message
        assert_eq!(parsed["current_model"], "grn_model");
    }

    #[tokio::test]
    async fn test_auto_save_with_multiple_messages() {
        let mut chat = create_test_chat().await;
        
        // Add multiple messages
        for i in 0..5 {
            chat.messages.push(Message {
                role: "user".to_string(),
                content: format!("Message {}", i),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                reasoning: None,
            });
        }
        
        let temp_dir = TempDir::new().unwrap();
        let test_logs_dir = temp_dir.path().join("logs");
        std::fs::create_dir_all(&test_logs_dir).unwrap();
        
        let file_path = test_logs_dir.join("history-12345.json");
        let result = crate::chat::state::save_state(
            &chat.messages,
            &chat.current_model,
            chat.total_tokens_used,
            file_path.to_str().unwrap()
        );
        
        assert!(result.is_ok());
        
        // Verify file was created and can be loaded
        let (loaded_messages, _, _, _) = crate::chat::state::load_state(file_path.to_str().unwrap()).unwrap();
        assert_eq!(loaded_messages.len(), 6); // 5 user + 1 system
    }
};
