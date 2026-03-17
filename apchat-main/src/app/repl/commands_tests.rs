#[cfg(test)]
mod tests {
    use crate::config::ClientConfig;
    use apchat_models::ModelColor;
    use std::sync::Arc;

    /// Helper to parse "/set model url" command arguments
    /// Returns: (color_option, url_option)
    /// - color_option: None means "all models", Some(color) means specific model
    /// - url_option: None means "show current URLs", Some(url) means "set URL"
    fn parse_set_model_url_args(line: &str) -> (Option<ModelColor>, Option<String>) {
        // Remove "/set model url" prefix and trim
        let args = line.strip_prefix("/set model url").unwrap_or(line).trim();
        
        if args.is_empty() {
            // "/set model url" - show current URLs
            return (None, None);
        }
        
        if args == "help" || args == "--help" || args == "-h" {
            // "/set model url help" - show help
            return (Some(ModelColor::BluModel), Some("help".to_string()));
        }
        
        let parts: Vec<&str> = args.split_whitespace().collect();
        
        if parts.len() == 1 {
            // "/set model url <url>" - set URL for all models
            (None, Some(parts[0].to_string()))
        } else if parts.len() >= 2 {
            // "/set model url <color> <url>" - set URL for specific model
            let color = match parts[0].to_lowercase().as_str() {
                "blu" | "blue" => ModelColor::BluModel,
                "grn" | "green" => ModelColor::GrnModel,
                "red" => ModelColor::RedModel,
                _ => return (Some(ModelColor::BluModel), Some("invalid".to_string())), // Error case
            };
            (Some(color), Some(parts[1].to_string()))
        } else {
            (None, None)
        }
    }

    #[test]
    fn test_parse_set_model_url_show_all() {
        // "/set model url" (no args) should return (None, None)
        let (color, url) = parse_set_model_url_args("/set model url");
        assert_eq!(color, None);
        assert_eq!(url, None);
    }

    #[test]
    fn test_parse_set_model_url_show_all_with_extra_space() {
        // "/set model url " (trailing space) should return (None, None)
        let (color, url) = parse_set_model_url_args("/set model url ");
        assert_eq!(color, None);
        assert_eq!(url, None);
    }

    #[test]
    fn test_parse_set_model_url_blu() {
        // "/set model url blu <url>" should return (Some(BluModel), Some(url))
        let (color, url) = parse_set_model_url_args("/set model url blu http://localhost:8080");
        assert_eq!(color, Some(ModelColor::BluModel));
        assert_eq!(url, Some("http://localhost:8080".to_string()));
    }

    #[test]
    fn test_parse_set_model_url_blue() {
        // "/set model url blue <url>" should also work (alias)
        let (color, url) = parse_set_model_url_args("/set model url blue http://localhost:8080");
        assert_eq!(color, Some(ModelColor::BluModel));
        assert_eq!(url, Some("http://localhost:8080".to_string()));
    }

    #[test]
    fn test_parse_set_model_url_grn() {
        // "/set model url grn <url>" should return (Some(GrnModel), Some(url))
        let (color, url) = parse_set_model_url_args("/set model url grn http://localhost:9090");
        assert_eq!(color, Some(ModelColor::GrnModel));
        assert_eq!(url, Some("http://localhost:9090".to_string()));
    }

    #[test]
    fn test_parse_set_model_url_green() {
        // "/set model url green <url>" should also work (alias)
        let (color, url) = parse_set_model_url_args("/set model url green http://localhost:9090");
        assert_eq!(color, Some(ModelColor::GrnModel));
        assert_eq!(url, Some("http://localhost:9090".to_string()));
    }

    #[test]
    fn test_parse_set_model_url_red() {
        // "/set model url red <url>" should return (Some(RedModel), Some(url))
        let (color, url) = parse_set_model_url_args("/set model url red http://localhost:7070");
        assert_eq!(color, Some(ModelColor::RedModel));
        assert_eq!(url, Some("http://localhost:7070".to_string()));
    }

    #[test]
    fn test_parse_set_model_url_all_models() {
        // "/set model url <url>" (no color) should return (None, Some(url))
        let (color, url) = parse_set_model_url_args("/set model url http://localhost:8080");
        assert_eq!(color, None);
        assert_eq!(url, Some("http://localhost:8080".to_string()));
    }

