use anyhow::{Context, Result};
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

use clap::Parser;


mod logging;
mod open_file;
mod preview;
mod core;
mod policy;
mod tools;
mod agents;
mod models;
mod tools_execution;
mod cli;
mod config;
mod chat;
mod api;

use logging::{ConversationLogger, log_request, log_request_to_file, log_response, log_stream_chunk};
use core::{ToolRegistry, ToolParameters};
use core::tool_context::ToolContext;
use policy::PolicyManager;
use tools_execution::parse_xml_tool_calls;
use tools_execution::validation::{repair_tool_call_with_model, validate_and_fix_tool_calls_in_place};
use cli::{Cli, Commands};
use config::{ClientConfig, GROQ_API_URL, normalize_api_url, initialize_tool_registry, initialize_agent_system};
use chat::{save_state, load_state};
use chat::history::summarize_and_trim_history;
use chat::session::chat as chat_session;
use api::{call_api, call_api_streaming, call_api_with_llm_client, call_api_streaming_with_llm_client};
use agents::{
    PlanningCoordinator, GroqLlmClient,
    ChatMessage, ToolDefinition, ExecutionContext,
};
use models::{
    ModelType, Message, ToolCall, FunctionCall,
    SwitchModelArgs,
    ChatRequest, Tool, FunctionDef,
    ChatResponse, Usage,
    StreamChunk,
};


pub(crate) const MAX_CONTEXT_TOKENS: usize = 100_000; // Keep conversation under this to avoid rate limits
pub(crate) const MAX_RETRIES: u32 = 3;

pub(crate) struct KimiChat {
    pub(crate) api_key: String,
    pub(crate) work_dir: PathBuf,
    pub(crate) client: reqwest::Client,
    pub(crate) messages: Vec<Message>,
    pub(crate) current_model: ModelType,
    pub(crate) total_tokens_used: usize,
    pub(crate) logger: Option<ConversationLogger>,
    pub(crate) tool_registry: ToolRegistry,
    // Agent system
    pub(crate) agent_coordinator: Option<PlanningCoordinator>,
    pub(crate) use_agents: bool,
    // Client configuration
    pub(crate) client_config: ClientConfig,
    // Policy manager
    pub(crate) policy_manager: PolicyManager,
    // Streaming mode
    pub(crate) stream_responses: bool,
    // Verbose debug mode
    pub(crate) verbose: bool,
    // Debug level for controlling debug output (0=off, 1=basic, 2=detailed, etc.)
    pub(crate) debug_level: u32,
}

impl KimiChat {
    /// Normalize API URL by ensuring it has the correct path for OpenAI-compatible endpoints
    pub(crate) fn normalize_api_url(url: &str) -> String {
        normalize_api_url(url)
    }

    /// Generate system prompt based on current model
    pub(crate) fn get_system_prompt() -> String {
        "You are an AI assistant with access to file operations and model switching capabilities. \
        The system supports multiple models that can be switched during the conversation:\n\
        - grn_model (GrnModel): **Preferred for cost efficiency** - significantly cheaper than BluModel while providing good performance for most tasks\n\
        - blu_model (BluModel): Use when GrnModel struggles or when you need faster responses\n\n\
        IMPORTANT: You have been provided with a set of tools (functions) that you can use. \
        Only use the tools that are provided to you - do not make up tool names or attempt to use tools that are not available. \
        When making multiple file edits, use plan_edits to create a complete plan, then apply_edit_plan to execute all changes atomically. \
        This prevents issues where you lose track of file state between sequential edits.\n\n\
        Model switches may happen automatically during the conversation based on tool usage and errors. \
        The currently active model will be indicated in system messages as the conversation progresses.".to_string()
    }

