use anyhow::Result;
use colored::Colorize;

use crate::APChat;
use apchat_vty::{print_heart_red, print_heart_yellow};
use crate::cli::Cli;
use crate::config::ClientConfig;
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

    if cli.agents {
        print_heart_red(&format!("{}", "🚀 Multi-Agent System ENABLED".green().bold()), true);
    }

    print_heart_red(&format!("{}", format!("Task: {}", task_text).bright_yellow()), true);
    print_heart_red("", true);

    // Resolve terminal backend
    let backend_type = crate::resolve_terminal_backend(cli)?;

    let mut chat = APChat::new_with_config(
        client_config.clone(),
        work_dir.clone(),
        cli.agents,
        policy_manager.clone(),
        cli.stream,
        cli.verbose,
        backend_type,
        cli.early_superpowers,
    );

    // Initialize logger for task mode
    chat.logger = match ConversationLogger::new_task_mode(&chat.work_dir).await {
        Ok(l) => Some(l),
        Err(e) => {
            print_heart_yellow(&format!("Task logging disabled: {}", e), true);
            None
        }
    };

    let response = if chat.use_agents && chat.agent_coordinator.is_some() {
        // Use agent system
        match chat.process_with_agents(&task_text, None).await {
            Ok(response) => response,
            Err(e) => {
                print_heart_yellow(&format!("{} {}\n", "Agent Error:".bright_red().bold(), e), true);
                // Fallback to regular chat (no cancellation in task mode)
                match crate::chat::session::chat(&mut chat, &task_text, None).await {
                    Ok(response) => response,
                    Err(e) => {
                        print_heart_yellow(&format!("{} {}\n", "Error:".bright_red().bold(), e), true);
                        return Ok(());
                    }
                }
            }
        }
    } else {
        // Use regular chat (no cancellation in task mode)
        match crate::chat::session::chat(&mut chat, &task_text, None).await {
            Ok(response) => response,
            Err(e) => {
                print_heart_yellow(&format!("{} {}\n", "Error:".bright_red().bold(), e), true);
                return Ok(());
            }
        }
    };

    if cli.pretty {
        print_heart_red(
            &serde_json::to_string_pretty(&serde_json::json!({
                "response": response,
                "agents_used": chat.use_agents
            }))
            .unwrap_or_else(|_| response.to_string()),
            true
        );
    } else {
        print_heart_red(&response, true);
    }

    Ok(())
}
