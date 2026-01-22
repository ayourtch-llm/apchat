use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc as tokio_mpsc};
use apchat_policy::PolicyManager;
use apchat_terminal::TerminalManager;
use apchat_skills::SkillRegistry;
use apchat_todo::TodoManager;
use apchat_vty::print_heart_red;
use crate::content_limiter::ContentLimiter;
use apchat_models::types::ModelColor;
use apchat_mspc::MspcMessage;

/// Tool execution context
///
/// This struct provides the execution context for tools, including:
/// - Working directory for file operations
/// - Session identifier for tracking operations
/// - Environment variables for configuration
/// - Policy manager for permission checking
/// - Terminal manager for PTY session management
/// - Skill registry for accessing skills
/// - Todo manager for task tracking
/// - Non-interactive flag for web/API mode
/// - Current model string for subagent spawning (formatted as "modname@backend(url)")
/// - LLM clients for making API calls
/// - MSPC channel sender for broadcasting progress updates
/// - MSPC channel receiver for listening to interrupt signals
#[derive(Clone)]
pub struct ToolContext {
    pub work_dir: PathBuf,
    pub session_id: String,
    pub environment: HashMap<String, String>,
    pub policy_manager: PolicyManager,
    pub terminal_manager: Option<Arc<Mutex<TerminalManager>>>,
    pub skill_registry: Option<Arc<SkillRegistry>>,
    pub todo_manager: Option<Arc<TodoManager>>,
    pub non_interactive: bool,
    pub current_model_string: Option<String>,
    pub content_limiter: Option<Arc<ContentLimiter>>, // NEW
    pub llm_clients: HashMap<ModelColor, Arc<dyn apchat_llm_api::client::LlmClient>>, // NEW
    pub mspc_sender: Option<tokio_mpsc::Sender<MspcMessage>>, // NEW - MSPC channel sender
    pub mspc_receiver: Option<Arc<Mutex<tokio_mpsc::Receiver<MspcMessage>>>>, // NEW - MSPC channel receiver
}

impl std::fmt::Debug for ToolContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolContext")
            .field("work_dir", &self.work_dir)
            .field("session_id", &self.session_id)
            .field("environment", &self.environment)
            .field("policy_manager", &self.policy_manager)
            .field("terminal_manager", &self.terminal_manager)
            .field("skill_registry", &self.skill_registry)
            .field("todo_manager", &self.todo_manager)
            .field("non_interactive", &self.non_interactive)
            .field("current_model_string", &self.current_model_string)
            .field("content_limiter", &self.content_limiter)
            .field("llm_clients_count", &self.llm_clients.len())
            .field("mspc_sender", &self.mspc_sender.is_some())
            .field("mspc_receiver", &self.mspc_receiver.is_some())
            .finish()
    }
}

impl ToolContext {
    pub fn new(work_dir: PathBuf, session_id: String, policy_manager: PolicyManager) -> Self {
        Self {
            work_dir,
            session_id,
            environment: HashMap::new(),
            policy_manager,
            terminal_manager: None,
            skill_registry: None,
            todo_manager: None,
            non_interactive: false,
            current_model_string: None,
            content_limiter: None,
            llm_clients: HashMap::new(),
            mspc_sender: None,
            mspc_receiver: None,
        }
    }

