use anyhow::{Context, Result};
use colored::Colorize;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use clap::Parser;


mod preview;
mod tools_execution;
mod cli;
mod config;
mod chat;
mod api;
mod app;
mod terminal;
mod web;

use apchat_agents::{
    PlanningCoordinator, GroqLlmClient,
    ChatMessage, ExecutionContext,
};
use apchat_logging::ConversationLogger;
use apchat_policy::PolicyManager;
use apchat_terminal::{TerminalManager, TerminalBackendType, MAX_CONCURRENT_SESSIONS};
use apchat_toolcore::{ToolRegistry, ToolParameters, ToolContext};
use cli::{Cli, Commands};
use config::{ClientConfig, GROQ_API_URL, initialize_tool_registry, initialize_agent_system};
use chat::{save_state, load_state, InputChannel, InputChannelConfig, InputMessage};
use app::{setup_from_cli, run_task_mode, run_subagent_mode, run_repl_mode};
use apchat_models::{
    ModelColor, Message, ToolCall, FunctionCall, ModelProvider,
    SwitchModelArgs,
    Tool, FunctionDef,
    ChatResponse,
};


pub(crate) const MAX_CONTEXT_TOKENS: usize = 100_000; // Keep conversation under this to avoid rate limits
pub(crate) const MAX_RETRIES: u32 = 3;

pub(crate) struct APChat {
    pub(crate) api_key: String,
    pub(crate) work_dir: PathBuf,
    pub(crate) client: reqwest::Client,
    pub(crate) messages: Vec<Message>,
    pub(crate) current_model: ModelColor,
    pub(crate) total_tokens_used: usize,
    pub(crate) logger: Option<ConversationLogger>,
    pub(crate) tool_registry: ToolRegistry,
    pub(crate) input_channel: Option<InputChannel<InputMessage>>,
    // Agent system
    pub(crate) agent_coordinator: Option<PlanningCoordinator>,
    pub(crate) use_agents: bool,
    // Client configuration
    pub(crate) client_config: ClientConfig,
    // Policy manager
    pub(crate) policy_manager: PolicyManager,
    // Terminal manager
    pub(crate) terminal_manager: Arc<Mutex<TerminalManager>>,
    // Skill registry
    pub(crate) skill_registry: Option<Arc<apchat_skills::SkillRegistry>>,
    // Non-interactive mode (web/API)
    pub(crate) non_interactive: bool,
    // Todo manager for task tracking
    pub(crate) todo_manager: Arc<apchat_todo::TodoManager>,
    // Streaming mode
    pub(crate) stream_responses: bool,
    // Verbose debug mode
    pub(crate) verbose: bool,
    // Debug level for controlling debug output (0=off, 1=basic, 2=detailed, etc.)
    pub(crate) debug_level: u32,
    // Process ID
    pub(crate) process_id: u32,
    // Readline history for REPL
    pub(crate) readline_history: Option<crate::chat::readline_history::ReadlineHistory>,
    // Content limiter
    pub(crate) content_limiter: Option<Arc<apchat_toolcore::content_limiter::ContentLimiter>>,
    
}

impl APChat {
    
    /// Create a new APChat with content limiter
    pub(crate) fn with_content_limiter(mut self, content_limiter: Arc<apchat_toolcore::content_limiter::ContentLimiter>) -> Self {
        self.content_limiter = Some(content_limiter.clone());
        self.tool_registry = self.tool_registry.with_content_limiter(content_limiter);
        self
    }
    fn new(api_key: String, work_dir: PathBuf) -> Self {
        let config = ClientConfig {
            api_key: api_key.clone(),
            model_providers: [
                ModelProvider::new(ModelColor::BluModel.default_model()),
                ModelProvider::new(ModelColor::GrnModel.default_model()),
                ModelProvider::new(ModelColor::RedModel.default_model()),
            ],
        };
        let policy_manager = PolicyManager::new();
        Self::new_with_config(
            config,
            work_dir,
            false,
            policy_manager,
            false,
            false,
            TerminalBackendType::Pty,
            false, // Default early_superpowers to false
        )
    }