    #[test]
    fn test_parse_set_model_url_invalid_color() {
        // "/set model url invalid <url>" should return error indicator
        let (color, url) = parse_set_model_url_args("/set model url invalid http://localhost:8080");
        assert_eq!(url, Some("invalid".to_string()));
    }

    #[test]
    fn test_parse_set_model_url_help() {
        // "/set model url help" should return help indicator
        let (color, url) = parse_set_model_url_args("/set model url help");
        assert_eq!(url, Some("help".to_string()));
    }

    #[test]
    fn test_client_config_set_api_url_single_model() {
        // Test that set_api_url only affects the specified model
        let mut config = ClientConfig::new();
        let url = "http://localhost:8080/v1";
        
        config.set_api_url(ModelColor::BluModel, Some(url.to_string()));
        
        assert_eq!(config.get_api_url(ModelColor::BluModel), Some(&url.to_string()));
        assert_eq!(config.get_api_url(ModelColor::GrnModel), None);
        assert_eq!(config.get_api_url(ModelColor::RedModel), None);
    }

    #[test]
    fn test_client_config_set_api_url_all_models() {
        // Test setting URL for all models
        let mut config = ClientConfig::new();
        let url = "http://localhost:8080/v1";
        
        config.set_api_url(ModelColor::BluModel, Some(url.to_string()));
        config.set_api_url(ModelColor::GrnModel, Some(url.to_string()));
        config.set_api_url(ModelColor::RedModel, Some(url.to_string()));
        
        assert_eq!(config.get_api_url(ModelColor::BluModel), Some(&url.to_string()));
        assert_eq!(config.get_api_url(ModelColor::GrnModel), Some(&url.to_string()));
        assert_eq!(config.get_api_url(ModelColor::RedModel), Some(&url.to_string()));
    }

    #[tokio::test]
    async fn test_cmd_set_model_url_integration() {
        // Integration test for the full cmd_set_model_url command
        // This test verifies the command dispatch works correctly
        use crate::config::ClientConfig;
        use crate::APChat;
        use apchat_models::ModelColor;
        use apchat_policy::PolicyManager;
        use apchat_toolcore::ToolRegistry;
        use apchat_terminal::TerminalManager;
        use apchat_todo::TodoManager;
        use crate::config::FeatureFlags;
        use std::sync::Arc;
        use tokio::sync::Mutex;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().to_path_buf();

        let mut chat = APChat {
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
            terminal_manager: Arc::new(Mutex::new(TerminalManager::new(work_dir.clone()))),
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
        };

        let current_model = Arc::new(std::sync::RwLock::new(ModelColor::GrnModel));

        // Test setting URL for all models via dispatch_command
        let result = crate::app::repl::commands::dispatch_command(
            &mut chat,
            "/set model url http://localhost:8080/v1",
            &current_model,
        ).await;
        
        assert_eq!(result, crate::app::repl::commands::CommandResult::Handled);
        assert_eq!(chat.client_config.get_api_url(ModelColor::BluModel), Some(&"http://localhost:8080/v1".to_string()));
        assert_eq!(chat.client_config.get_api_url(ModelColor::GrnModel), Some(&"http://localhost:8080/v1".to_string()));
        assert_eq!(chat.client_config.get_api_url(ModelColor::RedModel), Some(&"http://localhost:8080/v1".to_string()));

        // Test setting URL for specific model via dispatch_command
        let result = crate::app::repl::commands::dispatch_command(
            &mut chat,
            "/set model url blu http://localhost:9090/v1",
            &current_model,
        ).await;
        
        assert_eq!(result, crate::app::repl::commands::CommandResult::Handled);
        assert_eq!(chat.client_config.get_api_url(ModelColor::BluModel), Some(&"http://localhost:9090/v1".to_string()));
        // Other models should still have the old URL
        assert_eq!(chat.client_config.get_api_url(ModelColor::GrnModel), Some(&"http://localhost:8080/v1".to_string()));
        assert_eq!(chat.client_config.get_api_url(ModelColor::RedModel), Some(&"http://localhost:8080/v1".to_string()));
    }
}