    /// Get the API URL to use based on the current model and client configuration
    pub(crate) fn get_api_url(&self, model: &ModelType) -> String {
        let url = match model {
            ModelType::BluModel => {
                self.client_config.api_url_blu_model
                    .as_ref()
                    .map(|s| s.clone())
                    .unwrap_or_else(|| GROQ_API_URL.to_string())
            }
            ModelType::GrnModel => {
                self.client_config.api_url_grn_model
                    .as_ref()
                    .map(|s| s.clone())
                    .unwrap_or_else(|| GROQ_API_URL.to_string())
            }
            ModelType::AnthropicModel => {
                // For Anthropic, default to the official API or look for Anthropic-specific URLs
                env::var("ANTHROPIC_BASE_URL")
                    .or_else(|_| env::var("ANTHROPIC_BASE_URL_BLU"))
                    .or_else(|_| env::var("ANTHROPIC_BASE_URL_GRN"))
                    .unwrap_or_else(|_| "https://api.anthropic.com".to_string())
            }
            ModelType::Custom(_) => {
                // For custom models, default to the first available override or Groq
                self.client_config.api_url_blu_model
                    .as_ref()
                    .or(self.client_config.api_url_grn_model.as_ref())
                    .map(|s| s.clone())
                    .unwrap_or_else(|| GROQ_API_URL.to_string())
            }
        };

        // Normalize the URL to ensure it has the correct path
        Self::normalize_api_url(&url)
    }

    /// Get the appropriate API key for a given model based on configuration
    pub(crate) fn get_api_key(&self, model: &ModelType) -> String {
        match model {
            ModelType::BluModel => {
                self.client_config.api_key_blu_model
                    .as_ref()
                    .map(|s| s.clone())
                    .unwrap_or_else(|| self.api_key.clone())
            }
            ModelType::GrnModel => {
                self.client_config.api_key_grn_model
                    .as_ref()
                    .map(|s| s.clone())
                    .unwrap_or_else(|| self.api_key.clone())
            }
            ModelType::AnthropicModel => {
                // For Anthropic, look for Anthropic-specific keys first
                env::var("ANTHROPIC_API_KEY")
                    .or_else(|_| env::var("ANTHROPIC_AUTH_TOKEN"))
                    .or_else(|_| env::var("ANTHROPIC_AUTH_TOKEN_BLU"))
                    .or_else(|_| env::var("ANTHROPIC_AUTH_TOKEN_GRN"))
                    .unwrap_or_else(|_| self.api_key.clone())
            }
            ModelType::Custom(_) => {
                // For custom models, default to the first available override or default key
                self.client_config.api_key_blu_model
                    .as_ref()
                    .or(self.client_config.api_key_grn_model.as_ref())
                    .map(|s| s.clone())
                    .unwrap_or_else(|| self.api_key.clone())
            }
        }
    }

    fn new(api_key: String, work_dir: PathBuf) -> Self {
        let config = ClientConfig {
            api_key: api_key.clone(),
            api_url_blu_model: None,
            api_url_grn_model: None,
            api_key_blu_model: None,
            api_key_grn_model: None,
            model_blu_model_override: None,
            model_grn_model_override: None,
        };
        let policy_manager = PolicyManager::new();
        Self::new_with_config(config, work_dir, false, policy_manager, false, false)
    }

