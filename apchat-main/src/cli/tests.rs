#[cfg(test)]
mod tests {
    use super::{Cli, Commands};
    use clap::Parser;
    use std::env;

    // Helper function to parse CLI args from a string slice
    fn parse_cli_from_args(args: &[&str]) -> Result<Cli, clap::Error> {
        // Simulate command line arguments
        let mut cli_args = vec!["apchat"];
        cli_args.extend(args);
        
        Cli::try_parse_from(cli_args)
    }

    #[test]
    fn test_default_cli_parsing() -> Result<(), Box<dyn std::error::Error>> {
        // RED: This test should initially fail because we haven't implemented the test yet
        let cli = parse_cli_from_args(&[])?;
        
        assert!(cli.command.is_none());
        assert!(!cli.interactive); // Default should be false
        assert!(!cli.agents);
        assert!(!cli.auto_confirm);
        assert!(!cli.stream);
        assert!(!cli.verbose);
        assert!(!cli.web);
        assert_eq!(cli.web_port, 8080);
        assert_eq!(cli.web_bind, "127.0.0.1");
        assert!(!cli.web_attachable);
        assert!(!cli.learn_policies);
        
        Ok(())
    }

    #[test]
    fn test_interactive_flag() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&["--interactive"])?;
        
        assert!(cli.interactive);
        
        Ok(())
    }

    #[test]
    fn test_short_interactive_flag() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&["-i"])?;
        
        assert!(cli.interactive);
        
        Ok(())
    }

    #[test]
    fn test_agents_flag() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&["--agents"])?;
        
        assert!(cli.agents);
        
        Ok(())
    }

    #[test]
    fn test_auto_confirm_flag() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&["--auto-confirm"])?;
        
        assert!(cli.auto_confirm);
        
        Ok(())
    }

    #[test]
    fn test_stream_flag() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&["--stream"])?;
        
        assert!(cli.stream);
        
        Ok(())
    }

    #[test]
    fn test_verbose_flag_short() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&["-v"])?;
        
        assert!(cli.verbose);
        
        Ok(())
    }

    #[test]
    fn test_verbose_flag_long() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&["--verbose"])?;
        
        assert!(cli.verbose);
        
        Ok(())
    }

    #[test]
    fn test_task_argument() -> Result<(), Box<dyn std::error::Error>> {
        let task_text = "help me debug this issue";
        let cli = parse_cli_from_args(&["--task", task_text])?;
        
        assert_eq!(cli.task, Some(task_text.to_string()));
        
        Ok(())
    }

    #[test]
    fn test_pretty_flag_with_task() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&["--task", "test", "--pretty"])?;
        
        assert!(cli.pretty);
        
        Ok(())
    }

    #[test]
    fn test_llama_cpp_url_flag() -> Result<(), Box<dyn std::error::Error>> {
        let url = "http://localhost:8080";
        let cli = parse_cli_from_args(&["--llama-cpp-url", url])?;
        
        assert_eq!(cli.llama_cpp_url, Some(url.to_string()));
        
        Ok(())
    }

    #[test]
    fn test_individual_model_urls() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&[
            "--api-url-blu-model", "http://localhost:8080",
            "--api-url-grn-model", "http://localhost:8081",
            "--api-url-red-model", "http://localhost:8082"
        ])?;
        
        assert_eq!(cli.api_url_blu_model, Some("http://localhost:8080".to_string()));
        assert_eq!(cli.api_url_grn_model, Some("http://localhost:8081".to_string()));
        assert_eq!(cli.api_url_red_model, Some("http://localhost:8082".to_string()));
        
        Ok(())
    }

    #[test]
    fn test_model_overrides() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&[
            "--model-blu-model", "claude-3-sonnet",
            "--model-grn-model", "llama-3-8b",
            "--model-red-model", "gpt-4"
        ])?;
        
        assert_eq!(cli.model_blu_model, Some("claude-3-sonnet".to_string()));
        assert_eq!(cli.model_grn_model, Some("llama-3-8b".to_string()));
        assert_eq!(cli.model_red_model, Some("gpt-4".to_string()));
        
        Ok(())
    }

    #[test]
    fn test_global_model_override() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&["--model", "claude-3-haiku"])?;
        
        assert_eq!(cli.model, Some("claude-3-haiku".to_string()));
        
        Ok(())
    }

    #[test]
    fn test_backend_types() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&[
            "--blu-backend", "anthropic",
            "--grn-backend", "groq",
            "--red-backend", "llama"
        ])?;
        
        assert_eq!(cli.blu_backend, Some("anthropic".to_string()));
        assert_eq!(cli.grn_backend, Some("groq".to_string()));
        assert_eq!(cli.red_backend, Some("llama".to_string()));
        
        Ok(())
    }

    #[test]
    fn test_api_keys() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&[
            "--blu-key", "sk-blu-key",
            "--grn-key", "sk-grn-key",
            "--red-key", "sk-red-key"
        ])?;
        
        assert_eq!(cli.blu_key, Some("sk-blu-key".to_string()));
        assert_eq!(cli.grn_key, Some("sk-grn-key".to_string()));
        assert_eq!(cli.red_key, Some("sk-red-key".to_string()));
        
        Ok(())
    }

    #[test]
    fn test_web_server_flags() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&["--web", "--web-port", "3000", "--web-attachable"])?;
        
        assert!(cli.web);
        assert_eq!(cli.web_port, 3000);
        assert!(cli.web_attachable);
        
        Ok(())
    }

    #[test]
    fn test_policy_file_flag() -> Result<(), Box<dyn std::error::Error>> {
        let policy_path = "/custom/path/policies.toml";
        let cli = parse_cli_from_args(&["--policy-file", policy_path])?;
        
        assert_eq!(cli.policy_file, Some(policy_path.to_string()));
        
        Ok(())
    }

    #[test]
    fn test_learn_policies_flag() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&["--learn-policies"])?;
        
        assert!(cli.learn_policies);
        
        Ok(())
    }

    #[test]
    fn test_terminal_backend_flag() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&["--terminal-backend", "tmux"])?;
        
        assert_eq!(cli.terminal_backend, Some("tmux".to_string()));
        
        Ok(())
    }

    // Test subcommands
    #[test]
    fn test_read_command() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&["read", "src/main.rs"])?;
        
        match cli.command {
            Some(Commands::Read { file_path }) => {
                assert_eq!(file_path, "src/main.rs");
            }
            _ => panic!("Expected Read command"),
        }
        
        Ok(())
    }

    #[test]
    fn test_write_command() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&["write", "test.txt", "hello world"])?;
        
        match cli.command {
            Some(Commands::Write { file_path, content }) => {
                assert_eq!(file_path, "test.txt");
                assert_eq!(content, "hello world");
            }
            _ => panic!("Expected Write command"),
        }
        
        Ok(())
    }

    #[test]
    fn test_edit_command() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&[
            "edit", "test.txt", 
            "--old-content", "old", 
            "--new-content", "new"
        ])?;
        
        match cli.command {
            Some(Commands::Edit { file_path, old_content, new_content }) => {
                assert_eq!(file_path, "test.txt");
                assert_eq!(old_content, "old");
                assert_eq!(new_content, "new");
            }
            _ => panic!("Expected Edit command"),
        }
        
        Ok(())
    }

    #[test]
    fn test_edit_command_short_flags() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&[
            "edit", "test.txt", 
            "-o", "old", 
            "-n", "new"
        ])?;
        
        match cli.command {
            Some(Commands::Edit { file_path, old_content, new_content }) => {
                assert_eq!(file_path, "test.txt");
                assert_eq!(old_content, "old");
                assert_eq!(new_content, "new");
            }
            _ => panic!("Expected Edit command"),
        }
        
        Ok(())
    }

    #[test]
    fn test_list_command_default_pattern() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&["list"])?;
        
        match cli.command {
            Some(Commands::List { pattern }) => {
                assert_eq!(pattern, "*");
            }
            _ => panic!("Expected List command"),
        }
        
        Ok(())
    }

    #[test]
    fn test_list_command_custom_pattern() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&["list", "src/*.rs"])?;
        
        match cli.command {
            Some(Commands::List { pattern }) => {
                assert_eq!(pattern, "src/*.rs");
            }
            _ => panic!("Expected List command"),
        }
        
        Ok(())
    }

    #[test]
    fn test_search_command_basic() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&["search", "function test"])?;
        
        match cli.command {
            Some(Commands::Search { query, pattern, regex, case_insensitive, max_results }) => {
                assert_eq!(query, "function test");
                assert_eq!(pattern, "*.rs");
                assert!(!regex);
                assert!(!case_insensitive);
                assert_eq!(max_results, 100);
            }
            _ => panic!("Expected Search command"),
        }
        
        Ok(())
    }

    #[test]
    fn test_search_command_with_flags() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&[
            "search", "test.*function",
            "--pattern", "*.js",
            "--regex",
            "--case-insensitive",
            "--max-results", "50"
        ])?;
        
        match cli.command {
            Some(Commands::Search { query, pattern, regex, case_insensitive, max_results }) => {
                assert_eq!(query, "test.*function");
                assert_eq!(pattern, "*.js");
                assert!(regex);
                assert!(case_insensitive);
                assert_eq!(max_results, 50);
            }
            _ => panic!("Expected Search command"),
        }
        
        Ok(())
    }

    #[test]
    fn test_environment_variable_web_port() -> Result<(), Box<dyn std::error::Error>> {
        // Set environment variable temporarily
        env::set_var("APCHAT_WEB_PORT", "9000");
        
        let cli = parse_cli_from_args(&["--web"])?;
        
        assert_eq!(cli.web_port, 9000);
        
        // Clean up
        env::remove_var("APCHAT_WEB_PORT");
        
        Ok(())
    }

    #[test]
    fn test_environment_variable_web_bind() -> Result<(), Box<dyn std::error::Error>> {
        env::set_var("APCHAT_WEB_BIND", "0.0.0.0");
        
        let cli = parse_cli_from_args(&["--web"])?;
        
        assert_eq!(cli.web_bind, "0.0.0.0");
        
        env::remove_var("APCHAT_WEB_BIND");
        
        Ok(())
    }

    #[test]
    fn test_shell_completion_generation() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_from_args(&["--generate", "bash"])?;
        
        assert!(matches!(cli.generate, Some(clap_complete::Shell::Bash)));
        
        Ok(())
    }

    #[test]
    fn test_invalid_command_should_fail() {
        let result = parse_cli_from_args(&["--invalid-flag"]);
        
        assert!(result.is_err());
    }

  #[cfg(test)]
