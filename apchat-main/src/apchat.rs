// APChat struct and implementation
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use apchat_vty::{print_heart_yellow, print_heart_red};

use apchat_logging::ConversationLogger;
use apchat_policy::PolicyManager;
use apchat_terminal::{TerminalManager, TerminalBackendType, MAX_CONCURRENT_SESSIONS};
use apchat_toolcore::{ToolRegistry, ToolParameters, ToolContext, parameter_validation::validate_tool_call};
use crate::cli::Cli;
use crate::config::{ClientConfig, initialize_tool_registry};
use crate::chat::{save_state, load_state};
use apchat_models::{
    ModelColor, Message, ToolCall, FunctionCall, ModelProvider,
    SwitchModelArgs,
    Tool, FunctionDef,
};


pub const MAX_CONTEXT_TOKENS: usize = 100_000; // Keep conversation under this to avoid rate limits
pub const MAX_RETRIES: u32 = 3;

pub struct APChat {
    pub(crate) api_key: String,
    pub(crate) work_dir: PathBuf,
    pub(crate) client: reqwest::Client,
    pub(crate) messages: Vec<Message>,
    pub(crate) current_model: ModelColor,
    pub(crate) total_tokens_used: usize,
    pub(crate) logger: Option<ConversationLogger>,
    pub(crate) tool_registry: ToolRegistry,
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
    pub(crate) readline_history: Option<apchat_vty::history::ReadlineHistory>,
    // Content limiter
    pub(crate) content_limiter: Option<Arc<apchat_toolcore::content_limiter::ContentLimiter>>,
    // MSPC channel for multi-stream input processing
    pub(crate) mspc_channel: Option<Arc<crate::mspc::MspcChannel>>,
    // Signal channel for sending confirmation requests to readline
    pub(crate) signal_sender: Option<tokio::sync::mpsc::Sender<crate::mspc::MspcMessage>>,
    // Signal channel receiver for receiving interrupt signals
    pub(crate) signal_receiver: Option<Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<crate::mspc::MspcMessage>>>>,
    // Confirmation registry for managing tool confirmation requests
    pub(crate) confirmation_registry: Option<Arc<apchat_toolcore::confirmation::ConfirmationRegistry>>,
}

impl APChat {

    /// Create a new APChat with content limiter
    pub fn with_content_limiter(mut self, content_limiter: Arc<apchat_toolcore::content_limiter::ContentLimiter>) -> Self {
        self.content_limiter = Some(content_limiter.clone());
        self.tool_registry = self.tool_registry.with_content_limiter(content_limiter);
        self
    }

    /// Create a new APChat with MSPC channel
    pub fn with_mspc_channel(mut self, channel: Arc<crate::mspc::MspcChannel>) -> Self {
        self.mspc_channel = Some(channel);
        self
    }

    pub fn with_signal_sender(mut self, sender: tokio::sync::mpsc::Sender<crate::mspc::MspcMessage>) -> Self {
        self.signal_sender = Some(sender);
        self
    }

    pub fn with_signal_receiver(mut self, receiver: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<crate::mspc::MspcMessage>>>) -> Self {
        self.signal_receiver = Some(receiver);
        self
    }

    pub fn with_confirmation_registry(mut self, registry: Arc<apchat_toolcore::confirmation::ConfirmationRegistry>) -> Self {
        self.confirmation_registry = Some(registry);
        self
    }