    fn new_with_agents(api_key: String, work_dir: PathBuf, use_agents: bool) -> Self {
        let config = ClientConfig {
            api_key: api_key.clone(),
            api_url_blu_model: None,
            api_url_grn_model: None,
            api_key_blu_model: None,
            api_key_grn_model: None,
            model_blu_model_override: None,
            model_grn_model_override: None,
        };
        let policy_manager = PolicyManager::new();
        Self::new_with_config(config, work_dir, use_agents, policy_manager, false, false)
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

    fn new_with_config(client_config: ClientConfig, work_dir: PathBuf, use_agents: bool, policy_manager: PolicyManager, stream_responses: bool, verbose: bool) -> Self {
        let tool_registry = initialize_tool_registry();
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

        // Determine initial model based on overrides or defaults
        // Default to GPT-OSS for cost efficiency - it's significantly cheaper than Kimi
        // while still providing good performance for most tasks
        let initial_model = if let Some(ref override_model) = client_config.model_grn_model_override {
            ModelType::Custom(override_model.clone())
        } else {
            ModelType::GrnModel
        };

        let mut chat = Self {
            api_key: client_config.api_key.clone(),
            work_dir,
            client: reqwest::Client::new(),
            messages: Vec::new(),
            current_model: initial_model,
            total_tokens_used: 0,
            logger: None,
            tool_registry,
            agent_coordinator,
            use_agents,
            client_config,
            policy_manager,
            stream_responses,
            verbose,
            debug_level: 0, // Default debug level is 0 (off)
        };

        // Add system message to inform the model about capabilities
        let system_content = Self::get_system_prompt();

        chat.messages.push(Message {
            role: "system".to_string(),
            content: system_content,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        });

        // Add initial model notification
        chat.messages.push(Message {
            role: "system".to_string(),
            content: format!("Current model: {}", chat.current_model.display_name()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
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
    async fn process_with_agents(&mut self, user_request: &str) -> Result<String> {
        // Get API URL before mutable borrow
        let api_url = self.get_api_url(&self.current_model);
        let api_key = self.get_api_key(&self.current_model);

        if let Some(coordinator) = &mut self.agent_coordinator {
            // Create execution context for agents
            let tool_registry_arc = std::sync::Arc::new(self.tool_registry.clone());
            let llm_client = std::sync::Arc::new(GroqLlmClient::new(
                api_key,
                self.current_model.as_str().to_string(),
                api_url,
                "process_with_agents".to_string()
            ));

            // Convert message history to agent format
            let conversation_history: Vec<ChatMessage> = self.messages.iter().map(|msg| {
                ChatMessage {
                    role: msg.role.clone(),
                    content: msg.content.clone(),
                    tool_calls: msg.tool_calls.clone().map(|calls| {
                        calls.into_iter().map(|call| crate::agents::agent::ToolCall {
                            id: call.id,
                            function: crate::agents::agent::FunctionCall {
                                name: call.function.name,
                                arguments: call.function.arguments,
                            },
                        }).collect()
                    }),
                    tool_call_id: msg.tool_call_id.clone(),
                    name: msg.name.clone(),
                }
            }).collect();

            let context = ExecutionContext {
                workspace_dir: self.work_dir.clone(),
                session_id: format!("session_{}", chrono::Utc::now().timestamp()),
                tool_registry: tool_registry_arc,
                llm_client,
                conversation_history,
            };

            // Process request through coordinator
            let result = coordinator.process_user_request(user_request, &context).await?;

            // Update message history
            self.messages.push(Message {
                role: "user".to_string(),
                content: user_request.to_string(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            });

            self.messages.push(Message {
                role: "assistant".to_string(),
                content: result.content.clone(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
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
            "blu_model" | "blu-model" => ModelType::BluModel,
            "grn_model" | "grn-model" => ModelType::GrnModel,
            "anthropic" | "claude" | "anthropic_model" | "anthropic-model" => ModelType::AnthropicModel,
            _ => anyhow::bail!("Unknown model: {}. Available: 'blu_model', 'grn_model', 'anthropic'", model_str),
        };

        if new_model == self.current_model {
            return Ok(format!(
                "Already using {} model",
                self.current_model.display_name()
            ));
        }

        let old_model = self.current_model.clone();
        self.current_model = new_model.clone();

        // Add message to conversation history about model switch
        self.messages.push(Message {
            role: "system".to_string(),
            content: format!("Model switched to: {} (reason: {})", new_model.display_name(), reason),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        });

        Ok(format!(
            "Switched from {} to {} - Reason: {}",
            old_model.display_name(),
            new_model.display_name(),
            reason
        ))
    }

    fn save_state(&self, file_path: &str) -> Result<String> {
        save_state(&self.messages, &self.current_model, self.total_tokens_used, file_path)
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

                let context = ToolContext::new(
                    self.work_dir.clone(),
                    format!("session_{}", chrono::Utc::now().timestamp()),
                    self.policy_manager.clone()
                );

                let result = self.tool_registry.execute_tool(name, params, &context).await;

                if result.success {
                    Ok(result.content)
                } else {
                    Err(anyhow::anyhow!("Tool '{}' failed: {}", name, result.error.unwrap_or_else(|| "Unknown error".to_string())))
                }
            }
        }
    }

    async fn summarize_and_trim_history(&mut self) -> Result<()> {
        summarize_and_trim_history(self).await
    }

    /// Attempt to repair malformed tool calls using a separate API call to a model
    async fn repair_tool_call_with_model(&self, tool_call: &ToolCall, error_msg: &str) -> Result<ToolCall> {
        repair_tool_call_with_model(self, tool_call, error_msg).await
    }

    fn validate_and_fix_tool_calls_in_place(&mut self) -> Result<bool> {
        validate_and_fix_tool_calls_in_place(self)
    }
    async fn call_api_streaming(&self, orig_messages: &[Message]) -> Result<(Message, Option<Usage>, ModelType)> {
        call_api_streaming(self, orig_messages).await
    }

    async fn call_api(&self, orig_messages: &[Message]) -> Result<(Message, Option<Usage>, ModelType)> {
        call_api(self, orig_messages).await
    }

    async fn call_api_with_llm_client(&self, messages: &[Message], model: &ModelType) -> Result<(Message, Option<Usage>, ModelType)> {
        call_api_with_llm_client(self, messages, model).await
    }

    async fn call_api_streaming_with_llm_client(&self, messages: &[Message], model: &ModelType) -> Result<(Message, Option<Usage>, ModelType)> {
        call_api_streaming_with_llm_client(self, messages, model).await
    }
    async fn chat(&mut self, user_message: &str) -> Result<String> {
        chat_session(self, user_message).await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file if it exists
    dotenvy::dotenv().ok();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Determine API URLs for each model
    // Priority: specific flags (--api-url-blu-model, --api-url-grn-model) override general flag (--llama-cpp-url)
    // Also check for Anthropic environment variables
    let api_url_blu_model = cli.api_url_blu_model
        .or_else(|| cli.llama_cpp_url.clone())
        .or_else(|| env::var("ANTHROPIC_BASE_URL_BLU").ok())
        .or_else(|| env::var("ANTHROPIC_BASE_URL").ok());

    let api_url_grn_model = cli.api_url_grn_model
        .or_else(|| cli.llama_cpp_url.clone())
        .or_else(|| env::var("ANTHROPIC_BASE_URL_GRN").ok())
        .or_else(|| env::var("ANTHROPIC_BASE_URL").ok());

    // Check for per-model API keys (for Anthropic or other services)
    let api_key_blu_model = env::var("ANTHROPIC_AUTH_TOKEN_BLU").ok()
        .or_else(|| env::var("ANTHROPIC_AUTH_TOKEN").ok());

    let api_key_grn_model = env::var("ANTHROPIC_AUTH_TOKEN_GRN").ok()
        .or_else(|| env::var("ANTHROPIC_AUTH_TOKEN").ok());

    // Auto-detect Anthropic and set appropriate model names if not overridden
    let is_anthropic_blu = api_url_blu_model.as_ref()
        .map(|url| url.contains("anthropic"))
        .unwrap_or(false);
    let is_anthropic_grn = api_url_grn_model.as_ref()
        .map(|url| url.contains("anthropic"))
        .unwrap_or(false);

    let model_blu_override = cli.model_blu_model.clone()
        .or_else(|| cli.model.clone())
        .or_else(|| {
            if is_anthropic_blu {
                env::var("ANTHROPIC_MODEL_BLU").ok()
                    .or_else(|| env::var("ANTHROPIC_MODEL").ok())
                    .or(Some("claude-3-5-sonnet-20241022".to_string()))
            } else {
                None
            }
        });

    let model_grn_override = cli.model_grn_model.clone()
        .or_else(|| cli.model.clone())
        .or_else(|| {
            if is_anthropic_grn {
                env::var("ANTHROPIC_MODEL_GRN").ok()
                    .or_else(|| env::var("ANTHROPIC_MODEL").ok())
                    .or(Some("claude-3-5-sonnet-20241022".to_string()))
            } else {
                None
            }
        });

    // API key is only required if at least one model uses Groq (no API URL specified and no per-model key)
    let needs_groq_key = (api_url_blu_model.is_none() && api_key_blu_model.is_none())
                      || (api_url_grn_model.is_none() && api_key_grn_model.is_none());

    let api_key = if needs_groq_key {
        env::var("GROQ_API_KEY")
            .context("GROQ_API_KEY environment variable not set. Use --api-url-blu-model and/or --api-url-grn-model with ANTHROPIC_AUTH_TOKEN to use other backends.")?
    } else {
        // Using custom backends with per-model keys, no Groq key needed
        String::new()
    };

    // Use current directory as work_dir so the AI can see project files
    // NB: do NOT use the 'workspace' subdirectory as work_dir
    let work_dir = env::current_dir()?;

    // If a subcommand was provided, execute it and exit
    if let Some(command) = cli.command {
        // Special handling for Switch command which needs KimiChat
        let result = match &command {
            Commands::Switch { model, reason } => {
                let mut chat = KimiChat::new("".to_string(), work_dir.clone());
                chat.switch_model(model, reason)?
            }
            _ => command.execute().await?
        };
        println!("{}", result);
        return Ok(());
    }

    // Create client configuration from CLI arguments
    // Priority: specific flags override general --model flag, with auto-detection for Anthropic
    let client_config = ClientConfig {
        api_key: api_key.clone(),
        api_url_blu_model: api_url_blu_model.clone(),
        api_url_grn_model: api_url_grn_model.clone(),
        api_key_blu_model,
        api_key_grn_model,
        model_blu_model_override: model_blu_override.clone(),
        model_grn_model_override: model_grn_override.clone(),
    };

    // Inform user about auto-detected Anthropic configuration
    if is_anthropic_blu {
        let model_name = model_blu_override.as_ref().unwrap();
        eprintln!("{} Anthropic detected for blu_model: using model '{}'", "🤖".cyan(), model_name);
    }
    if is_anthropic_grn {
        let model_name = model_grn_override.as_ref().unwrap();
        eprintln!("{} Anthropic detected for grn_model: using model '{}'", "🤖".cyan(), model_name);
    }

    // Create policy manager based on CLI arguments
    let policy_manager = if cli.auto_confirm {
        eprintln!("{} Auto-confirm mode enabled - all actions will be approved automatically", "🚀".green());
        PolicyManager::allow_all()
    } else if cli.policy_file.is_some() || cli.learn_policies {
        let policy_file = cli.policy_file.unwrap_or_else(|| "policies.toml".to_string());
        let policy_path = work_dir.join(&policy_file);
        match PolicyManager::from_file(&policy_path, cli.learn_policies) {
            Ok(pm) => {
                eprintln!("{} Loaded policy file: {}", "📋".cyan(), policy_path.display());
                if cli.learn_policies {
                    eprintln!("{} Policy learning enabled - user decisions will be saved to policy file", "📚".cyan());
                }
                pm
            }
            Err(e) => {
                eprintln!("{} Failed to load policy file: {}", "⚠️".yellow(), e);
                eprintln!("{} Using default policy (ask for confirmation)", "📋".cyan());
                PolicyManager::new()
            }
        }
    } else {
        PolicyManager::new()
    };

    // Handle task mode if requested
    if let Some(task_text) = cli.task {
        println!("{}", "🤖 Kimi Chat - Task Mode".bright_cyan().bold());
        println!("{}", format!("Working directory: {}", work_dir.display()).bright_black());

        if cli.agents {
            println!("{}", "🚀 Multi-Agent System ENABLED".green().bold());
        }

        println!("{}", format!("Task: {}", task_text).bright_yellow());
        println!();

        let mut chat = KimiChat::new_with_config(client_config.clone(), work_dir.clone(), cli.agents, policy_manager.clone(), cli.stream, cli.verbose);

        // Initialize logger for task mode
        chat.logger = match ConversationLogger::new_task_mode(&chat.work_dir).await {
            Ok(l) => Some(l),
            Err(e) => {
                eprintln!("Task logging disabled: {}", e);
                None
            }
        };

        let response = if chat.use_agents && chat.agent_coordinator.is_some() {
            // Use agent system
            match chat.process_with_agents(&task_text).await {
                Ok(response) => response,
                Err(e) => {
                    eprintln!("{} {}\n", "Agent Error:".bright_red().bold(), e);
                    // Fallback to regular chat
                    match chat.chat(&task_text).await {
                        Ok(response) => response,
                        Err(e) => {
                            eprintln!("{} {}\n", "Error:".bright_red().bold(), e);
                            return Ok(());
                        }
                    }
                }
            }
        } else {
            // Use regular chat
            match chat.chat(&task_text).await {
                Ok(response) => response,
                Err(e) => {
                    eprintln!("{} {}\n", "Error:".bright_red().bold(), e);
                    return Ok(());
                }
            }
        };

        if cli.pretty {
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "response": response,
                "agents_used": chat.use_agents
            })).unwrap_or_else(|_| response.to_string()));
        } else {
            println!("{}", response);
        }

        return Ok(());
    }

    // If interactive flag is set (or default), proceed to REPL
    if !cli.interactive {
        // If not interactive and no subcommand, just exit
        println!("No subcommand provided and interactive mode not requested. Exiting.");
        return Ok(());
    }

    println!("{}", "🤖 Kimi Chat - Claude Code-like Experience".bright_cyan().bold());
    println!("{}", format!("Working directory: {}", work_dir.display()).bright_black());

    if cli.agents {
        println!("{}", "🚀 Multi-Agent System ENABLED - Specialized agents will handle your tasks".green().bold());
    }

    println!("{}", "Type 'exit' or 'quit' to exit\n".bright_black());

    let mut chat = KimiChat::new_with_config(client_config, work_dir, cli.agents, policy_manager, cli.stream, cli.verbose);

    // Show the actual current model configuration
    let current_model_display = match chat.current_model {
        ModelType::BluModel => format!("BluModel/{} (auto-switched from default)", chat.current_model.display_name()),
        ModelType::GrnModel => format!("GrnModel/{} (default)", chat.current_model.display_name()),
        ModelType::AnthropicModel => format!("Anthropic/{}", chat.current_model.display_name()),
        ModelType::Custom(ref name) => format!("Custom/{}", name),
    };

    // Show what backends are being used
    let blu_backend = if chat.client_config.api_url_blu_model.as_ref().map(|u| u.contains("anthropic")).unwrap_or(false) ||
                       env::var("ANTHROPIC_AUTH_TOKEN_BLU").is_ok() {
        "Anthropic API 🧠"
    } else if chat.client_config.api_url_blu_model.is_some() {
        "llama.cpp 🦙"
    } else {
        "Groq API 🚀"
    };

    let grn_backend = if chat.client_config.api_url_grn_model.as_ref().map(|u| u.contains("anthropic")).unwrap_or(false) ||
                       env::var("ANTHROPIC_AUTH_TOKEN_GRN").is_ok() {
        "Anthropic API 🧠"
    } else if chat.client_config.api_url_grn_model.is_some() {
        "llama.cpp 🦙"
    } else {
        "Groq API 🚀"
    };

    println!("{}", format!("Default model: {} • BluModel uses {}, GrnModel uses {}",
        current_model_display, blu_backend, grn_backend).bright_black());

    // Debug info (shown at debug level 1+)
    if chat.should_show_debug(1) {
        println!("{}", format!("🔧 DEBUG: blu_model URL: {:?}", chat.client_config.api_url_blu_model).bright_black());
        println!("{}", format!("🔧 DEBUG: grn_model URL: {:?}", chat.client_config.api_url_grn_model).bright_black());
        println!("{}", format!("🔧 DEBUG: Current model: {:?}", chat.current_model).bright_black());
    }

    // Initialize logger (async) – logs go into the workspace directory
    chat.logger = match ConversationLogger::new(&chat.work_dir).await {
        Ok(l) => Some(l),
        Err(e) => {
            eprintln!("Logging disabled: {}", e);
            None
        }
    };

    // If logger was created, log the initial system message that KimiChat::new added
    if let Some(logger) = &mut chat.logger {
        // The first message in chat.messages is the system prompt
        if let Some(sys_msg) = chat.messages.first() {
            logger
                .log(
                    "system",
                    &sys_msg.content,
                    None,
                    false,
                )
                .await;
        }
    }

    let mut rl = DefaultEditor::new()?;

    // Read kimi.md if it exists to get project context
    let kimi_context = if let Ok(kimi_content) = chat.read_file("kimi.md") {
        println!("{} {}", "📖".bright_cyan(), "Reading project context from kimi.md...".bright_black());
        kimi_content
    } else {
        println!("{} {}", "📖".bright_cyan(), "No kimi.md found. Starting fresh.".bright_black());
        String::new()
    };

    if !kimi_context.is_empty() {
        let sys_msg = Message {
            role: "system".to_string(),
            content: format!("Project context: {}", kimi_context),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        };
        // Log this system addition
        if let Some(logger) = &mut chat.logger {
            logger
                .log("system", &sys_msg.content, None, false)
                .await;
        }
        chat.messages.push(sys_msg);
    }

    loop {
        let model_indicator = format!("[{}]", chat.current_model.display_name()).bright_magenta();
        let readline = rl.readline(&format!("{} {} ", model_indicator, "You:".bright_green().bold()));

        match readline {
            Ok(line) => {
                let line = line.trim();

                if line.is_empty() {
                    continue;
                }

                if line == "exit" || line == "quit" {
                    println!("{}", "Goodbye!".bright_cyan());
                    break;
                }

                // Handle /save and /load commands
                if line.starts_with("/save ") {
                    let file_path = line[6..].trim();
                    match chat.save_state(file_path) {
                        Ok(msg) => println!("{} {}", "💾".bright_green(), msg),
                        Err(e) => eprintln!("{} Failed to save: {}", "❌".bright_red(), e),
                    }
                    continue;
                }

                if line.starts_with("/load ") {
                    let file_path = line[6..].trim();
                    match chat.load_state(file_path) {
                        Ok(msg) => println!("{} {}", "📂".bright_green(), msg),
                        Err(e) => eprintln!("{} Failed to load: {}", "❌".bright_red(), e),
                    }
                    continue;
                }

                // Handle /debug command
                if line == "/debug" {
                    println!("{} Debug level: {} (binary: {:b})", "🔧".bright_cyan(), chat.get_debug_level(), chat.get_debug_level());
                    println!("{} Usage: /debug <level>", "💡".bright_yellow());
                    println!("  0 = off");
                    println!("  1 = basic (bit 0)");
                    println!("  2 = detailed (bit 1)");
                    println!("  4 = verbose (bit 2)");
                    println!("  Example: /debug 3 (enables basic + detailed)");
                    continue;
                }

                if line.starts_with("/debug ") {
                    let level_str = line[7..].trim();
                    match level_str.parse::<u32>() {
                        Ok(level) => {
                            chat.set_debug_level(level);
                            println!("{} Debug level set to {} (binary: {:b})", "🔧".bright_green(), level, level);
                        }
                        Err(_) => {
                            eprintln!("{} Invalid debug level: '{}'. Use a number like 0, 1, 3, 7, etc.", "❌".bright_red(), level_str);
                        }
                    }
                    continue;
                }

                rl.add_history_entry(line)?;

                // Log the user message before sending
                if let Some(logger) = &mut chat.logger {
                    logger.log("user", line, None, false).await;
                }

                let response = if chat.use_agents && chat.agent_coordinator.is_some() {
                    // Use agent system
                    match chat.process_with_agents(line).await {
                        Ok(response) => response,
                        Err(e) => {
                            eprintln!("{} {}\n", "Agent Error:".bright_red().bold(), e);
                            // Fallback to regular chat
                            match chat.chat(line).await {
                                Ok(response) => response,
                                Err(e) => {
                                    eprintln!("{} {}\n", "Error:".bright_red().bold(), e);
                                    continue;
                                }
                            }
                        }
                    }
                } else {
                    // Use regular chat
                    match chat.chat(line).await {
                        Ok(response) => response,
                        Err(e) => {
                            eprintln!("{} {}\n", "Error:".bright_red().bold(), e);
                            continue;
                        }
                    }
                };

                // Log assistant response
                if let Some(logger) = &mut chat.logger {
                    logger.log("assistant", &response, None, false).await;
                }

                // Display response if not streaming (streaming already displayed it)
                if !chat.stream_responses {
                    let model_label = format!("[{}]", chat.current_model.display_name()).bright_magenta();
                    let assistant_label = "Assistant:".bright_blue().bold();
                    println!("\n{} {} {}\n", model_label, assistant_label, response);
                } else {
                    // Add extra newline after streaming to separate from next prompt
                    println!();
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("{}", "^C".bright_black());
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("{}", "Goodbye!".bright_cyan());
                break;
            }
            Err(err) => {
                eprintln!("{} {}", "Error:".bright_red().bold(), err);
                break;
            }
        }
    }

    // Graceful shutdown of logger (flush & close)
    if let Some(logger) = &mut chat.logger {
        logger.shutdown().await;
    }

    Ok(())
}