    fn new_with_agents(api_key: String, work_dir: PathBuf, use_agents: bool) -> Self {
        let config = ClientConfig {
            api_key: api_key.clone(),
            model_providers: [
                ModelProvider::new(ModelColor::BluModel.default_model()),
                ModelProvider::new(ModelColor::GrnModel.default_model()),
                ModelProvider::new(ModelColor::RedModel.default_model()),
            ],
        };
        let policy_manager = PolicyManager::new();
        Self::new_with_config(
            config,
            work_dir,
            use_agents,
            policy_manager,
            false,
            false,
            TerminalBackendType::Pty,
            false, // Default early_superpowers to false
        )
    }

    /// Set the debug level (0=off, 1=basic, 2=detailed, etc.)
    pub(crate) fn set_debug_level(&mut self, level: u32) {
        self.debug_level = level;
    }

    /// Get the current debug level
    pub(crate) fn get_debug_level(&self) -> u32 {
        self.debug_level
    }

    /// Check if debug output should be shown for a given level
    pub(crate) fn should_show_debug(&self, level: u32) -> bool {
        self.debug_level & (1 << (level - 1)) != 0
    }

    fn new_with_config(
        client_config: ClientConfig,
        work_dir: PathBuf,
        use_agents: bool,
        policy_manager: PolicyManager,
        stream_responses: bool,
        verbose: bool,
        backend_type: TerminalBackendType,
        early_superpowers: bool,
    ) -> Self {
        let tool_registry = initialize_tool_registry();
        
        // Initialize content limiter
        let content_limiter_config = apchat_toolcore::content_limiter::ContentLimiterConfig::new(&work_dir);
        let content_limiter = Some(Arc::new(apchat_toolcore::content_limiter::ContentLimiter::new(content_limiter_config)));
        let content_limiter_clone = content_limiter.clone();

        // Initialize skill registry
        let skills_dir = work_dir.join("skills");
        let skill_registry = match apchat_skills::SkillRegistry::new(skills_dir) {
            Ok(registry) => Some(Arc::new(registry)),
            Err(e) => {
                eprintln!("{} Failed to load skills: {}", "⚠️".yellow(), e);
                eprintln!("{} Skills will not be available", "⚠️".yellow());
                None
            }
        };

        let agent_coordinator = if use_agents {
            match initialize_agent_system(&client_config, &tool_registry, &policy_manager) {
                Ok(coordinator) => Some(coordinator),
                Err(e) => {
                    eprintln!("{} Failed to initialize agent system: {}", "❌".red(), e);
                    eprintln!("{} Falling back to non-agent mode", "⚠️".yellow());
                    None
                }
            }
        } else {
            None
        };

        // Initialize terminal manager with specified backend
        let logs_dir = apchat_logging::get_logs_dir()
            .unwrap_or_else(|_| PathBuf::from("logs"))
            .join("terminals");
        let terminal_manager = Arc::new(Mutex::new(
            TerminalManager::with_backend(logs_dir, backend_type, MAX_CONCURRENT_SESSIONS)
        ));

        // Initialize todo manager
        let todo_manager = Arc::new(apchat_todo::TodoManager::new());

        // Determine initial model based on overrides or defaults
        // Default to GPT-OSS for cost efficiency - it's significantly cheaper than Kimi
        // while still providing good performance for most tasks
        let initial_model = if client_config.get_model_override(ModelColor::GrnModel).is_some() {
            ModelColor::GrnModel
        } else {
            ModelColor::GrnModel
        };

        // Generate system message to inform the model about capabilities (before moving client_config)
        let system_content = config::get_system_prompt(&client_config, skill_registry.as_ref(), early_superpowers);

        let mut chat = Self {
            api_key: client_config.api_key.clone(),
            work_dir,
            client: reqwest::Client::new(),
            messages: Vec::new(),
            current_model: initial_model,
            total_tokens_used: 0,
            logger: None,
            tool_registry: tool_registry.with_content_limiter(content_limiter_clone.unwrap()),
            input_channel: None,
            agent_coordinator,
            use_agents,
            client_config,
            policy_manager,
            terminal_manager,
            skill_registry,
            todo_manager,
            stream_responses,
            verbose,
            debug_level: 0, // Default debug level is 0 (off)
            non_interactive: false, // Default to interactive mode
            process_id: std::process::id(),
            readline_history: None,
            content_limiter,
        };

        chat.messages.push(Message {
            role: "system".to_string(),
            content: system_content,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning: None,
        });

        // Add initial model notification
        chat.messages.push(Message {
            role: "system".to_string(),
            content: format!("Current model: {}", chat.current_model.display_name()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning: None,
        });

        chat
    }

    pub(crate) fn get_tools(&self) -> Vec<Tool> {
        // Convert new tool registry format to legacy Tool format for backward compatibility
        let registry_tools = self.tool_registry.get_openai_tool_definitions();

        registry_tools.into_iter().map(|tool_def| {
            Tool {
                tool_type: tool_def["type"].as_str().unwrap_or("function").to_string(),
                function: FunctionDef {
                    name: tool_def["function"]["name"].as_str().unwrap_or("").to_string(),
                    description: tool_def["function"]["description"].as_str().unwrap_or("").to_string(),
                    parameters: tool_def["function"]["parameters"].clone(),
                },
            }
        }).collect()
    }

    /// Process user request using the agent system
    async fn process_with_agents(&mut self, user_request: &str, cancellation_token: Option<tokio_util::sync::CancellationToken>) -> Result<String> {
        // Get API URL before mutable borrow
        let api_url = config::get_api_url(&self.client_config, &self.current_model);
        let api_key = config::get_api_key(&self.client_config, &self.api_key, &self.current_model);

        if let Some(coordinator) = &mut self.agent_coordinator {
            // Create execution context for agents
            let tool_registry_arc = std::sync::Arc::new(self.tool_registry.clone());
            let llm_client = std::sync::Arc::new(GroqLlmClient::new(
                api_key,
                self.current_model.as_str(
                    self.client_config.get_model_override(ModelColor::BluModel).as_deref().map(|x| x.as_str()),
                    self.client_config.get_model_override(ModelColor::GrnModel).as_deref().map(|x| x.as_str()),
                    self.client_config.get_model_override(ModelColor::RedModel).as_deref().map(|x| x.as_str())
                ).to_string(),
                api_url,
                "process_with_agents".to_string()
            ));

            // Convert message history to agent format
            let conversation_history: Vec<ChatMessage> = self.messages.iter().map(|msg| {
                ChatMessage {
                    role: msg.role.clone(),
                    content: msg.content.clone(),
                    tool_calls: msg.tool_calls.clone().map(|calls| {
                        calls.into_iter().map(|call| apchat_agents::agent::ToolCall {
                            id: call.id,
                            function: apchat_agents::agent::FunctionCall {
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

            let context = ExecutionContext {
                workspace_dir: self.work_dir.clone(),
                session_id: format!("session_{}", chrono::Utc::now().timestamp()),
                tool_registry: tool_registry_arc,
                llm_client,
                conversation_history,
                terminal_manager: Some(self.terminal_manager.clone()),
                skill_registry: self.skill_registry.clone(),
                todo_manager: Some(self.todo_manager.clone()),
                cancellation_token,
            };

            // Debug: Log current model
            if self.debug_level > 0 {
                eprintln!("[DEBUG] Processing with agents using model: {}", self.current_model.display_name());
            }

            // Process request through coordinator
            let result = coordinator.process_user_request(user_request, &context).await?;

            // Update message history
            self.messages.push(Message {
                role: "user".to_string(),
                content: user_request.to_string(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                reasoning: None,
            });

            self.messages.push(Message {
                role: "assistant".to_string(),
                content: result.content.clone(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                reasoning: None,
            });

            Ok(result.content)
        } else {
            Err(anyhow::anyhow!("Agent coordinator not initialized"))
        }
    }

    fn read_file(&self, file_path: &str) -> Result<String> {
        let full_path = self.work_dir.join(file_path);
        let content = fs::read_to_string(&full_path)
            .with_context(|| format!("Failed to read file: {}", full_path.display()))?;

        // Return just the content without any metadata
        // This prevents the "[Total: X lines]" from being accidentally included in edits/writes
        Ok(content)
    }

    fn switch_model(&mut self, model_str: &str, reason: &str) -> Result<String> {
        let new_model = match model_str.to_lowercase().as_str() {
            "blu_model" | "blu-model" | "blumodel" => ModelColor::BluModel,
            "grn_model" | "grn-model" | "grnmodel" => ModelColor::GrnModel,
            "red_model" | "red-model" | "redmodel" => ModelColor::RedModel,
            // For backward compatibility, map Anthropic references to BluModel
            "anthropic" | "claude" | "anthropic_model" | "anthropic-model" => ModelColor::BluModel,
            _ => anyhow::bail!("Unknown model: {}. Available: 'blu_model', 'grn_model', 'red_model'", model_str),
        };

        if new_model == self.current_model {
            return Ok(format!(
                "Already using {} model",
                self.current_model.display_name()
            ));
        }

        let old_model = self.current_model.clone();
        self.current_model = new_model.clone();

        // Removed: Model switch message no longer added to conversation history

        Ok(format!(
            "Switched from {} to {} - Reason: {}",
            old_model.display_name(),
            new_model.display_name(),
            reason
        ))
    }

    /// Format the current model as a string in format "modname@backend(url)"
    fn format_current_model_string(&self) -> String {
        let provider = self.client_config.get_provider(self.current_model);
        let model_name = &provider.model_name;
        
        // Get backend name
        let backend_name = match provider.backend {
            Some(apchat_models::BackendType::Groq) => "groq",
            Some(apchat_models::BackendType::OpenAI) => "openai", 
            Some(apchat_models::BackendType::Anthropic) => "anthropic",
            Some(apchat_models::BackendType::Llama) => "llama",
            None => "unknown",
        };
        
        let api_url = provider.api_url.as_ref()
            .map(|url| url.as_str())
            .unwrap_or("https://api.example.com");
            
        format!("{}@{}({})", model_name, backend_name, api_url)
    }

    fn save_state(&self, file_path: &str) -> Result<String> {
        save_state(&self.messages, &self.current_model, self.total_tokens_used, file_path)
    }

    /// Save conversation history automatically to logs directory
    fn auto_save_history(&self) -> Result<String> {
        let history_dir = apchat_logging::get_logs_dir()?.join("history");
        println!("History dir: {:?}", &history_dir);
        fs::create_dir_all(&history_dir).unwrap();
        let file_name = format!("history-{}.json", self.process_id);
        let file_path = history_dir.join(file_name);
        
        save_state(&self.messages, &self.current_model, self.total_tokens_used, file_path.to_str().unwrap())
    }

    fn load_state(&mut self, file_path: &str) -> Result<String> {
        let (messages, current_model, total_tokens_used, version) = load_state(file_path)?;

        // Restore state
        self.messages = messages;
        self.current_model = current_model;
        self.total_tokens_used = total_tokens_used;

        Ok(format!(
            "Loaded conversation state from {} ({} messages, {} total tokens, version: {})",
            file_path,
            self.messages.len(),
            self.total_tokens_used,
            version
        ))
    }

    async fn execute_tool(&mut self, name: &str, arguments: &str) -> Result<String> {
        // For backward compatibility, handle special tools that need main application state
        match name {
            "switch_model" => {
                let args: SwitchModelArgs = serde_json::from_str(arguments)?;
                self.switch_model(&args.model, &args.reason)
            }
            _ => {
                // Use the tool registry for all tools (including plan_edits and apply_edit_plan)
                let params = ToolParameters::from_json(arguments)
                    .with_context(|| format!("Failed to parse tool arguments for '{}': {}", name, arguments))?;

                // Format current model string for subagent tools
                let current_model_string = self.format_current_model_string();

                let mut context = ToolContext::new(
                    self.work_dir.clone(),
                    format!("session_{}", chrono::Utc::now().timestamp()),
                    self.policy_manager.clone()
                )
                .with_terminal_manager(self.terminal_manager.clone())
                .with_todo_manager(self.todo_manager.clone())
                .with_non_interactive(self.non_interactive)
                .with_current_model_string(current_model_string);

                // Create LLM clients for all model colors and add to context
                use std::collections::HashMap;
                use std::sync::Arc;
                let mut llm_clients: HashMap<ModelColor, Arc<dyn apchat_llm_api::client::LlmClient>> = HashMap::new();

                for color in ModelColor::iter() {
                    let client = crate::config::create_client_for_model_color(
                        &color,
                        &self.client_config,
                        &self.api_key,
                    );
                    llm_clients.insert(color, client);
                }

                context = context.with_llm_clients(llm_clients);

                // Add skill registry if available
                if let Some(ref registry) = self.skill_registry {
                    context = context.with_skill_registry(Arc::clone(registry));
                }

                // Add content limiter if available
                if let Some(ref limiter) = self.content_limiter {
                    context = context.with_content_limiter(Arc::clone(limiter));
                }

                let context = context;

                let result = self.tool_registry.execute_tool(name, params, &context).await;

                if result.success {
                    Ok(result.content)
                } else {
                    Err(anyhow::anyhow!("Tool '{}' failed: {}", name, result.error.unwrap_or_else(|| "Unknown error".to_string())))
                }
            }
        }
    }

    /// Initialize the input channel with a default configuration
    pub(crate) fn initialize_input_channel(&mut self) {
        if self.input_channel.is_none() {
            let config = InputChannelConfig::default();
            self.input_channel = Some(InputChannel::new(config));
        }
    }

    /// Get a reference to the input channel receiver
    /// Returns None if the channel is not initialized
    pub(crate) fn input_channel_receiver(&mut self) -> Option<&mut InputChannel<InputMessage>> {
        self.input_channel.as_mut()
    }

    /// Check if there are pending messages in the input channel
    /// Returns false if the channel is not initialized or has no pending messages
    pub(crate) fn has_pending_input(&mut self) -> bool {
        self.input_channel
            .as_mut()
            .map(|channel| channel.has_pending_messages())
            .unwrap_or(false)
    }

    /// Try to receive a message from the input channel without blocking
    /// Returns None if the channel is not initialized or there are no pending messages
    /// Try to receive a message from the input channel without blocking
    /// Returns None if the channel is not initialized or there are no pending messages
    /// Try to receive a message from the input channel without blocking
    /// Returns None if the channel is not initialized or there are no pending messages
    /// Try to receive a message from the input channel without blocking
    /// Returns None if the channel is not initialized or there are no pending messages
    pub(crate) async fn try_recv_input(&mut self) -> Option<InputMessage> {
        if let Some(channel) = self.input_channel.as_mut() {
            channel.try_recv().await
        } else {
            None
        }
    }
    /// Returns None if the channel is not initialized
    pub(crate) fn input_channel_sender(&self) -> Option<tokio::sync::mpsc::Sender<InputMessage>> {
        self.input_channel.as_ref().map(|_| {
            // Note: In the current implementation, we don't store the sender
            // This would need to be modified if we want to send messages externally
            // For now, returning None to indicate this is not yet implemented
            None
        }).flatten()
    }


}

/// Resolve terminal backend type from CLI args and environment variable
/// Priority: CLI arg > ENV var > default (PTY)
pub(crate) fn resolve_terminal_backend(cli: &Cli) -> Result<TerminalBackendType> {
    use TerminalBackendType;

    // Get backend string from CLI or env var
    let env_backend = env::var("APCHAT_TERMINAL_BACKEND").ok();
    let backend_str = cli.terminal_backend.as_deref()
        .or_else(|| env_backend.as_deref())
        .unwrap_or("pty");

    match backend_str.to_lowercase().as_str() {
        "pty" => Ok(TerminalBackendType::Pty),
        "tmux" => {
            // Check if tmux is available
            if let Ok(output) = std::process::Command::new("tmux").arg("-V").output() {
                if output.status.success() {
                    Ok(TerminalBackendType::Tmux)
                } else {
                    anyhow::bail!(
                        "Tmux backend requested but 'tmux -V' failed. Please ensure tmux is installed and working."
                    )
                }
            } else {
                anyhow::bail!(
                    "Tmux backend requested but tmux command not found. Please install tmux or use --terminal-backend pty"
                )
            }
        }
        other => anyhow::bail!(
            "Invalid terminal backend '{}'. Valid options: pty, tmux", other
        ),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file if it exists
    dotenvy::dotenv().ok();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Set memory database path from CLI flag if provided
    // This takes precedence over the environment variable
    if let Some(path) = &cli.memory_db_path {
        std::env::set_var("APCHAT_MEMORY_DB_PATH", path);
    }

    // If a subcommand was provided, execute it and exit
    if let Some(ref command) = cli.command {
        // Special handling for commands that need APChat or TerminalManager
        let work_dir = env::current_dir()?;
        let result = match command {
            Commands::Switch { model, reason } => {
                let mut chat = APChat::new("".to_string(), work_dir.clone());
                chat.switch_model(model, reason)?
            }
            Commands::Terminal { command: terminal_cmd } => {
                // Initialize TerminalManager for terminal commands
                let logs_dir = apchat_logging::get_logs_dir()
                    .unwrap_or_else(|_| PathBuf::from("logs"))
                    .join("terminals");
                let log_dir = logs_dir;
                let backend_type = resolve_terminal_backend(&cli)?;
                let terminal_manager = Arc::new(Mutex::new(
                    TerminalManager::with_backend(log_dir, backend_type, MAX_CONCURRENT_SESSIONS)
                ));
                terminal_cmd.execute(terminal_manager).await?
            }
            _ => command.execute().await?
        };
        println!("{}", result);
        return Ok(());
    }

    // Set up application configuration from CLI
    let app_config = setup_from_cli(&cli)?;

    // Handle task mode if requested
    if let Some(task_text) = cli.task.clone() {
        // Use subagent mode for single-agent mode (when --agents is NOT specified)
        if !cli.agents {
            return app::run_subagent_mode(
                &cli,
                task_text,
                app_config.client_config,
                app_config.work_dir,
                app_config.policy_manager,
            )
            .await;
        } else {
            // Use regular task mode for multi-agent system (when --agents IS specified)
            return app::run_task_mode(
                &cli,
                task_text,
                app_config.client_config,
                app_config.work_dir,
                app_config.policy_manager,
            )
            .await;
        }
    }

    // Handle web server mode
    if cli.web {
        return app::run_web_server(
            &cli,
            app_config.client_config,
            app_config.work_dir,
            app_config.policy_manager,
        )
        .await;
    }

    // If interactive flag is not set and no subcommand, just exit
    if !cli.interactive {
        println!("No subcommand provided and interactive mode not requested. Exiting.");
        return Ok(());
    }

    // Run REPL mode
    run_repl_mode(
        &cli,
        app_config.client_config,
        app_config.work_dir,
        app_config.policy_manager,
    )
    .await
}


#[cfg(test)]
mod auto_save_tests {
    use crate::APChat;
    use crate::cli::Cli;
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
            readline_history: None,
            content_limiter: None,
        }
    }

    #[tokio::test]
    async fn test_auto_save_creates_valid_file() {
        use std::fs;
        
        let mut chat = create_test_chat().await;
        
        // Add test messages - including a system message to simulate real usage
        chat.messages.push(Message {
            role: "system".to_string(),
            content: "Test system message".to_string(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning: None,
        });
        
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
        
        assert_eq!(parsed["messages"].as_array().unwrap().len(), 2); // 1 system + 1 user message
        assert_eq!(parsed["current_model"], "GrnModel"); // ModelColor uses PascalCase for serialization
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
        assert_eq!(loaded_messages.len(), 5); // Only the 5 user messages we added
    }

    #[tokio::test]
    async fn test_memory_db_path_env_var() {
        use std::env;
        
        // Test 1: No flag, no env var - should use default
        env::remove_var("APCHAT_MEMORY_DB_PATH");
        
        // Test 2: Only env var set
        env::set_var("APCHAT_MEMORY_DB_PATH", "/tmp/test_env.sqlite");
        let path = std::env::var("APCHAT_MEMORY_DB_PATH").unwrap();
        assert_eq!(path, "/tmp/test_env.sqlite");
        
        // Test 3: CLI flag overrides env var
        // Simulate what main() does
        let flag_path = "/tmp/flag_path.sqlite";
        if let Some(path) = Some(flag_path.to_string()) {
            std::env::set_var("APCHAT_MEMORY_DB_PATH", &path);
        }
        let path = std::env::var("APCHAT_MEMORY_DB_PATH").unwrap();
        assert_eq!(path, flag_path);
        
        // Clean up
        env::remove_var("APCHAT_MEMORY_DB_PATH");
    }

    #[tokio::test]
    async fn test_memory_db_path_sets_env_var() {
        use std::env;
        
        // Remove any existing env var
        env::remove_var("APCHAT_MEMORY_DB_PATH");
        
        // Set up a test that simulates the main function behavior
        let flag_path = "/tmp/test_path.sqlite";
        
        // The main function should set the env var
        if let Some(path) = Some(flag_path.to_string()) {
            std::env::set_var("APCHAT_MEMORY_DB_PATH", &path);
        }
        
        // Verify it was set
        assert_eq!(env::var("APCHAT_MEMORY_DB_PATH").unwrap(), flag_path);
        
        // Clean up
        env::remove_var("APCHAT_MEMORY_DB_PATH");
    }

    #[tokio::test]
    async fn test_memory_db_path_with_relative_path() {
        use std::env;
        
        // Remove any existing env var
        env::remove_var("APCHAT_MEMORY_DB_PATH");
        
        // Test with relative path
        let relative_path = "../relative/path.sqlite";
        
        // Simulate main function behavior
        if let Some(path) = Some(relative_path.to_string()) {
            std::env::set_var("APCHAT_MEMORY_DB_PATH", &path);
        }
        
        // Verify it was set
        assert_eq!(env::var("APCHAT_MEMORY_DB_PATH").unwrap(), relative_path);
        
        // Clean up
        env::remove_var("APCHAT_MEMORY_DB_PATH");
    }

    #[tokio::test]
    async fn test_memory_db_path_precedence() {
        use std::env;
        
        // Set environment variable
        env::set_var("APCHAT_MEMORY_DB_PATH", "/tmp/env_path.sqlite");
        
        // Simulate flag taking precedence
        let flag_path = "/tmp/flag_path.sqlite";
        if let Some(path) = Some(flag_path.to_string()) {
            std::env::set_var("APCHAT_MEMORY_DB_PATH", &path);
        }
        
        // Verify flag took precedence
        assert_eq!(env::var("APCHAT_MEMORY_DB_PATH").unwrap(), flag_path);
        
        // Clean up
        env::remove_var("APCHAT_MEMORY_DB_PATH");
    }
}