mod cli_tests {
    use apchat_llm_api::config::parse_model_attings;
    use apchat_llm_api::BackendType;
    use apchat_llm_api::{ANTHROPIC_API_URL, GROQ_API_URL, OPENAI_API_URL};

    #[test]
    fn test_model_parsing_integration() {
        // Test model@backend format
        let (model, backend, url) = parse_model_attings("foo@anthropic");
        assert_eq!(model, "foo");
        assert_eq!(backend, Some(BackendType::Anthropic));
        assert_eq!(url, None);

        // Test model@backend(url) format  
        let (model, backend, url) = parse_model_attings("foo@anthropic(https://custom.anthropic.com)");
        assert_eq!(model, "foo");
        assert_eq!(backend, Some(BackendType::Anthropic));
        assert_eq!(url, Some("https://custom.anthropic.com".to_string()));

        // Test @backend format (default model)
        let (model, backend, url) = parse_model_attings("@anthropic");
        assert_eq!(model, "claude-3-5-sonnet-20241022");
        assert_eq!(backend, Some(BackendType::Anthropic));
        assert_eq!(url, Some(ANTHROPIC_API_URL.to_string()));

        // Test @groq format (default model for Groq)
        let (model, backend, url) = parse_model_attings("@groq");
        assert_eq!(model, "llama-3.1-8b-instant");
        assert_eq!(backend, Some(BackendType::Groq));
        assert_eq!(url, Some(GROQ_API_URL.to_string()));

        // Test @openai format (default model for OpenAI)
        let (model, backend, url) = parse_model_attings("@openai");
        assert_eq!(model, "gpt-4o-mini");
        assert_eq!(backend, Some(BackendType::OpenAI));
        assert_eq!(url, Some(OPENAI_API_URL.to_string()));

        // Test @llama format (default model for Llama)
        let (model, backend, url) = parse_model_attings("@llama");
        assert_eq!(model, "llama3.1");
        assert_eq!(backend, Some(BackendType::Llama));
        assert_eq!(url, None);

        // Test model only (no backend)
        let (model, backend, url) = parse_model_attings("custom-model");
        assert_eq!(model, "custom-model");
        assert_eq!(backend, None);
        assert_eq!(url, None);
    }
}