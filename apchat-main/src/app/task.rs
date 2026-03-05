use anyhow::Result;
use colored::Colorize;

use crate::APChat;
use apchat_vty::{print_heart_red, print_heart_yellow};
use crate::cli::Cli;
use crate::config::{ClientConfig, FeatureFlags};
use apchat_policy::PolicyManager;
use apchat_logging::ConversationLogger;
use std::path::PathBuf;

/// Run in task mode - execute a single task and exit
pub async fn run_task_mode(
    cli: &Cli,
    task_text: String,
    client_config: ClientConfig,
    work_dir: PathBuf,
    policy_manager: PolicyManager,
) -> Result<()> {
    print_heart_red(&format!("{}", "🤖 APChat - Task Mode".bright_cyan().bold()), true);
    print_heart_red(&format!("{}", format!("Working directory: {}", work_dir.display()).bright_black()), true);
    print_heart_red(&format!("{}", format!("Task: {}", task_text).bright_yellow()), true);
    print_heart_red("", true);

    // Resolve terminal backend
    let backend_type = crate::resolve_terminal_backend(cli)?;

    let flags = FeatureFlags {
        early_superpowers: cli.early_superpowers,
        context_mode: cli.context_mode,
        mcp_servers: cli.mcp_server.clone(),
        searxng_url: cli.searxng.clone(),
        image_processing: cli.image_processing,
        ..FeatureFlags::default()
    };

    let mut chat = APChat::new_with_config(
        client_config.clone(),
        work_dir.clone(),
        policy_manager.clone(),
        cli.stream,
        cli.verbose,
        backend_type,
        flags.clone(),
    );

    // Set bogus_ack_msg from CLI flag
    if let Some(bogus_ack) = &cli.bogus_ack_msg {
        chat.bogus_ack_msg = Some(bogus_ack.clone());
    }

    // Register MCP server tools (async initialization)
    chat.register_mcp_tools(&flags).await;

    // Set summarize_subagents flag from CLI
    chat.summarize_subagents = !cli.no_summarize_subagents;

    // Initialize logger for task mode
    chat.logger = match ConversationLogger::new_task_mode(&chat.work_dir).await {
        Ok(l) => Some(l),
        Err(e) => {
            print_heart_yellow(&format!("Task logging disabled: {}", e), true);
            None
        }
    };

    // Use regular chat (no cancellation in task mode)
    let response = match crate::chat::session::chat(&mut chat, &task_text, None).await {
        Ok(response) => response,
        Err(e) => {
            print_heart_yellow(&format!("{} {}\n", "Error:".bright_red().bold(), e), true);
            return Ok(());
        }
    };

    if cli.pretty {
        print_heart_red(
            &serde_json::to_string_pretty(&serde_json::json!({
                "response": response
            }))
            .unwrap_or_else(|_| response.to_string()),
            true
        );
    } else {
        print_heart_red(&response, true);
    }

    if let Some(logger) = &mut chat.logger {
        logger.shutdown().await;
    }

    Ok(())
}