    pub fn with_non_interactive(mut self, non_interactive: bool) -> Self {
        self.non_interactive = non_interactive;
        self
    }

    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.environment.insert(key, value);
        self
    }

    pub fn with_terminal_manager(mut self, terminal_manager: Arc<Mutex<TerminalManager>>) -> Self {
        self.terminal_manager = Some(terminal_manager);
        self
    }

    pub fn with_skill_registry(mut self, skill_registry: Arc<SkillRegistry>) -> Self {
        self.skill_registry = Some(skill_registry);
        self
    }

    pub fn with_todo_manager(mut self, todo_manager: Arc<TodoManager>) -> Self {
        self.todo_manager = Some(todo_manager);
        self
    }

    pub fn with_content_limiter(mut self, content_limiter: Arc<ContentLimiter>) -> Self {
        self.content_limiter = Some(content_limiter);
        self
    }

    pub fn with_current_model_string(mut self, model_string: String) -> Self {
        self.current_model_string = Some(model_string);
        self
    }

    pub fn with_llm_clients(mut self, llm_clients: HashMap<ModelColor, Arc<dyn apchat_llm_api::client::LlmClient>>) -> Self {
        self.llm_clients = llm_clients;
        self
    }

    pub fn with_mspc_sender(mut self, sender: tokio_mpsc::Sender<MspcMessage>) -> Self {
        self.mspc_sender = Some(sender);
        self
    }

    pub fn with_mspc_receiver(mut self, receiver: Arc<Mutex<tokio_mpsc::Receiver<MspcMessage>>>) -> Self {
        self.mspc_receiver = Some(receiver);
        self
    }

    /// Get an LLM client for a specific model color
    /// Returns None if no client is configured for that color
    pub fn get_llm_client(&self, model_color: &ModelColor) -> Option<Arc<dyn apchat_llm_api::client::LlmClient>> {
        self.llm_clients.get(model_color).cloned()
    }

    /// Check if an action is permitted by the policy
    /// Returns (approved: bool, rejection_reason: Option<String>)
    pub fn check_permission(
        &self,
        action: apchat_policy::ActionType,
        target: &str,
        prompt_message: &str,
    ) -> anyhow::Result<(bool, Option<String>)> {
        use apchat_policy::Decision;
        use colored::Colorize;
        use std::io::{self, BufRead, Write};

        let decision = self.policy_manager.evaluate(&action, target);

        match decision {
            Decision::Allow => Ok((true, None)),
            Decision::Deny => Ok((false, Some("Denied by policy".to_string()))),
            Decision::Ask => {
                // Auto-approve for memory-related actions (non-interactive mode for memory ops)
                // Memory operations are tool operations, not direct user actions
                if matches!(
                    action,
                    apchat_policy::ActionType::MemoryStore
                        | apchat_policy::ActionType::MemoryQuery
                        | apchat_policy::ActionType::MemoryUpdate
                        | apchat_policy::ActionType::MemoryList
                        | apchat_policy::ActionType::MemoryDelete
                ) {
                    print_heart_red(&format!("{} {}", "✓".green(), "Auto-confirmed (memory operation)".bright_black()), true);
                    return Ok((true, None));
                }

                // In non-interactive mode (web/API), auto-approve since confirmation
                // was already handled via web UI
                if self.non_interactive {
                    print_heart_red(&format!("{} {}", "✓".green(), "Auto-confirmed (web UI)".bright_black()), true);
                    return Ok((true, None));
                }

                // Ask the user for confirmation in interactive mode
                print_heart_red(&format!("\n{}", prompt_message.bright_green().bold()), true);
                print_heart_red(&format!(">>> "), false);
                io::stdout().flush()?;

                let stdin = io::stdin();
                let mut handle = stdin.lock();
                let mut response = String::new();
                handle.read_line(&mut response)?;

                let response = response.trim();
                let response_lower = response.to_lowercase();
                let approved = response_lower.is_empty() || response_lower == "y" || response_lower == "yes";

                let rejection_reason = if !approved {
                    // Ask for reason if rejected
                    print_heart_red(&format!("{}", "Why not? (optional - helps the AI understand):".bright_yellow()), true);
                    print_heart_red(&format!(">>> "), false);
                    io::stdout().flush()?;

                    let mut reason = String::new();
                    match handle.read_line(&mut reason) {
                        Ok(_) => {
                            let reason = reason.trim();
                            if reason.is_empty() {
                                None
                            } else {
                                Some(reason.to_string())
                            }
                        }
                        Err(_) => None,
                    }
                } else {
                    None
                };

                // Learn from the user's decision if learning is enabled
                if self.policy_manager.is_learning() {
                    let decision = if approved { Decision::Allow } else { Decision::Deny };
                    let _ = self.policy_manager.learn(action, target.to_string(), decision, rejection_reason.clone());
                }

                Ok((approved, rejection_reason))
            }
        }
    }
}