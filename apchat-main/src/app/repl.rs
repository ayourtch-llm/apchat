use anyhow::Result;
use colored::Colorize;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::io::{BufRead, Write};

use apchat_vty::{print_heart_yellow, print_heart_red};

use crate::APChat;
use crate::cli::Cli;
use crate::config::ClientConfig;
use crate::chat::history::intelligent_compaction;
use crate::mspc::{MspcChannel, MspcMessage};
use crate::mspc::{OutputDestination, broadcast_to_all, OutputMessage};
use crate::input_router::TerminalInputRouter;
use crate::app::TerminalOutputDestination;
use apchat_policy::PolicyManager;
use apchat_logging::ConversationLogger;
use apchat_models::{ModelColor, Message};

/// Run interactive REPL mode
pub async fn run_repl_mode(
    cli: &Cli,
    client_config: ClientConfig,
    work_dir: PathBuf,
    policy_manager: PolicyManager,
    webex_sink: Option<std::sync::Arc<apchat_webex::WebexOutputSink>>,
    mspc_channel_opt: Option<Arc<MspcChannel>>,
) -> Result<()> {
    print_heart_red(&format!("{}", "🤖 APChat - Claude Code-like Experience".bright_cyan().bold()), true);
    print_heart_red(&format!("{}", format!("Working directory: {}", work_dir.display()).bright_black()), true);

    if cli.agents {
        print_heart_red(&format!("{}", "🚀 Multi-Agent System ENABLED - Specialized agents will handle your tasks".green().bold()), true);
    }

    print_heart_red(&format!("{}", "Type 'exit' or 'quit' to exit, '/model' to switch models, '/history' to view history, or '/skills' for all commands\n".bright_black()), true);

    // Resolve terminal backend
    let backend_type = crate::resolve_terminal_backend(cli)?;

    let mut chat = APChat::new_with_config(
        client_config,
        work_dir,
        cli.agents,
        policy_manager,
        cli.stream,
        cli.verbose,
        backend_type,
        cli.early_superpowers,
    );

    // Comprehensive model configuration display
    print_heart_red(&format!("{}", "═".repeat(80).bright_black()), true);
    print_heart_red(&format!("{}", "🤖 Model Configuration".bright_cyan().bold()), true);
    print_heart_red(&format!("{}", "═".repeat(80).bright_black()), true);

    // Current model display
    let current_model_display = match chat.current_model {
        ModelColor::BluModel => format!("BluModel/{} (auto-switched from default)", chat.current_model.display_name()),
        ModelColor::GrnModel => format!("GrnModel/{} (default)", chat.current_model.display_name()),
        ModelColor::RedModel => format!("RedModel/{}", chat.current_model.display_name()),
    };
    print_heart_red(&format!("{} {}", "📍 Current:".bright_green().bold(), current_model_display.bright_white()), true);

    // Display model provider debug info
    print_heart_red(&format!("\n{} Model Providers:", "🔧".bright_cyan().bold()), true);
    print_heart_red(&format!("{} BluModel: {:?}", "   ".bright_black(), chat.client_config.get_provider(ModelColor::BluModel)), true);
    print_heart_red(&format!("{} GrnModel: {:?}", "   ".bright_black(), chat.client_config.get_provider(ModelColor::GrnModel)), true);
    print_heart_red(&format!("{} RedModel: {:?}", "   ".bright_black(), chat.client_config.get_provider(ModelColor::RedModel)), true);

    // Function to get API key preview for a model based on its resolved configuration
    let get_api_key_preview = |model_color: ModelColor, _backend: &str| {
        let key_preview = match model_color {
            ModelColor::BluModel => {
                if let Some(key) = chat.client_config.get_api_key(ModelColor::BluModel) {
                    Some(format!("{}***", &key[..key.len().min(3)]))
                } else if !chat.client_config.api_key.is_empty() {
                    Some(format!("{}***", &chat.client_config.api_key[..chat.client_config.api_key.len().min(3)]))
                } else {
                    None
                }
            }
            ModelColor::GrnModel => {
                if let Some(key) = chat.client_config.get_api_key(ModelColor::GrnModel) {
                    Some(format!("{}***", &key[..key.len().min(3)]))
                } else if !chat.client_config.api_key.is_empty() {
                    Some(format!("{}***", &chat.client_config.api_key[..chat.client_config.api_key.len().min(3)]))
                } else {
                    None
                }
            }
            ModelColor::RedModel => {
                if let Some(key) = chat.client_config.get_api_key(ModelColor::RedModel) {
                    Some(format!("{}***", &key[..key.len().min(3)]))
                } else if !chat.client_config.api_key.is_empty() {
                    Some(format!("{}***", &chat.client_config.api_key[..chat.client_config.api_key.len().min(3)]))
                } else {
                    None
                }
            }
        };
        
        key_preview.map(|k| k.green())
    };

    // Display model details by iterating over all colors
    for model_color in ModelColor::iter() {
        let provider = chat.client_config.get_provider(model_color);
        let backend_name = provider.backend.as_ref().map_or("Groq", |b| b.as_str());
        
        let (label, has_star) = match model_color {
            ModelColor::BluModel => ("BluModel:".bright_blue().bold(), false),
            ModelColor::GrnModel => ("GrnModel:".bright_green().bold(), true),
            ModelColor::RedModel => ("RedModel:".bright_red().bold(), false),
        };
        
        let star_suffix = if has_star { " ⭐" } else { "" };
        print_heart_red(&format!("{} {} ({}){}", label, provider.model_name, backend_name.bright_black(), star_suffix), true);
        print_heart_red(&format!("   {} {}", "API:".bright_black(), provider.api_url.as_ref().map(|s| s.as_str()).unwrap_or("https://api.groq.com/openai/v1/chat/completions").bright_black()), true);
        if let Some(key_preview) = get_api_key_preview(model_color, backend_name) {
            print_heart_red(&format!("   {} {}", "Key:".bright_black(), key_preview), true);
        }
    }

    print_heart_red(&format!("{}", "═".repeat(80).bright_black()), true);

    // Debug info (shown at debug level 1+)
    if chat.should_show_debug(1) {
        print_heart_red(&format!("{}", format!("🔧 DEBUG: blu_model URL: {:?}", chat.client_config.get_api_url(ModelColor::BluModel)).bright_black()), true);
        print_heart_red(&format!("{}", format!("🔧 DEBUG: grn_model URL: {:?}", chat.client_config.get_api_url(ModelColor::GrnModel)).bright_black()), true);
        print_heart_red(&format!("{}", format!("🔧 DEBUG: Current model: {:?}", chat.current_model).bright_black()), true);
    }

    // Initialize logger (async) – logs go into the workspace directory
    chat.logger = match ConversationLogger::new(&chat.work_dir).await {
        Ok(l) => Some(l),
        Err(e) => {
            print_heart_yellow(&format!("Logging disabled: {}", e), true);
            None
        }
    };

    // If logger was created, log the initial system message that APChat::new added
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

    // Run session-start hook to inject skill context
    let hook_path = chat.work_dir.join("hooks/session-start.sh");
    if hook_path.exists() {
        use std::process::Command;
        match Command::new(&hook_path)
            .arg(chat.work_dir.to_string_lossy().to_string())
            .output()
        {
            Ok(output) if output.status.success() => {
                let hook_content = String::from_utf8_lossy(&output.stdout).to_string();
                if !hook_content.trim().is_empty() {
                    // Add hook output as a system message
                    chat.messages.push(Message {
                        role: "system".to_string(),
                        content: hook_content,
                        tool_calls: None,
                        tool_call_id: None,
                        name: None,
                        reasoning: None,
                    });

                    if cli.verbose {
                        print_heart_red(&format!("{}", "✓ Session-start hook executed".green()), true);
                    }
                }
            }
            Ok(output) => {
                print_heart_yellow(&format!("{} Session-start hook failed: {}",
                    "⚠️".yellow(),
                    String::from_utf8_lossy(&output.stderr)), true);
            }
            Err(e) => {
                print_heart_yellow(&format!("{} Failed to execute session-start hook: {}", "⚠️".yellow(), e), true);
            }
        }
    }

    // Load readline history (in a scope to ensure guard is dropped)
    {
        let mut rl = apchat_vty::ReadlineInstance::get()?;
        match apchat_vty::history::load_and_add_to_editor(&mut rl) {
            Ok(_) => {
                let history_len = apchat_vty::history::load_history(None)?.len();
                print_heart_red(&format!("{} Loaded {} readline history entries",
                         "📖".bright_green(),
                         history_len), true);
            }
            Err(e) => {
                print_heart_yellow(&format!("{} Failed to load readline history: {}", "⚠️".yellow(), e), true);
            }
        }
        // Guard is automatically dropped here when scope ends
    }

    // Validate and process idle timeout configuration
    let idle_config = if let (Some(timeout_secs), Some(input_text)) = (&cli.idle_timeout, &cli.idle_input) {
        // Validate timeout range
        if *timeout_secs < 1 || *timeout_secs > 86400 {
            anyhow::bail!("Idle timeout must be between 1 and 86400 seconds");
        }
        print_heart_red(&format!("{} Idle timeout enabled: {} seconds -> \"{}\"",
            "⏱️".bright_yellow(),
            timeout_secs,
            input_text.bright_cyan()
        ), true);
        Some((*timeout_secs, input_text.clone()))
    } else if cli.idle_timeout.is_some() || cli.idle_input.is_some() {
        anyhow::bail!("Both --idle-timeout and --idle-input must be specified together");
    } else {
        None
    };

    // Read apchat.md if it exists to get project context
    let kimi_context = if let Ok(kimi_content) = chat.read_file("apchat.md") {
        print_heart_red(&format!("{} {}", "📖".bright_cyan(), "Reading project context from apchat.md...".bright_black()), true);
        kimi_content
    } else {
        print_heart_red(&format!("{} {}", "📖".bright_cyan(), "No apchat.md found. Starting fresh.".bright_black()), true);
        String::new()
    };

    if !kimi_context.is_empty() {
        let sys_msg = Message {
            role: "system".to_string(),
            content: format!("Project context: {}", kimi_context),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning: None,
        };
        // Log this system addition
        if let Some(logger) = &mut chat.logger {
            logger
                .log("system", &sys_msg.content, None, false)
                .await;
        }
        chat.messages.push(sys_msg);
    }

    // Set up a persistent Ctrl-C handler for the entire REPL session
    // This holds the current operation's cancellation token
    let current_token: std::sync::Arc<std::sync::Mutex<Option<tokio_util::sync::CancellationToken>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let current_token_for_handler = current_token.clone();

    // Spawn a single Ctrl-C handler that will last the entire session
    tokio::spawn(async move {
        loop {
            if tokio::signal::ctrl_c().await.is_ok() {
                if let Ok(guard) = current_token_for_handler.lock() {
                    if let Some(ref token) = *guard {
                        print_heart_red(&format!("\n{}", "^C - Interrupting...".bright_yellow()), true);
                        token.cancel();
                    }
                }
            }
        }
    });

    // Initialize MSPC channel for input decoupling
    // Use provided channel (shared with Webex) or create new one
    let mspc_channel = mspc_channel_opt.unwrap_or_else(|| Arc::new(MspcChannel::new(100)));

    // Set MSPC channel on chat so tools can use it for confirmation requests
    chat.mspc_channel = Some(mspc_channel.clone());

    // Create a confirmation registry for tool confirmations
    use apchat_toolcore::confirmation::ConfirmationRegistry;
    let confirmation_registry = Arc::new(ConfirmationRegistry::new());
    chat.confirmation_registry = Some(confirmation_registry.clone());

    // Create a signal channel for sending signals (e.g., confirmation requests) to the terminal input router
    let (signal_sender, signal_receiver) = tokio::sync::mpsc::channel::<MspcMessage>(10);
    let signal_sender_for_main = signal_sender.clone();

    // Create an interrupt channel for tools to receive interrupt signals
    let (interrupt_sender, interrupt_receiver) = tokio::sync::mpsc::channel::<MspcMessage>(10);
    let interrupt_receiver_mutex = Arc::new(tokio::sync::Mutex::new(interrupt_receiver));

    // Set signal sender on chat so tools can send confirmation requests
    chat.signal_sender = Some(signal_sender);
    chat.signal_receiver = Some(interrupt_receiver_mutex);

    // Create output destinations
    let mut output_destinations: Vec<Box<dyn OutputDestination>> = vec![];
    output_destinations.push(Box::new(TerminalOutputDestination::new()));

    // Share current model state with background input task for prompt display
    let current_model_shared = Arc::new(std::sync::RwLock::new(chat.current_model));
    let current_model_for_main = current_model_shared.clone();

    // Spawn terminal input router to handle stdin and route to MSPC channel
    let mut terminal_router = TerminalInputRouter::new(mspc_channel.clone());
    terminal_router = terminal_router.with_signal_receiver(signal_receiver);
    let client_config_for_router = chat.client_config.clone();
    let confirmation_registry_for_router = confirmation_registry.clone(); // Clone for router task
    let interrupt_sender_for_router = interrupt_sender.clone(); // Clone for router task

    let router_handle = tokio::spawn(async move {
        // Wrap the signal receiver in a Tokio Mutex so it can be shared across spawn_blocking calls
        let signal_receiver_mutex = Arc::new(tokio::sync::Mutex::new(
            terminal_router.take_signal_receiver().expect("Signal receiver should be set")
        ));

        loop {
            // Get current model state for prompt
            let current_model = {
                current_model_shared.read().unwrap().clone()
            };

            let model_name = get_model_name_for_prompt(&current_model, &client_config_for_router);
            let model_indicator = format!("[{} ({})]", current_model.display_name(), model_name).bright_magenta();
            let prompt_string = format!("{} {}", model_indicator, "You:".bright_green().bold());

            // Clone the Arc for use in spawn_blocking
            let receiver_mutex_clone = signal_receiver_mutex.clone();

            // Use spawn_blocking for readline (it's a blocking operation)
            // We pass the signal receiver to readline so it can receive confirmation requests
            let line_result = tokio::task::spawn_blocking(move || {
                // Lock the mutex to get access to the receiver
                let mut receiver_guard = receiver_mutex_clone.blocking_lock();
                // Dereference the MutexGuard to get access to the Receiver
                let receiver_ref = &mut *receiver_guard;

                apchat_vty::ReadlineInstance::readline_with_mspc(&prompt_string, Some(receiver_ref))
            }).await;

            match line_result {
                Ok(Ok(Some(line))) => {
                    // Add to readline history immediately so up-arrow works in next readline() call
                    if let Err(e) = apchat_vty::ReadlineInstance::add_history(&line) {
                        print_heart_yellow(&format!("{} Failed to add to history: {}", "⚠️".bright_yellow(), e), true);
                    }

                    let message = terminal_router.parse_input(&line);
                    if terminal_router.send_to_channel(message).await.is_err() {
                        break; // Channel closed, exit
                    }
                }
                Ok(Ok(None)) => {
                    // Empty line or confirmation request handled - continue
                    continue;
                }
                Ok(Err(e)) => {
                    let err_str = e.to_string();
                    // Check for tool confirmation response
                    if err_str.starts_with("__TOOL_CONFIRMATION_RESPONSE__:") {
                        // Extract the confirmation response and forward to confirmation registry
                        let response_str = err_str.strip_prefix("__TOOL_CONFIRMATION_RESPONSE__:").unwrap();
                        let parts: Vec<&str> = response_str.splitn(3, '|').collect();
                        let approved = parts.get(0).map(|s| *s == "true").unwrap_or(false);
                        let confirmation_id = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
                        let reason = parts.get(2).map(|s| s.to_string());

                        // Forward to confirmation registry
                        let registry = confirmation_registry_for_router.clone();
                        if let Err(e) = registry.complete(&confirmation_id, (approved, reason)).await {
                            print_heart_yellow(&format!("{} Failed to complete confirmation: {}", "⚠️".yellow(), e), true);
                        }
                        continue;
                    } else if err_str.starts_with("__CONFIRMATION_RESPONSE__:") {
                        // Extract the confirmation response and forward to main channel
                        let response_str = err_str.strip_prefix("__CONFIRMATION_RESPONSE__:").unwrap();
                        let parts: Vec<&str> = response_str.splitn(2, '|').collect();
                        let approved = parts.get(0).map(|s| *s == "true").unwrap_or(false);
                        let reason = parts.get(1).map(|s| s.to_string());
                        let _ = terminal_router.send_to_channel(
                            MspcMessage::ConfirmationResponse(approved, reason)
                        ).await;
                        continue;
                    } else if err_str.contains("EOF") {
                        // Ctrl-D pressed - send exit command
                        let _ = terminal_router.send_to_channel(
                            MspcMessage::Command("exit".to_string(), Some("terminal".to_string()))
                        ).await;
                        break;
                    } else if err_str.contains("Interrupted") {
                        // Ctrl-C pressed - send interrupt signal to cancel current operation
                        let _ = terminal_router.send_to_channel(
                            MspcMessage::InterruptSignal("interrupt".to_string(), Some("terminal".to_string()))
                        ).await;
                        continue;
                    } else {
                        // Other errors - exit
                        print_heart_yellow(&format!("{} {}", "Error reading input:".bright_red().bold(), e), true);
                        break;
                    }
                }
                Err(_) => break, // Task panic
            }
        }
    });


    // Clone interrupt sender for main loop
    let interrupt_sender_for_main = interrupt_sender.clone();

    // Helper function to get model name for a color from client config
    fn get_model_name_for_prompt(color: &ModelColor, client_config: &crate::config::ClientConfig) -> String {
        if let Some(override_model) = client_config.get_model_override(*color) {
            override_model.to_string()
        } else {
            let provider = client_config.get_provider(*color);
            provider.model_name.clone()
        }
    }

    'outer: loop {
        // Receive message from MSPC channel (background task handles readline)
        let mspc_result = mspc_channel.recv().await;
eprintln!("AYXX: {:?}", mspc_result);
        let message = match mspc_result {
            Some(msg) => msg,
            None => {
                // Channel closed, exit
                print_heart_red(&format!("\n{}", "Goodbye!".bright_cyan()), true);

                // Save readline history before exiting
                if let Err(save_err) = apchat_vty::ReadlineInstance::save_history() {
                    if chat.debug_level > 0 {
                        print_heart_yellow(&format!("{} Failed to save readline history: {}", "⚠️".yellow(), save_err), true);
                    }
                }

                break;
            }
        };

        // Extract content and sender from message
        let line = match message {
            MspcMessage::UserInput(content, _sender) => content,
            MspcMessage::Command(content, _sender) => content,
            MspcMessage::InterruptSignal(content, sender) => {
                // Forward interrupt signal to tools
                let _ = interrupt_sender_for_main.send(MspcMessage::InterruptSignal(content, sender)).await;

                // Interrupt without active operation - just show message and continue
                print_heart_red(&format!("\n{}", "No operation in progress to interrupt".bright_yellow()), true);
                continue;
            }
            MspcMessage::ConfirmationRequest(content, _sender) => {
                // Confirmation request from tool - display prompt and wait for response
                print_heart_red(&format!("\n{}", content.bright_green().bold()), true);
                print_heart_red(&format!("{} ", ">>>".bright_cyan()), false);
                std::io::stdout().flush().ok();

                // Read user response
                let stdin = std::io::stdin();
                let mut handle = stdin.lock();
                let mut response = String::new();

                if let Err(e) = handle.read_line(&mut response) {
                    print_heart_yellow(&format!("{} Failed to read response: {}", "❌".bright_red(), e), true);
                    // Send rejection on error
                    let _ = mspc_channel.send(MspcMessage::ConfirmationResponse(false, Some("Failed to read response".to_string()))).await;
                    continue;
                }

                let response = response.trim();
                let response_lower = response.to_lowercase();
                let approved = response_lower.is_empty() || response_lower == "y" || response_lower == "yes";

                // Get rejection reason if not approved
                let rejection_reason = if !approved {
                    print_heart_red(&format!("{} ", "Why not? (optional - helps the AI understand):".bright_yellow()), false);
                    std::io::stdout().flush().ok();

                    let mut reason = String::new();
                    if let Ok(_) = handle.read_line(&mut reason) {
                        let reason = reason.trim();
                        if reason.is_empty() {
                            None
                        } else {
                            Some(reason.to_string())
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Send confirmation response back via MSPC
                let _ = mspc_channel.send(MspcMessage::ConfirmationResponse(approved, rejection_reason)).await;
                continue;
            }
            _ => {
                // Unexpected message type for REPL, skip
                continue;
            }
        };

        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        // Handle exit commands
        if line == "exit" || line == "quit" {
            print_heart_red(&format!("{}", "Goodbye!".bright_cyan()), true);

            // Save readline history before exiting
            if let Err(e) = apchat_vty::ReadlineInstance::save_history() {
                if chat.debug_level > 0 {
                    print_heart_yellow(&format!("{} Failed to save readline history: {}", "⚠️".yellow(), e), true);
                }
            }

            break;
        }

        // Handle /save and /load commands
        if line.starts_with("/save ") {
            let file_path = line[6..].trim();
            match chat.save_state(file_path) {
                Ok(msg) => print_heart_red(&format!("{} {}", "💾".bright_green(), msg), true),
                Err(e) => print_heart_yellow(&format!("{} Failed to save: {}", "❌".bright_red(), e), true),
            }
            continue;
        }

        if line.starts_with("/load ") {
            let file_path = line[6..].trim();
            match chat.load_state(file_path) {
                Ok(msg) => print_heart_red(&format!("{} {}", "📂".bright_green(), msg), true),
                Err(e) => print_heart_yellow(&format!("{} Failed to load: {}", "❌".bright_red(), e), true),
            }
            continue;
        }

        // Handle /model command
        if line == "/model" || line.starts_with("/model ") {
            if line == "/model" {
                // Just display current model
                print_heart_red(&format!("{} Current model: {}", "🤖".bright_cyan(), chat.current_model.display_name()), true);
            } else {
                // Parse model argument
                let model_arg = line[7..].trim(); // Remove "/model " prefix
                
                if model_arg.is_empty() {
                    print_heart_red(&format!("{} Current model: {}", "🤖".bright_cyan(), chat.current_model.display_name()), true);
                    continue;
                }
                
                if model_arg == "help" || model_arg == "--help" || model_arg == "-h" {
                    print_heart_red(&format!("{} Model switching commands:", "🤖".bright_cyan()), true);
                    print_heart_red(&format!("  /model              - Show current model"), true);
                    print_heart_red(&format!("  /model <color>      - Switch to model by color"), true);
                    print_heart_red(&format!("  Available colors: blu, grn, red"), true);
                    print_heart_red(&format!("  Example: /model blu"), true);
                    continue;
                }
                
                // Map color arguments to actual model names
                let model_str = match model_arg.to_lowercase().as_str() {
                    "blu" | "blue" => "blu_model",
                    "grn" | "green" => "grn_model", 
                    "red" => "red_model",
                    _ => {
                        print_heart_yellow(&format!("{} Invalid model color: '{}'. Available: blu, grn, red", "❌".bright_red(), model_arg), true);
                        continue;
                    }
                };
                
                // Switch model with appropriate reason
                let reason = format!("User requested switch to {} model", model_arg);
                match chat.switch_model(model_str, &reason) {
                    Ok(msg) => {
                        print_heart_red(&format!("{} {}", "✓".bright_green(), msg), true);
                        print_heart_red(&format!("{} Current model: {}", "🤖".bright_cyan(), chat.current_model.display_name()), true);

                        // Update shared model state for background input task
                        {
                            let mut model_guard = current_model_for_main.write().unwrap();
                            *model_guard = chat.current_model;
                        }
                    }
                    Err(e) => {
                        print_heart_yellow(&format!("{} Failed to switch model: {}", "❌".bright_red(), e), true);
                    }
                }
            }
            continue;
        }

        // Handle /history command
        if line == "/history" {
            print_heart_red(&format!("{}", "📜 Conversation History:".bright_cyan()), true);
            print_heart_red(&format!("{}", "═".repeat(80).bright_black()), true);

            for (i, msg) in chat.messages.iter().enumerate() {
                let role_label = match msg.role.as_str() {
                    "system" => "SYS".bright_magenta(),
                    "user" => "USR".bright_green(),
                    "assistant" => "AST".bright_blue(),
                    "tool" => "TL ".bright_yellow(),
                    _ => "???".bright_red(),
                };

                // Check if assistant message has tool calls
                let tool_indicator = if msg.role == "assistant" {
                    if let Some(ref tool_calls) = msg.tool_calls {
                        if !tool_calls.is_empty() {
                            let tool_names: Vec<_> = tool_calls.iter()
                                .map(|tc| tc.function.name.as_str())
                                .collect();
                            format!(" 🔧[{}]", tool_names.join(", ")).bright_yellow().to_string()
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                } else if msg.role == "tool" {
                    // Show tool call ID for tool messages
                    if let Some(ref tool_call_id) = msg.tool_call_id {
                        format!(" (id: {})", &tool_call_id[..tool_call_id.len().min(8)]).bright_black().to_string()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                // Truncate content to 80 chars to leave room for tool indicator
                let content_preview = if msg.content.len() > 80 {
                    format!("{}...", &msg.content[..77])
                } else {
                    msg.content.clone()
                };

                // Replace newlines with spaces for single-line display
                let content_preview = content_preview.replace('\n', " ");

                print_heart_red(&format!("{:3}. [{}]{} {}", i, role_label, tool_indicator, content_preview.bright_black()), true);
            }

            print_heart_red(&format!("{}", "═".repeat(80).bright_black()), true);
            print_heart_red(&format!("{} Total messages: {}", "📊".bright_cyan(), chat.messages.len()), true);
            continue;
        }

        // Handle /debug command
        if line == "/debug" {
            print_heart_red(&format!("{} Debug level: {} (binary: {:b})", "🔧".bright_cyan(), chat.get_debug_level(), chat.get_debug_level()), true);
            print_heart_red(&format!("{} Usage: /debug <level>", "💡".bright_yellow()), true);
            print_heart_red(&format!("  0 = off"), true);
            print_heart_red(&format!("  1 = basic (bit 0)"), true);
            print_heart_red(&format!("  2 = detailed (bit 1)"), true);
            print_heart_red(&format!("  4 = verbose (bit 2)"), true);
            print_heart_red(&format!("  Example: /debug 3 (enables basic + detailed)"), true);
            continue;
        }

        if line.starts_with("/debug ") {
            let level_str = line[7..].trim();
            match level_str.parse::<u32>() {
                Ok(level) => {
                    chat.set_debug_level(level);
                    print_heart_red(&format!("{} Debug level set to {} (binary: {:b})", "🔧".bright_green(), level, level), true);
                }
                Err(_) => {
                    print_heart_yellow(&format!("{} Invalid debug level: '{}'. Use a number like 0, 1, 3, 7, etc.", "❌".bright_red(), level_str), true);
                }
            }
            continue;
        }

        // Handle /session commands
        if line == "/session" || line == "/session help" {
            print_heart_red(&format!("{} Session commands:", "🖥️".bright_cyan()), true);
            print_heart_red(&format!("  /session list           - List all terminal sessions"), true);
            print_heart_red(&format!("  /session show <id>      - Show screen buffer of session"), true);
            print_heart_red(&format!("  /session help           - Show this help"), true);
            continue;
        }

        if line == "/session list" {
            let manager = chat.terminal_manager.lock().await;
            match manager.list_sessions().await {
                Ok(sessions) => {
                    if sessions.is_empty() {
                        print_heart_red(&format!("{} No active terminal sessions", "ℹ️".bright_blue()), true);
                    } else {
                        print_heart_red(&format!("{} Active terminal sessions:", "🖥️".bright_cyan()), true);
                        for session in sessions {
                            let status_icon = if session.status.contains("Running") {
                                "▶️"
                            } else if session.status.contains("Exited") {
                                "⏹️"
                            } else {
                                "⏸️"
                            };
                            print_heart_red(&format!("  {} Session {}: {} ({}x{}) - {}",
                                status_icon,
                                session.id,
                                session.command,
                                session.cols,
                                session.rows,
                                session.status
                            ), true);
                        }
                    }
                }
                Err(e) => {
                    print_heart_yellow(&format!("{} Failed to list sessions: {}", "❌".bright_red(), e), true);
                }
            }
            continue;
        }

        if line.starts_with("/session show ") {
            let session_id = line[14..].trim();
            let manager = chat.terminal_manager.lock().await;
            match manager.get_screen(session_id, false, true).await {
                Ok(screen_contents) => {
                    // Get session info for size
                    let width = if let Ok(sessions) = manager.list_sessions().await {
                        sessions.iter()
                            .find(|s| s.id == session_id)
                            .map(|s| s.cols as usize)
                            .unwrap_or(80)
                    } else {
                        80
                    };

                    print_heart_red(&format!("{} Screen contents of session {}:", "📺".bright_cyan(), session_id), true);
                    print_heart_red(&format!("┌{}┐", "─".repeat(width)), true);
                    print_heart_red(&format!("{}", screen_contents), true);
                    print_heart_red(&format!("└{}┘", "─".repeat(width)), true);
                }
                Err(e) => {
                    print_heart_yellow(&format!("{} Failed to get screen for session '{}': {}", "❌".bright_red(), session_id, e), true);
                }
            }
            continue;
        }

        // Handle /skills command to show available skill commands
        if line == "/skills" || line == "/skills help" {
            print_heart_red(&format!("{} Available Commands:", "🎯".bright_cyan()), true);
            print_heart_red(&format!("  /model [color]          - Show current model or switch to model by color (blu/grn/red)"), true);
            print_heart_red(&format!("  /history                - Display conversation history with message roles"), true);
            print_heart_red(&format!("  /brainstorm             - Use brainstorming skill for interactive design refinement"), true);
            print_heart_red(&format!("  /write-plan             - Use writing-plans skill to create detailed implementation plan"), true);
            print_heart_red(&format!("  /execute-plan           - Use executing-plans skill to execute plan with checkpoints"), true);
            print_heart_red(&format!("  /compact               - Force immediate conversation compaction to reduce session size"), true);
            print_heart_red(&format!("  /confirm                - Toggle auto-confirm mode (enable/disable confirmation prompts)"), true);
            print_heart_red(&format!("  /skills help            - Show this help"), true);
            continue;
        }

        // Handle /brainstorm command
        if line == "/brainstorm" {
            if let Some(ref skill_registry) = chat.skill_registry {
                match skill_registry.get_skill("brainstorming") {
                    Some(skill) => {
                        let skill_msg = Message {
                            role: "system".to_string(),
                            content: format!(
                                "<skill_invocation>\n🎯 USING SKILL: {}\n\n{}\n\n**YOU MUST follow this skill exactly as written.**\n</skill_invocation>",
                                skill.name, skill.content
                            ),
                            tool_calls: None,
                            tool_call_id: None,
                            name: None,
                            reasoning: None,
                        };
                        chat.messages.push(skill_msg.clone());

                        if let Some(logger) = &mut chat.logger {
                            logger.log("system", &skill_msg.content, None, false).await;
                        }

                        print_heart_red(&format!("{} {} Brainstorming skill activated! 🎯", "✓".bright_green(), "Skill:".bright_cyan()), true);
                        print_heart_red(&format!("{}", "Ask your question or describe what you want to brainstorm about.".bright_black()), true);
                    }
                    None => {
                        print_heart_yellow(&format!("{} Brainstorming skill not found. Ensure skills/ directory contains brainstorming/SKILL.md", "❌".bright_red()), true);
                    }
                }
            } else {
                print_heart_yellow(&format!("{} Skill registry not available", "❌".bright_red()), true);
            }
            continue;
        }

        // Handle /write-plan command
        if line == "/write-plan" {
            if let Some(ref skill_registry) = chat.skill_registry {
                match skill_registry.get_skill("writing-plans") {
                    Some(skill) => {
                        let skill_msg = Message {
                            role: "system".to_string(),
                            content: format!(
                                "<skill_invocation>\n🎯 USING SKILL: {}\n\n{}\n\n**YOU MUST follow this skill exactly as written.**\n</skill_invocation>",
                                skill.name, skill.content
                            ),
                            tool_calls: None,
                            tool_call_id: None,
                            name: None,
                            reasoning: None,
                        };
                        chat.messages.push(skill_msg.clone());

                        if let Some(logger) = &mut chat.logger {
                            logger.log("system", &skill_msg.content, None, false).await;
                        }

                        print_heart_red(&format!("{} {} Writing-plans skill activated! 📋", "✓".bright_green(), "Skill:".bright_cyan()), true);
                        print_heart_red(&format!("{}", "Describe what you want to plan and I'll create a detailed implementation plan.".bright_black()), true);
                    }
                    None => {
                        print_heart_yellow(&format!("{} Writing-plans skill not found. Ensure skills/ directory contains writing-plans/SKILL.md", "❌".bright_red()), true);
                    }
                }
            } else {
                print_heart_yellow(&format!("{} Skill registry not available", "❌".bright_red()), true);
            }
            continue;
        }

        // Handle /execute-plan command
        if line == "/execute-plan" {
            if let Some(ref skill_registry) = chat.skill_registry {
                match skill_registry.get_skill("executing-plans") {
                    Some(skill) => {
                        let skill_msg = Message {
                            role: "system".to_string(),
                            content: format!(
                                "<skill_invocation>\n🎯 USING SKILL: {}\n\n{}\n\n**YOU MUST follow this skill exactly as written.**\n</skill_invocation>",
                                skill.name, skill.content
                            ),
                            tool_calls: None,
                            tool_call_id: None,
                            name: None,
                            reasoning: None,
                        };
                        chat.messages.push(skill_msg.clone());

                        if let Some(logger) = &mut chat.logger {
                            logger.log("system", &skill_msg.content, None, false).await;
                        }

                        print_heart_red(&format!("{} {} Executing-plans skill activated! 🚀", "✓".bright_green(), "Skill:".bright_cyan()), true);
                        print_heart_red(&format!("{}", "I'll execute the plan in batches with review checkpoints.".bright_black()), true);
                    }
                    None => {
                        print_heart_yellow(&format!("{} Executing-plans skill not found. Ensure skills/ directory contains executing-plans/SKILL.md", "❌".bright_red()), true);
                    }
                }
            } else {
                print_heart_yellow(&format!("{} Skill registry not available", "❌".bright_red()), true);
            }
            continue;
        }

        // Handle /compact command
        if line == "/compact" {
            print_heart_red(&format!("{} Starting manual conversation compaction...", "🗜️".bright_blue()), true);
            match intelligent_compaction(&mut chat, 0).await {
                Ok(()) => {
                    let session_size = crate::chat::history::calculate_conversation_size(&chat.messages);
                    print_heart_red(&format!("{} Compaction completed successfully!", "✓".bright_green()), true);
                    print_heart_red(&format!("{} Session size: {:.1} KB, Messages: {}", "📊".bright_cyan(),
                             session_size as f64 / 1024.0, chat.messages.len()), true);
                }
                Err(e) => {
                    print_heart_yellow(&format!("{} Failed to compact conversation: {}", "❌".bright_red(), e), true);
                }
            }
            continue;
        }

        // Handle /confirm command
        if line == "/confirm" {
            // Check if we're in auto-confirm mode
            let is_auto_confirm = chat.policy_manager.is_allow_all();
            
            // Toggle the mode
            if is_auto_confirm {
                // Switch to regular policy manager that asks for confirmation
                chat.policy_manager = PolicyManager::new();
                print_heart_red(&format!("{} Auto-confirm mode disabled. Actions will now require confirmation.", "✓".bright_green()), true);
            } else {
                // Switch to allow-all policy manager for auto-confirm
                chat.policy_manager = PolicyManager::allow_all();
                print_heart_red(&format!("{} Auto-confirm mode enabled. All actions will be approved automatically.", "✓".bright_green()), true);
            }
            
            // Print current state
            let current_state = chat.policy_manager.is_allow_all();
            print_heart_red(&format!("{} Auto-confirm: {}", "📋".bright_cyan(), if current_state {"enabled"} else {"disabled"}), true);
            continue;
        }

        // Save to persistent history file (in-memory history already added in background task)
        match apchat_vty::history::save_to_file(&
            apchat_vty::history::ReadlineEntry::with_session(
                line,
                format!("session_{}", chat.process_id)
            )
        ) {
            Ok(_) => {
                        if chat.debug_level > 2 {
                            print_heart_red(&format!("{} Saved to readline history", "✏️".bright_blue()), true);
                        }
                    }
            Err(e) => {
                        if chat.debug_level > 0 {
                            print_heart_yellow(&format!("{} Failed to save readline history: {}", "⚠️".yellow(), e), true);
                        }
                    }
        }

	// Log the user message before sending
	if let Some(logger) = &mut chat.logger {
	    logger.log("user", line, None, false).await;
	}

	// Auto-save history after user message
	match chat.auto_save_history() {
	    Ok(_) => {
		// Silently save in background (no output to user)
		if chat.debug_level > 1 {
		    print_heart_red(&format!("{} Auto-saved history to history-{}.json", 
			     "💾".bright_blue(), chat.process_id), true);
		}
	    }
	    Err(e) => {
		if chat.debug_level > 0 {
		    print_heart_yellow(&format!("{} Auto-save failed: {}", "⚠️".yellow(), e), true);
		}
	    }
	}

	// Create cancellation token for this operation
	let cancel_token = tokio_util::sync::CancellationToken::new();

	// Register this token with the persistent Ctrl-C handler
	{
	    let mut guard = current_token.lock().unwrap();
	    *guard = Some(cancel_token.clone());
	}

	// Use tokio::select! to race inference against interrupt signals
	let response = if chat.use_agents && chat.agent_coordinator.is_some() {
	    // Agent inference with interrupt handling
	    'agent_loop: loop {
		tokio::select! {
		    result = chat.process_with_agents(line, Some(cancel_token.clone())) => {
			match result {
			    Ok(response) => {
				break 'agent_loop response;
			    }
			    Err(e) if e.to_string().contains("cancelled") || e.to_string().contains("interrupted") => {
				// This shouldn't happen since interrupts are handled in select! branch
				// But if it does, treat it like an error
				print_heart_yellow(&format!("{} Unexpected interruption: {}", "⚠️".yellow(), e), true);
				{
				    let mut guard = current_token.lock().unwrap();
				    *guard = None;
				}
				chat.messages.push(Message {
				    role: "assistant".to_string(),
				    content: format!("[Interrupted: {}]", e),
				    tool_calls: None,
				    tool_call_id: None,
				    name: None,
				    reasoning: None,
				});
				continue 'outer;
			    }
			    Err(e) => {
				print_heart_yellow(&format!("{} {}\n", "Error:".bright_red().bold(), e), true);
				{
				    let mut guard = current_token.lock().unwrap();
				    *guard = None;
				}
				// Add assistant message to maintain turn alternation
				chat.messages.push(Message {
				    role: "assistant".to_string(),
				    content: format!("[Error: {}]", e),
				    tool_calls: None,
				    tool_call_id: None,
				    name: None,
				    reasoning: None,
				});
				continue 'outer;
			    }
			}
		    }
		    interrupt_msg = mspc_channel.recv() => {
			if let Some(msg) = interrupt_msg {
			    match msg {
				MspcMessage::InterruptSignal(_content, _sender) => {
				    print_heart_red(&format!("\n{}", "^C - Interrupting current operation...".bright_yellow()), true);
				    cancel_token.cancel();
				    {
					let mut guard = current_token.lock().unwrap();
					*guard = None;
				    }
				    // Add assistant message to maintain turn alternation
				    chat.messages.push(Message {
					role: "assistant".to_string(),
					content: "[Interrupted by user]".to_string(),
					tool_calls: None,
					tool_call_id: None,
					name: None,
					reasoning: None,
				    });
				    continue 'outer;
				}
				_ => {
				    // Non-interrupt message - ignore during inference
				}
			    }
			} else {
			    break 'agent_loop String::new();
			}
		    }
		}
	    }
	} else {
	    // Regular chat inference with interrupt handling
	    'chat_loop: loop {
		tokio::select! {
		    result = crate::chat::session::chat(&mut chat, line, Some(cancel_token.clone())) => {
			match result {
			    Ok(response) => {
				break 'chat_loop response;
			    }
			    Err(e) if e.to_string().contains("interrupted") => {
				// This shouldn't happen since interrupts are handled in select! branch
				// But if it does, treat it like an error
				print_heart_yellow(&format!("{} Unexpected interruption: {}", "⚠️".yellow(), e), true);
				{
				    let mut guard = current_token.lock().unwrap();
				    *guard = None;
				}
				chat.messages.push(Message {
				    role: "assistant".to_string(),
				    content: format!("[Interrupted: {}]", e),
				    tool_calls: None,
				    tool_call_id: None,
				    name: None,
				    reasoning: None,
				});
				continue 'outer;
			    }
			    Err(e) => {
				print_heart_yellow(&format!("{} {}\n", "Error:".bright_red().bold(), e), true);
				{
				    let mut guard = current_token.lock().unwrap();
				    *guard = None;
				}
				// Add assistant message to maintain turn alternation
				chat.messages.push(Message {
				    role: "assistant".to_string(),
				    content: format!("[Error: {}]", e),
				    tool_calls: None,
				    tool_call_id: None,
				    name: None,
				    reasoning: None,
				});
				continue 'outer;
			    }
			}
		    }
		    interrupt_msg = mspc_channel.recv() => {
eprintln!("AYXX: INTERRUPT: {:?}", &interrupt_msg);
			if let Some(msg) = interrupt_msg {
			    match msg {
				MspcMessage::InterruptSignal(_content, _sender) => {
				    print_heart_red(&format!("\n{}", "^C - Interrupting current operation...".bright_yellow()), true);
				    cancel_token.cancel();
				    {
					let mut guard = current_token.lock().unwrap();
					*guard = None;
				    }
				    // Add assistant message to maintain turn alternation
				    chat.messages.push(Message {
					role: "assistant".to_string(),
					content: "[Interrupted by user]".to_string(),
					tool_calls: None,
					tool_call_id: None,
					name: None,
					reasoning: None,
				    });
				    continue 'outer;
				}
				_ => {
				    // Non-interrupt message - ignore during inference
				}
			    }
			} else {
			    break 'chat_loop String::new();
			}
		    }
		}
	    }
	};

	// Clear the current token after operation completes
	{
	    let mut guard = current_token.lock().unwrap();
	    *guard = None;
	}

	// Log assistant response
	if let Some(logger) = &mut chat.logger {
	    logger.log("assistant", &response, None, false).await;
	}

	// Broadcast response to Webex if enabled
	if let Some(ref webex) = webex_sink {
	    if let Err(e) = webex.send_response(&response).await {
		print_heart_yellow(&format!("{} Failed to send to Webex: {}", "⚠️".yellow(), e), true);
	    }
	}

	// Display response if not streaming (streaming already displayed it)
	if !chat.stream_responses {
	    let model_name = get_model_name_for_prompt(&chat.current_model, &chat.client_config);
	    let model_label = format!("[{} ({})]", chat.current_model.display_name(), model_name).bright_magenta();
	    let assistant_label = "Assistant:".bright_blue().bold();
	    print_heart_red(&format!("\n{} {} {}\n", model_label, assistant_label, response), true);
	} else {
	    // Add extra newline after streaming to separate from next prompt
	    print_heart_red(&format!(""), true);
	}
    }

    // Abort terminal input router on exit
    router_handle.abort();
    
    // Graceful shutdown of logger (flush & close)
    if let Some(logger) = &mut chat.logger {
        logger.shutdown().await;
    }

    // Cleanup readline instance (save history and release resources)
    if let Err(e) = apchat_vty::ReadlineInstance::cleanup() {
        if chat.debug_level > 0 {
            print_heart_yellow(&format!("{} Failed to cleanup readline instance: {}", "⚠️".yellow(), e), true);
        }
    }

    Ok(())
}

#[cfg(test)]
mod repl_compact_tests {
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
            readline_history: None,
            content_limiter: None,
            mspc_channel: None,
            signal_sender: None,
            signal_receiver: None,
            confirmation_registry: None,
        }
    }

    #[tokio::test]
    async fn test_compact_command_exists() {
        // This test verifies that the /compact command is properly handled
        // and doesn't cause crashes when executed
        
        let mut chat = create_test_chat().await;
        
        // Add some messages to make compaction meaningful
        chat.messages.push(Message {
            role: "user".to_string(),
            content: "Test message for compaction".repeat(100),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning: None,
        });
        
        // The compact command should not crash and should preserve system messages
        let result = crate::chat::history::intelligent_compaction(&mut chat, 0).await;
        
        assert!(result.is_ok(), "Compaction should succeed");
        
        // Should still have at least the system message
        assert!(chat.messages.len() >= 1, "Should have at least system message after compaction");
    }
}
