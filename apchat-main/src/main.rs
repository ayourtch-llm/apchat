// Main entry point for APChat binary
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

// Import from library
use apchat::{APChat, resolve_terminal_backend};
use apchat::cli::{Cli, Commands};
use apchat::app::{setup_from_cli, run_subagent_mode, run_repl_mode};
use apchat_terminal::{TerminalManager, MAX_CONCURRENT_SESSIONS};
use apchat_logging;
use apchat_vty::{print_heart_red, print_heart_yellow};
use apchat_toolcore;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file if it exists
    dotenvy::dotenv().ok();

    // Initialize all directories and print their locations
    apchat_common::ApChatPaths::init_all()
        .expect("Failed to initialize APChat directories");

    // Parse CLI arguments
    let cli = Cli::parse();
    let original_args: Vec<String> = env::args().collect();

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
        print_heart_red(&format!("{}", result), true);
        return Ok(());
    }

    // Set up application configuration from CLI
    let app_config = setup_from_cli(&cli)?;
    
    // Initialize OutputRouter for emoji-prefixed text routing
    // This connects print_with_emoji to the router via vty's TEXT_OUTPUT_TX
    let router = apchat::mspc::initialize_output_router().await;

    // Initialize SQL logger for tool parsing debugging
    let db_path = cli.sql_log_path
        .clone()
        .unwrap_or_else(|| "/tmp/tool_debug.db".to_string());
    
    if let Err(e) = apchat_toolcore::init_sql_logger(&db_path).await {
        eprintln!("Warning: Failed to initialize SQL logger: {}", e);
    } else {
        eprintln!("SQL logger initialized at: {}", db_path);
    }

    // Handle task mode if requested
    if let Some(task_text) = cli.task.clone() {
        return run_subagent_mode(
            &cli,
            task_text,
            app_config.client_config,
            app_config.work_dir,
            app_config.policy_manager,
        )
        .await;
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
        print_heart_red("No subcommand provided and interactive mode not requested. Exiting.", true);
        return Ok(());
    }

    router.set_readline_active(true);

    // Run REPL mode (with optional Webex integration)
    let (webex_sink, mspc_channel_opt) = if let Some(ref user_email) = cli.webex_bot {
        // Create shared MSPC channel for both terminal and Webex
        let mspc_channel = Arc::new(apchat::mspc::MspcChannel::new(100));

        // Load Webex secret
        match apchat_webex::load_webex_secret() {
            Ok(token) => {
                if cli.webex_websocket {
                    print_heart_red(&format!("{} Initializing Webex WebSocket bot for {}", "🌐".bright_cyan(), user_email), true);

                    // Initialize Webex WebSocket router
                    let reconnect_hours = if cli.webex_reconnect_hours == 0 {
                        None
                    } else {
                        Some(cli.webex_reconnect_hours)
                    };
                    match apchat_webex::WebexWebSocketRouter::new(
                        token.clone(),
                        user_email.clone(),
                        mspc_channel.clone(),
                        reconnect_hours,
                    ).await {
                        Ok(router) => {
                            let room_id = router.room_id().to_string();
                            let client = router.client();
                            let last_message_id = router.last_user_message_id();

                            // Spawn WebSocket router as background task
                            tokio::spawn(async move {
                                if let Err(e) = router.run().await {
                                    print_heart_yellow(&format!("⚠️ Webex WebSocket router error: {}", e), true);
                                }
                            });

                            let sink = Arc::new(apchat_webex::WebexOutputSink::new(client.clone(), room_id, last_message_id));
                            print_heart_red(&format!("{} Webex WebSocket bot ready - responses will be broadcast", "✓".bright_green()), true);
                            (Some(sink), Some((mspc_channel, client, user_email.clone())))
                        }
                        Err(e) => {
                            print_heart_yellow(&format!("{} Failed to initialize Webex WebSocket bot: {}", "⚠️".yellow(), e), true);
                            // Create a default WebexClient for tool registration
                            let default_client = apchat_webex::WebexClient::new(token.clone());
                            (None, Some((mspc_channel, std::sync::Arc::new(default_client), user_email.clone())))
                        }
                    }
                } else {
                    print_heart_red(&format!("{} Initializing Webex bot (polling mode) for {}", "🌐".bright_cyan(), user_email), true);

                    // Initialize Webex input router (polling mode)
                    match apchat_webex::WebexInputRouter::new(
                        token.clone(),
                        user_email.clone(),
                        mspc_channel.clone(),
                    ).await {
                        Ok(router) => {
                            let room_id = router.room_id().to_string();
                            let client = router.client();
                            let last_message_id = router.last_user_message_id();

                            // Spawn Webex input router as background task
                            tokio::spawn(async move {
                                if let Err(e) = router.run().await {
                                    print_heart_yellow(&format!("⚠️ Webex input router error: {}", e), true);
                                }
                            });

                            let sink = Arc::new(apchat_webex::WebexOutputSink::new(client.clone(), room_id, last_message_id));
                            print_heart_red(&format!("{} Webex bot ready (polling mode)", "✓".bright_green()), true);
                            (Some(sink), Some((mspc_channel, client, user_email.clone())))
                        }
                        Err(e) => {
                            print_heart_yellow(&format!("{} Failed to initialize Webex bot: {}", "⚠️".yellow(), e), true);
                            // Create a default WebexClient for tool registration
                            let default_client = apchat_webex::WebexClient::new(token.clone());
                            (None, Some((mspc_channel, std::sync::Arc::new(default_client), user_email.clone())))
                        }
                    }
                }
            }
            Err(e) => {
                print_heart_yellow(&format!("{} {}", "⚠️".yellow(), e), true);
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    run_repl_mode(
        &cli,
        app_config.client_config,
        app_config.work_dir,
        app_config.policy_manager,
        webex_sink,
        mspc_channel_opt,
    )
    .await
}
