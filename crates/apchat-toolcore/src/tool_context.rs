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
    pub signal_sender: Option<tokio_mpsc::Sender<MspcMessage>>, // NEW - Signal channel sender (for confirmation requests)
    pub signal_receiver: Option<tokio_mpsc::Receiver<MspcMessage>>, // NEW - Signal channel receiver (for confirmation responses)
    pub confirmation_registry: Option<Arc<crate::confirmation::ConfirmationRegistry>>, // NEW - Confirmation registry
}

impl Clone for ToolContext {
    fn clone(&self) -> Self {
        Self {
            work_dir: self.work_dir.clone(),
            session_id: self.session_id.clone(),
            environment: self.environment.clone(),
            policy_manager: self.policy_manager.clone(),
            terminal_manager: self.terminal_manager.clone(),
            skill_registry: self.skill_registry.clone(),
            todo_manager: self.todo_manager.clone(),
            non_interactive: self.non_interactive,
            current_model_string: self.current_model_string.clone(),
            content_limiter: self.content_limiter.clone(),
            llm_clients: self.llm_clients.clone(),
            mspc_sender: self.mspc_sender.clone(),
            mspc_receiver: self.mspc_receiver.clone(),
            signal_sender: self.signal_sender.clone(),
            // Note: signal_receiver cannot be cloned, so we set it to None
            signal_receiver: None,
            confirmation_registry: self.confirmation_registry.clone(),
        }
    }
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
            .field("signal_sender", &self.signal_sender.is_some())
            .field("signal_receiver", &self.signal_receiver.is_some())
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
            signal_sender: None,
            signal_receiver: None,
            confirmation_registry: None,
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

    pub fn with_signal_sender(mut self, sender: tokio_mpsc::Sender<MspcMessage>) -> Self {
        self.signal_sender = Some(sender);
        self
    }

    pub fn with_signal_receiver(mut self, receiver: tokio_mpsc::Receiver<MspcMessage>) -> Self {
        self.signal_receiver = Some(receiver);
        self
    }

    pub fn with_confirmation_registry(mut self, registry: Arc<crate::confirmation::ConfirmationRegistry>) -> Self {
        self.confirmation_registry = Some(registry);
        self
    }

    /// Get an LLM client for a specific model color
    /// Returns None if no client is configured for that color
    pub fn get_llm_client(&self, model_color: &ModelColor) -> Option<Arc<dyn apchat_llm_api::client::LlmClient>> {
        self.llm_clients.get(model_color).cloned()
    }

    /// Check if an action is permitted by the policy
    /// Returns (approved: bool, rejection_reason: Option<String>)
    /// Check if an action is permitted by the policy
    /// Returns (approved: bool, rejection_reason: Option<String>)
    ///
    /// This async version uses MSPC for confirmation requests when available,
    /// falling back to stdin for backward compatibility.
    pub async fn check_permission_async(
        &self,
        action: apchat_policy::ActionType,
        target: &str,
        prompt_message: &str,
    ) -> anyhow::Result<(bool, Option<String>)> {
        use apchat_policy::Decision;
        use colored::Colorize;

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

                // Try to use signal channel for confirmation if available (preferred)
                if let Some(ref signal_sender) = self.signal_sender {
                    return self.check_permission_via_signal(signal_sender, &action, target, prompt_message).await;
                }

                // Fall back to stdin-based confirmation for backward compatibility
                self.check_permission_via_stdin(&action, target, prompt_message).await
            }
        }
    }

    /// Check permission using signal channel for confirmation
    /// This is the preferred method as it sends confirmation requests to the readline function
    async fn check_permission_via_signal(
        &self,
        signal_sender: &tokio_mpsc::Sender<MspcMessage>,
        action: &apchat_policy::ActionType,
        target: &str,
        prompt_message: &str,
    ) -> anyhow::Result<(bool, Option<String>)> {
        use apchat_policy::Decision;
        use colored::Colorize;

        // Check if confirmation registry is available
        if self.confirmation_registry.is_none() {
            print_heart_red(&format!("{} No confirmation registry available, falling back to stdin", "⚠️".yellow()), true);
            return self.check_permission_via_stdin(action, target, prompt_message).await;
        }

        let registry = self.confirmation_registry.as_ref().unwrap();

        // Register a pending confirmation and get a unique ID with a receiver
        let (confirmation_id, mut response_rx) = registry.register().await;

        // Send confirmation request via signal channel to readline
        let confirmation_msg = format!(
            "{}\nAction: {:?}\nTarget: {}",
            prompt_message, action, target
        );

        if let Err(e) = signal_sender.send(MspcMessage::ToolConfirmationRequest {
            content: confirmation_msg,
            confirmation_id: confirmation_id.clone(),
        }).await {
            // If sending fails, cancel the confirmation and fall back to stdin
            registry.cancel(&confirmation_id).await;
            print_heart_red(&format!("{} Failed to send confirmation via signal channel: {}", "⚠️".yellow(), e), true);
            return self.check_permission_via_stdin(action, target, prompt_message).await;
        }

        // Wait for confirmation response via the oneshot channel
        let timeout_duration = tokio::time::Duration::from_secs(300); // 5 minute timeout

        match tokio::time::timeout(timeout_duration, response_rx).await {
            Ok(Ok((approved, reason))) => {
                // Learn from the user's decision if learning is enabled
                if self.policy_manager.is_learning() {
                    let decision = if approved { Decision::Allow } else { Decision::Deny };
                    let _ = self.policy_manager.learn(action.clone(), target.to_string(), decision, reason.clone());
                }

                Ok((approved, reason))
            }
            Ok(Err(_)) => {
                // Response sender dropped
                print_heart_red(&format!("{} Confirmation response channel closed", "⚠️".yellow()), true);
                Ok((false, Some("Confirmation channel closed".to_string())))
            }
            Err(_) => {
                print_heart_red(&format!("{} Confirmation timeout after 5 minutes", "⏱".yellow()), true);
                Ok((false, Some("Confirmation timeout".to_string())))
            }
        }
    }

    /// Check permission using stdin (fallback for backward compatibility)
    async fn check_permission_via_stdin(
        &self,
        action: &apchat_policy::ActionType,
        target: &str,
        prompt_message: &str,
    ) -> anyhow::Result<(bool, Option<String>)> {
        use apchat_policy::Decision;
        use colored::Colorize;
        use std::io::{self, BufRead, Write};

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
            let _ = self.policy_manager.learn(action.clone(), target.to_string(), decision, rejection_reason.clone());
        }

        Ok((approved, rejection_reason))
    }

    /// Synchronous version of check_permission (for backward compatibility)
    /// This will block on the async runtime, so prefer check_permission_async when possible.
    pub fn check_permission(
        &self,
        action: apchat_policy::ActionType,
        target: &str,
        prompt_message: &str,
    ) -> anyhow::Result<(bool, Option<String>)> {
        use apchat_policy::Decision;
        use colored::Colorize;

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

                // Fall back to stdin for synchronous context
                // Note: This will not use MSPC as it requires async
                self.check_permission_sync_fallback(&action, target, prompt_message)
            }
        }
    }

    /// Synchronous fallback for check_permission (stdin-based)
    fn check_permission_sync_fallback(
        &self,
        action: &apchat_policy::ActionType,
        target: &str,
        prompt_message: &str,
    ) -> anyhow::Result<(bool, Option<String>)> {
        use apchat_policy::Decision;
        use colored::Colorize;
        use std::io::{self, BufRead, Write};

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
            let _ = self.policy_manager.learn(action.clone(), target.to_string(), decision, rejection_reason.clone());
        }

        Ok((approved, rejection_reason))
    }
}