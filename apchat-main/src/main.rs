// Main entry point for APChat binary
use anyhow::Result;
use clap::Parser;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

// Import from library
use apchat::{APChat, resolve_terminal_backend};
use apchat::cli::{Cli, Commands};
use apchat::app::{setup_from_cli, run_task_mode, run_subagent_mode, run_repl_mode};
use apchat_terminal::{TerminalManager, MAX_CONCURRENT_SESSIONS};
use apchat_logging;

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
            return run_subagent_mode(
                &cli,
                task_text,
                app_config.client_config,
                app_config.work_dir,
                app_config.policy_manager,
            )
            .await;
        } else {
            // Use regular task mode for multi-agent system (when --agents IS specified)
            return run_task_mode(
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
        return apchat::app::run_web_server(
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