    pub fn new(api_key: String, work_dir: PathBuf) -> Self {
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
            policy_manager,
            false,
            false,
            TerminalBackendType::Pty,
            false, // Default early_superpowers to false
            false, // Default delayed_instructions_enabled to false
        )
    }

    /// Set the debug level (0=off, 1=basic, 2=detailed, etc.)
    pub fn set_debug_level(&mut self, level: u32) {
        self.debug_level = level;
    }

    /// Get the current debug level
    pub fn get_debug_level(&self) -> u32 {
        self.debug_level
    }

    /// Check if debug output should be shown for a given level
    pub fn should_show_debug(&self, level: u32) -> bool {
        self.debug_level & (1 << (level - 1)) != 0
    }

    pub fn new_with_config(
        client_config: ClientConfig,
        work_dir: PathBuf,
        policy_manager: PolicyManager,
        stream_responses: bool,
        verbose: bool,
        backend_type: TerminalBackendType,
        early_superpowers: bool,
        delayed_instructions_enabled: bool,
    ) -> Self {
        let tool_registry = initialize_tool_registry(delayed_instructions_enabled);

        // Initialize content limiter
        let content_limiter_config = apchat_toolcore::content_limiter::ContentLimiterConfig::new(&work_dir);
        let content_limiter = Some(Arc::new(apchat_toolcore::content_limiter::ContentLimiter::new(content_limiter_config)));
        let content_limiter_clone = content_limiter.clone();

        // Initialize skill registry
        let skills_dir = work_dir.join("skills");
        let skill_registry = match apchat_skills::SkillRegistry::new(skills_dir) {
            Ok(registry) => Some(Arc::new(registry)),
            Err(e) => {
                print_heart_yellow(&format!("{} Failed to load skills: {}", "⚠️".yellow(), e), true);
                print_heart_yellow(&format!("{} Skills will not be available", "⚠️".yellow()), true);
                None
            }
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
        let system_content = crate::config::get_system_prompt(&client_config, skill_registry.as_ref(), early_superpowers);

        let mut chat = Self {
            api_key: client_config.api_key.clone(),
            work_dir,
            client: reqwest::Client::new(),
            messages: Vec::new(),
            current_model: initial_model,
            total_tokens_used: 0,
            logger: None,
            tool_registry: tool_registry.with_content_limiter(content_limiter_clone.unwrap()),
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
            mspc_channel: None,
            signal_sender: None,
            signal_receiver: None,
            confirmation_registry: None,
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

    pub fn get_tools(&self) -> Vec<Tool> {
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

    pub fn peek_file_top_10_lines(&self, file_path: &str) -> Result<String> {
        let full_path = self.work_dir.join(file_path);
        let content = fs::read_to_string(&full_path)
            .with_context(|| format!("Failed to read file: {}", full_path.display()))?;

        // Return just the content without any metadata
        // This prevents the "[Total: X lines]" from being accidentally included in edits/writes
        Ok(content)
    }

    pub fn switch_model(&mut self, model_str: &str, reason: &str) -> Result<String> {
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
    pub(crate) fn format_current_model_string(&self) -> String {
        let provider = self.client_config.get_provider(self.current_model);
        let model_name = &provider.model_name;

        // Get backend name
        let backend_name = match provider.backend {
            Some(apchat_models::BackendType::Groq) => "groq",
            Some(apchat_models::BackendType::OpenAI) => "openai",
            Some(apchat_models::BackendType::Anthropic) => "anthropic",
            Some(apchat_models::BackendType::Llama) => "llama",
            Some(apchat_models::BackendType::Ollama) => "ollama",
            None => "unknown",
        };

        let api_url = provider.api_url.as_ref()
            .map(|url| url.as_str())
            .unwrap_or("https://api.example.com");

        format!("{}@{}({})", model_name, backend_name, api_url)
    }

    pub fn save_state(&self, file_path: &str) -> Result<String> {
        save_state(&self.messages, &self.current_model, self.total_tokens_used, file_path)
    }

    /// Save conversation history automatically to logs directory
    pub fn auto_save_history(&self) -> Result<String> {
        let history_dir = apchat_logging::get_logs_dir()?.join("history");
        print_heart_red(&format!("History dir: {:?}", &history_dir), true);
        fs::create_dir_all(&history_dir).unwrap();
        let file_name = format!("history-{}.json", self.process_id);
        let file_path = history_dir.join(file_name);

        save_state(&self.messages, &self.current_model, self.total_tokens_used, file_path.to_str().unwrap())
    }

    pub fn load_state(&mut self, file_path: &str) -> Result<String> {
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

    pub async fn execute_tool(&mut self, name: &str, arguments: &str) -> Result<String> {
        // For backward compatibility, handle special tools that need main application state
        match name {
            "switch_model" => {
                let args: SwitchModelArgs = serde_json::from_str(arguments)?;
                self.switch_model(&args.model, &args.reason)
            }
            _ => {
                // Use the tool registry for all tools (including plan_edits and apply_edit_plan)
                let params = ToolParameters::from_json(arguments)
                    .with_context(|| format!("Failed to parse tool arguments for '{}'.", name))?;

                // Get tool schema for validation
                let tool = self.tool_registry.get_tool(name)
                    .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found in registry.", name))?;

                // Get parameter definitions from the tool
                let param_definitions: HashMap<String, Value> = tool.parameters()
                    .iter()
                    .map(|(key, def)| {
                        (key.clone(), serde_json::to_value(def).unwrap_or(Value::Null))
                    })
                    .collect();

                // Create a minimal ToolCall for validation (only need function name and arguments)
                let tool_call = apchat_models::ToolCall {
                    id: format!("call_{}", chrono::Utc::now().timestamp_millis()),
                    tool_type: "function".to_string(),
                    function: apchat_models::FunctionCall {
                        name: name.to_string(),
                        arguments: arguments.to_string(),
                    },
                };

                // Validate the tool call against the schema
                let params = match validate_tool_call(&tool_call, &params, &param_definitions) {
                    Ok(params) => params,
                    Err(e) => return Err(anyhow::anyhow!("Parameter validation failed for '{}': {}", name, e)),
                };

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

                // Add MSPC sender and receiver if available
                if let Some(ref mspc_channel) = self.mspc_channel {
                    context = context.with_mspc_sender(mspc_channel.sender());
                    context = context.with_mspc_receiver(mspc_channel.receiver());
                }

                // Add signal sender and confirmation registry if available
                if let Some(ref signal_sender) = self.signal_sender {
                    context = context.with_signal_sender(signal_sender.clone());
                }

                // Add signal receiver if available (for interruptible tools)
                if let Some(ref signal_receiver) = self.signal_receiver {
                    context = context.with_signal_receiver(signal_receiver.clone());
                }

                if let Some(ref confirmation_registry) = self.confirmation_registry {
                    context = context.with_confirmation_registry(confirmation_registry.clone());
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


}

/// Resolve terminal backend type from CLI args and environment variable
/// Priority: CLI arg > ENV var > default (PTY)
pub fn resolve_terminal_backend(cli: &Cli) -> Result<TerminalBackendType> {
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
