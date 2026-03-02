use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;

use apchat_vty::{print_heart_yellow, print_heart_red, status_info, IdleConfig};
use apchat_logging::ConversationLogger;
use apchat_models::{ModelColor, Message};
use apchat_policy::PolicyManager;

use crate::APChat;
use crate::cli::Cli;
use crate::config::{ClientConfig, FeatureFlags};

/// Initialize the REPL session.
///
/// Displays the banner, creates APChat, shows model configuration,
/// sets up logging, runs the session-start hook, loads readline history,
/// validates idle config, and reads project context.
pub async fn initialize_repl(
    cli: &Cli,
    client_config: ClientConfig,
    work_dir: PathBuf,
    policy_manager: PolicyManager,
) -> Result<(APChat, Option<IdleConfig>)> {
    print_heart_red(&format!("{}", "🤖 APChat - Claude Code-like Experience".bright_cyan().bold()), true);
    print_heart_red(&format!("{}", format!("Working directory: {}", work_dir.display()).bright_black()), true);
    print_heart_red(&format!("{}", "Type 'exit' or 'quit' to exit, '/model' to switch models, '/history' to view history, or '/skills' for all commands\n".bright_black()), true);

    // Resolve terminal backend and create APChat
    let backend_type = crate::resolve_terminal_backend(cli)?;

    let flags = FeatureFlags {
        early_superpowers: cli.early_superpowers,
        delayed_instructions: cli.delayed_instructions,
        metacog_tools: cli.metacog_tools,
        python_sandbox: cli.python_sandbox,
        self_regulate: cli.self_regulate,
        learning_opportunities: cli.learning_opportunities,
        community_skills: cli.community_skills,
        tiling_tree: cli.tiling_tree,
        convening_experts: cli.convening_experts,
        crafting_instructions: cli.crafting_instructions,
        reviewing_ai_papers: cli.reviewing_ai_papers,
        elements_of_style: cli.elements_of_style,
        self_edit: cli.self_edit,
        diff_fuzz: cli.diff_fuzz,
        forecasting: cli.forecasting,
        context_mode: cli.context_mode,
        financial_services: cli.financial_services,
        mcp_servers: cli.mcp_server.clone(),
        searxng_url: cli.searxng.clone(),
    };

    let mut chat = APChat::new_with_config(
        client_config,
        work_dir,
        policy_manager,
        cli.stream,
        cli.verbose,
        backend_type,
        flags.clone(),
    );

    // Register MCP server tools (async initialization)
    chat.register_mcp_tools(&flags).await;

    // Set summarize_subagents flag from CLI
    chat.summarize_subagents = !cli.no_summarize_subagents;

    // Set process ID for status info
    status_info::set_pid(std::process::id().try_into().unwrap());

    print_model_config(&chat);

    // Initialize logger
    chat.logger = match ConversationLogger::new(&chat.work_dir).await {
        Ok(l) => Some(l),
        Err(e) => {
            print_heart_yellow(&format!("Logging disabled: {}", e), true);
            None
        }
    };

    // Log initial system message that APChat::new added
    if let Some(logger) = &mut chat.logger {
        if let Some(sys_msg) = chat.messages.first() {
            logger
                .log("system", &sys_msg.content, None, false)
                .await;
        }
    }

    run_session_start_hook(&mut chat, cli.verbose);

    load_readline_history()?;

    let idle_config = parse_idle_config(cli)?;

    load_project_context(&mut chat).await;

    Ok((chat, idle_config))
}

fn print_model_config(chat: &APChat) {
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
        if let Some(key_preview) = get_api_key_preview(model_color, &chat.client_config) {
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
}

fn get_api_key_preview(model_color: ModelColor, client_config: &ClientConfig) -> Option<colored::ColoredString> {
    let key_preview = match model_color {
        ModelColor::BluModel => {
            if let Some(key) = client_config.get_api_key(ModelColor::BluModel) {
                Some(format!("{}***", &key[..key.len().min(3)]))
            } else if !client_config.api_key.is_empty() {
                Some(format!("{}***", &client_config.api_key[..client_config.api_key.len().min(3)]))
            } else {
                None
            }
        }
        ModelColor::GrnModel => {
            if let Some(key) = client_config.get_api_key(ModelColor::GrnModel) {
                Some(format!("{}***", &key[..key.len().min(3)]))
            } else if !client_config.api_key.is_empty() {
                Some(format!("{}***", &client_config.api_key[..client_config.api_key.len().min(3)]))
            } else {
                None
            }
        }
        ModelColor::RedModel => {
            if let Some(key) = client_config.get_api_key(ModelColor::RedModel) {
                Some(format!("{}***", &key[..key.len().min(3)]))
            } else if !client_config.api_key.is_empty() {
                Some(format!("{}***", &client_config.api_key[..client_config.api_key.len().min(3)]))
            } else {
                None
            }
        }
    };

    key_preview.map(|k| k.green())
}

fn run_session_start_hook(chat: &mut APChat, verbose: bool) {
    use std::process::Command;

    let hook_path = chat.work_dir.join("hooks/session-start.sh");
    if !hook_path.exists() {
        return;
    }

    match Command::new(&hook_path)
        .arg(chat.work_dir.to_string_lossy().to_string())
        .output()
    {
        Ok(output) if output.status.success() => {
            let hook_content = String::from_utf8_lossy(&output.stdout).to_string();
            if !hook_content.trim().is_empty() {
                chat.messages.push(Message {
                    role: "system".to_string(),
                    content: hook_content,
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                    reasoning: None,
                });

                if verbose {
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

fn load_readline_history() -> Result<()> {
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
    Ok(())
}

fn parse_idle_config(cli: &Cli) -> Result<Option<IdleConfig>> {
    if let (Some(timeout_secs), Some(input_text)) = (&cli.idle_timeout, &cli.idle_input) {
        if *timeout_secs < 1 || *timeout_secs > 86400 {
            anyhow::bail!("Idle timeout must be between 1 and 86400 seconds");
        }
        print_heart_red(&format!("{} Idle timeout enabled: {} seconds -> \"{}\"",
            "⏱️".bright_yellow(),
            timeout_secs,
            input_text.bright_cyan()
        ), true);
        Ok(Some(IdleConfig {
            timeout_secs: *timeout_secs,
            input_text: input_text.clone(),
        }))
    } else if cli.idle_timeout.is_some() || cli.idle_input.is_some() {
        anyhow::bail!("Both --idle-timeout and --idle-input must be specified together");
    } else {
        Ok(None)
    }
}

async fn load_project_context(chat: &mut APChat) {
    let kimi_context = if let Ok(kimi_content) = chat.peek_file_top_10_lines("apchat.md") {
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
        if let Some(logger) = &mut chat.logger {
            logger
                .log("system", &sys_msg.content, None, false)
                .await;
        }
        chat.messages.push(sys_msg);
    }
}
