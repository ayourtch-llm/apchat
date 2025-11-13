use anyhow::Result;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::env;
use std::path::PathBuf;

use crate::KimiChat;
use crate::cli::Cli;
use crate::config::ClientConfig;
use crate::policy::PolicyManager;
use crate::logging::ConversationLogger;
use crate::models::{ModelType, Message};

/// Run interactive REPL mode
pub async fn run_repl_mode(
    cli: &Cli,
    client_config: ClientConfig,
    work_dir: PathBuf,
    policy_manager: PolicyManager,
) -> Result<()> {
    println!("{}", "🤖 Kimi Chat - Claude Code-like Experience".bright_cyan().bold());
    println!("{}", format!("Working directory: {}", work_dir.display()).bright_black());

    if cli.agents {
        println!("{}", "🚀 Multi-Agent System ENABLED - Specialized agents will handle your tasks".green().bold());
    }

    println!("{}", "Type 'exit' or 'quit' to exit\n".bright_black());

    let mut chat = KimiChat::new_with_config(
        client_config,
        work_dir,
        cli.agents,
        policy_manager,
        cli.stream,
        cli.verbose,
    );

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
