use colored::Colorize;
use std::sync::Arc;

use apchat_vty::{print_heart_yellow, print_heart_red};
use apchat_models::{ModelColor, Message};
use apchat_models::types::ContentPart;
use apchat_policy::PolicyManager;

use crate::APChat;
use crate::mspc::MspcMessage;
use crate::chat::history::{intelligent_compaction, calculate_conversation_size};

/// Result of attempting to handle a slash command.
#[derive(Debug, PartialEq)]
pub enum CommandResult {
    /// Command was handled; continue to next loop iteration.
    Handled,
    /// Input was not a recognized command; proceed to inference.
    NotACommand,
}

/// Attempt to dispatch a slash command.
///
/// Returns `Handled` if the command was processed, or `NotACommand` if the
/// input should be sent to the LLM as a regular message.
pub async fn dispatch_command(
    chat: &mut APChat,
    line: &str,
    current_model_shared: &Arc<std::sync::RwLock<ModelColor>>,
) -> CommandResult {
    if line.starts_with("/save ")                                         { return cmd_save(chat, line); }
    if line.starts_with("/load ")                                         { return cmd_load(chat, line); }
    if line == "/model" || line.starts_with("/model ")                    { return cmd_model(chat, line, current_model_shared); }
    if line == "/history"                                                 { return cmd_history(chat); }
    if line == "/debug" || line.starts_with("/debug ")                    { return cmd_debug(chat, line); }
    if line == "/session" || line == "/session help"
        || line == "/session list" || line.starts_with("/session show ")  { return cmd_session(chat, line).await; }
    if line == "/skills" || line == "/skills help"                        { return cmd_skills(); }
    if line == "/brainstorm"                                              { return cmd_skill_activate(chat, "brainstorming",    "Brainstorming",    "🎯", "Ask your question or describe what you want to brainstorm about.").await; }
    if line == "/write-plan"                                              { return cmd_skill_activate(chat, "writing-plans",    "Writing-plans",    "📋", "Describe what you want to plan and I'll create a detailed implementation plan.").await; }
    if line == "/execute-plan"                                            { return cmd_skill_activate(chat, "executing-plans", "Executing-plans",  "🚀", "I'll execute the plan in batches with review checkpoints.").await; }
    if line == "/compact"                                                 { return cmd_compact(chat).await; }
    if line == "/confirm"                                                 { return cmd_confirm(chat); }
    if line.starts_with("/set model url")                                 { return cmd_set_model_url(chat, line, current_model_shared).await; }

    CommandResult::NotACommand
}

fn cmd_save(chat: &mut APChat, line: &str) -> CommandResult {
    let file_path = line[6..].trim();
    match chat.save_state(file_path) {
        Ok(msg) => print_heart_red(&format!("{} {}", "💾".bright_green(), msg), true),
        Err(e) => print_heart_yellow(&format!("{} Failed to save: {}", "❌".bright_red(), e), true),
    }
    CommandResult::Handled
}

fn cmd_load(chat: &mut APChat, line: &str) -> CommandResult {
    let file_path = line[6..].trim();
    match chat.load_state(file_path) {
        Ok(msg) => print_heart_red(&format!("{} {}", "📂".bright_green(), msg), true),
        Err(e) => print_heart_yellow(&format!("{} Failed to load: {}", "❌".bright_red(), e), true),
    }
    CommandResult::Handled
}

fn cmd_model(chat: &mut APChat, line: &str, current_model_shared: &Arc<std::sync::RwLock<ModelColor>>) -> CommandResult {
    if line == "/model" {
        print_heart_red(&format!("{} Current model: {}", "🤖".bright_cyan(), chat.current_model.display_name()), true);
        return CommandResult::Handled;
    }

    let model_arg = line[7..].trim(); // Remove "/model " prefix

    if model_arg.is_empty() {
        print_heart_red(&format!("{} Current model: {}", "🤖".bright_cyan(), chat.current_model.display_name()), true);
        return CommandResult::Handled;
    }

    if model_arg == "help" || model_arg == "--help" || model_arg == "-h" {
        print_heart_red(&format!("{} Model switching commands:", "🤖".bright_cyan()), true);
        print_heart_red(&format!("  /model              - Show current model"), true);
        print_heart_red(&format!("  /model <color>      - Switch to model by color"), true);
        print_heart_red(&format!("  Available colors: blu, grn, red"), true);
        print_heart_red(&format!("  Example: /model blu"), true);
        return CommandResult::Handled;
    }

    // Map color arguments to actual model names
    let model_str = match model_arg.to_lowercase().as_str() {
        "blu" | "blue" => "blu_model",
        "grn" | "green" => "grn_model",
        "red" => "red_model",
        _ => {
            print_heart_yellow(&format!("{} Invalid model color: '{}'. Available: blu, grn, red", "❌".bright_red(), model_arg), true);
            return CommandResult::Handled;
        }
    };

    let reason = format!("User requested switch to {} model", model_arg);
    match chat.switch_model(model_str, &reason) {
        Ok(msg) => {
            print_heart_red(&format!("{} {}", "✓".bright_green(), msg), true);
            print_heart_red(&format!("{} Current model: {}", "🤖".bright_cyan(), chat.current_model.display_name()), true);

            // Update shared model state for background input task
            {
                let mut model_guard = current_model_shared.write().unwrap();
                *model_guard = chat.current_model;
            }

            // Build the new prompt string and send RefreshPrompt signal
            // We need to access client_config to build the prompt, so we use chat's config
            let client_config = chat.client_config.clone();
            let current_model = chat.current_model.clone();
            let model_name = crate::app::repl::get_model_name_for_prompt(&current_model, &client_config);
            let model_indicator = format!("[{} ({})]", current_model.display_name(), model_name).bright_magenta();
            let request_count = apchat_vty::request_counter::get_count();
            let new_prompt = format!("{}[{}] {}", model_indicator, request_count, "You:".bright_green().bold());

            if let Some(ref signal_sender) = chat.signal_sender {
                let _ = signal_sender.try_send(MspcMessage::RefreshPrompt(new_prompt));
            }
        }
        Err(e) => {
            print_heart_yellow(&format!("{} Failed to switch model: {}", "❌".bright_red(), e), true);
        }
    }
    CommandResult::Handled
}

fn cmd_history(chat: &APChat) -> CommandResult {
    print_heart_red(&format!("{}", "📜 Conversation History:".bright_cyan()), true);
    print_heart_red(&format!("{}", "═".repeat(80).bright_black()), true);

    for (i, msg) in chat.messages.iter().enumerate() {
        let role_label = match msg.role.as_str() {
            "system"    => "SYS".bright_magenta(),
            "user"      => "USR".bright_green(),
            "assistant" => "AST".bright_blue(),
            "tool"      => "TL ".bright_yellow(),
            _           => "???".bright_red(),
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
        let text_content = msg.text_only();
        let content_preview = if text_content.len() > 80 {
            let content: String = text_content.chars().take(77).collect();
            format!("{}...", &content)
        } else {
            text_content
        };

        // Replace newlines with spaces for single-line display
        let content_preview = content_preview.replace('\n', " ");

        print_heart_red(&format!("{:3}. [{}]{} {}", i, role_label, tool_indicator, content_preview.bright_black()), true);
    }

    print_heart_red(&format!("{}", "═".repeat(80).bright_black()), true);
    print_heart_red(&format!("{} Total messages: {}", "📊".bright_cyan(), chat.messages.len()), true);
    CommandResult::Handled
}

fn cmd_debug(chat: &mut APChat, line: &str) -> CommandResult {
    // Check for inference debug subcommand: /debug inference 1|0
    if line.starts_with("/debug inference ") {
        let value_str = line[17..].trim();
        match value_str {
            "1" => {
                chat.set_inference_debug(true);
                print_heart_red(&format!("{} Inference debug enabled", "🔧".bright_green()), true);
            }
            "0" => {
                chat.set_inference_debug(false);
                print_heart_red(&format!("{} Inference debug disabled", "🔧".bright_green()), true);
            }
            _ => {
                print_heart_yellow(&format!("{} Invalid value for inference debug: '{}'. Use 1 to enable or 0 to disable.", "❌".bright_red(), value_str), true);
            }
        }
        return CommandResult::Handled;
    }

    // Check for webex debug subcommand: /debug webex 1|0
    if line.starts_with("/debug webex ") {
        let value_str = line[13..].trim();
        match value_str {
            "1" => {
                chat.set_webex_debug(true);
                print_heart_red(&format!("{} Webex debug enabled", "🔧".bright_green()), true);
            }
            "0" => {
                chat.set_webex_debug(false);
                print_heart_red(&format!("{} Webex debug disabled", "🔧".bright_green()), true);
            }
            _ => {
                print_heart_yellow(&format!("{} Invalid value for webex debug: '{}'. Use 1 to enable or 0 to disable.", "❌".bright_red(), value_str), true);
            }
        }
        return CommandResult::Handled;
    }

    if line == "/debug" {
        print_heart_red(&format!("{} Debug level: {} (binary: {:b})", "🔧".bright_cyan(), chat.get_debug_level(), chat.get_debug_level()), true);
        print_heart_red(&format!("{} Inference debug: {}", "🔧".bright_cyan(), if chat.get_inference_debug() { "enabled" } else { "disabled" }), true);
        print_heart_red(&format!("{} Webex debug: {}", "🔧".bright_cyan(), if chat.get_webex_debug() { "enabled" } else { "disabled" }), true);
        print_heart_red(&format!("{} Usage: /debug <level>", "💡".bright_yellow()), true);
        print_heart_red(&format!("  0 = off"), true);
        print_heart_red(&format!("  1 = basic (bit 0)"), true);
        print_heart_red(&format!("  2 = detailed (bit 1)"), true);
        print_heart_red(&format!("  4 = verbose (bit 2)"), true);
        print_heart_red(&format!("  Example: /debug 3 (enables basic + detailed)"), true);
        print_heart_red(&format!(""), true);
        print_heart_red(&format!("{} Usage: /debug inference <1|0>", "💡".bright_yellow()), true);
        print_heart_red(&format!("  1 = enable inference debug"), true);
        print_heart_red(&format!("  0 = disable inference debug"), true);
        print_heart_red(&format!("  Example: /debug inference 1"), true);
        print_heart_red(&format!(""), true);
        print_heart_red(&format!("{} Usage: /debug webex <1|0>", "💡".bright_yellow()), true);
        print_heart_red(&format!("  1 = enable webex debug (hides all 🔍 debug logs)"), true);
        print_heart_red(&format!("  0 = disable webex debug (show all 🔍 debug logs)"), true);
        print_heart_red(&format!("  Example: /debug webex 1"), true);
        return CommandResult::Handled;
    }

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
    CommandResult::Handled
}

async fn cmd_session(chat: &mut APChat, line: &str) -> CommandResult {
    if line == "/session" || line == "/session help" {
        print_heart_red(&format!("{} Session commands:", "🖥️".bright_cyan()), true);
        print_heart_red(&format!("  /session list           - List all terminal sessions"), true);
        print_heart_red(&format!("  /session show <id>      - Show screen buffer of session"), true);
        print_heart_red(&format!("  /session help           - Show this help"), true);
        return CommandResult::Handled;
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
        return CommandResult::Handled;
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
        return CommandResult::Handled;
    }

    CommandResult::NotACommand
}

fn cmd_skills() -> CommandResult {
    print_heart_red(&format!("{} Available Commands:", "🎯".bright_cyan()), true);
    print_heart_red(&format!("  /model [color]          - Show current model or switch to model by color (blu/grn/red)"), true);
    print_heart_red(&format!("  /history                - Display conversation history with message roles"), true);
    print_heart_red(&format!("  /brainstorm             - Use brainstorming skill for interactive design refinement"), true);
    print_heart_red(&format!("  /write-plan             - Use writing-plans skill to create detailed implementation plan"), true);
    print_heart_red(&format!("  /execute-plan           - Use executing-plans skill to execute plan with checkpoints"), true);
    print_heart_red(&format!("  /compact               - Force immediate conversation compaction to reduce session size"), true);
    print_heart_red(&format!("  /confirm                - Toggle auto-confirm mode (enable/disable confirmation prompts)"), true);
    print_heart_red(&format!("  /skills help            - Show this help"), true);
    CommandResult::Handled
}

/// Activate a skill by name, injecting it into the conversation as a system message.
///
/// All three skill commands (/brainstorm, /write-plan, /execute-plan) follow
/// the same pattern: look up the skill, wrap it in a <skill_invocation> block,
/// push as a system message, log it, and print confirmation.
async fn cmd_skill_activate(
    chat: &mut APChat,
    skill_name: &str,
    display_name: &str,
    icon: &str,
    hint: &str,
) -> CommandResult {
    if let Some(ref skill_registry) = chat.skill_registry {
        match skill_registry.get_skill(skill_name) {
            Some(skill) => {
                let skill_msg = Message {
                    role: "system".to_string(),
                    content: vec![ContentPart::Text(format!(
                        "<skill_invocation>\n🎯 USING SKILL: {}\n\n{}\n\n**YOU MUST follow this skill exactly as written.**\n</skill_invocation>",
                        skill.name, skill.content
                    ))],
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                    reasoning: None,
                };
                chat.messages.push(skill_msg.clone());

                if let Some(logger) = &mut chat.logger {
                    logger.log("system", &skill_msg.text_only(), None, false).await;
                }

                print_heart_red(&format!("{} {} {} skill activated! {}", "✓".bright_green(), "Skill:".bright_cyan(), display_name, icon), true);
                print_heart_red(&format!("{}", hint.bright_black()), true);
            }
            None => {
                print_heart_yellow(&format!("{} {} skill not found. Ensure skills/ directory contains {}/SKILL.md", "❌".bright_red(), display_name, skill_name), true);
            }
        }
    } else {
        print_heart_yellow(&format!("{} Skill registry not available", "❌".bright_red()), true);
    }
    CommandResult::Handled
}

async fn cmd_compact(chat: &mut APChat) -> CommandResult {
    print_heart_red(&format!("{} Starting manual conversation compaction...", "🗜️".bright_blue()), true);
    match intelligent_compaction(chat, 0).await {
        Ok(()) => {
            let session_size = calculate_conversation_size(&chat.messages);
            print_heart_red(&format!("{} Compaction completed successfully!", "✓".bright_green()), true);
            print_heart_red(&format!("{} Session size: {:.1} KB, Messages: {}", "📊".bright_cyan(),
                     session_size as f64 / 1024.0, chat.messages.len()), true);
        }
        Err(e) => {
            print_heart_yellow(&format!("{} Failed to compact conversation: {}", "❌".bright_red(), e), true);
        }
    }
    CommandResult::Handled
}

fn cmd_confirm(chat: &mut APChat) -> CommandResult {
    let is_auto_confirm = chat.policy_manager.is_allow_all();

    // Toggle the mode
    if is_auto_confirm {
        chat.policy_manager = PolicyManager::new();
        print_heart_red(&format!("{} Auto-confirm mode disabled. Actions will now require confirmation.", "✓".bright_green()), true);
    } else {
        chat.policy_manager = PolicyManager::allow_all();
        print_heart_red(&format!("{} Auto-confirm mode enabled. All actions will be approved automatically.", "✓".bright_green()), true);
    }

    let current_state = chat.policy_manager.is_allow_all();
    print_heart_red(&format!("{} Auto-confirm: {}", "📋".bright_cyan(), if current_state {"enabled"} else {"disabled"}), true);
    CommandResult::Handled
}

/// Handle /set model url command
/// Syntax:
///   /set model url              - Show current URLs for all models
///   /set model url <url>        - Set URL for all models
///   /set model url <color> <url> - Set URL for specific model (blu/grn/red)
async fn cmd_set_model_url(
    chat: &mut APChat,
    line: &str,
    _current_model_shared: &Arc<std::sync::RwLock<ModelColor>>,
) -> CommandResult {
    // Remove "/set model url" prefix and trim
    let args = line.strip_prefix("/set model url").unwrap_or(line).trim();

    if args.is_empty() {
        // "/set model url" - show current URLs
        print_heart_red(&format!("{} Current API URLs:", "🔗".bright_cyan()), true);
        print_heart_red(&format!("  blu: {}", display_url(chat.client_config.get_api_url(ModelColor::BluModel))), true);
        print_heart_red(&format!("  grn: {}", display_url(chat.client_config.get_api_url(ModelColor::GrnModel))), true);
        print_heart_red(&format!("  red: {}", display_url(chat.client_config.get_api_url(ModelColor::RedModel))), true);
        return CommandResult::Handled;
    }

    if args == "help" || args == "--help" || args == "-h" {
        // "/set model url help" - show help
        print_heart_red(&format!("{} Set model API URL commands:", "🔗".bright_cyan()), true);
        print_heart_red(&format!("  /set model url              - Show current URLs"), true);
        print_heart_red(&format!("  /set model url <url>        - Set URL for all models"), true);
        print_heart_red(&format!("  /set model url <color> <url> - Set URL for specific model"), true);
        print_heart_red(&format!("  Available colors: blu, grn, red"), true);
        print_heart_red(&format!("  Examples:"), true);
        print_heart_red(&format!("    /set model url http://localhost:8080/v1"), true);
        print_heart_red(&format!("    /set model url blu http://localhost:8080/v1"), true);
        return CommandResult::Handled;
    }

    let parts: Vec<&str> = args.split_whitespace().collect();

    if parts.len() == 1 {
        // "/set model url <url>" - set URL for all models
        let url = parts[0];
        chat.client_config.set_api_url(ModelColor::BluModel, Some(url.to_string()));
        chat.client_config.set_api_url(ModelColor::GrnModel, Some(url.to_string()));
        chat.client_config.set_api_url(ModelColor::RedModel, Some(url.to_string()));
        print_heart_red(&format!("{} API URL set for all models: {}", "✓".bright_green(), url), true);
    } else if parts.len() >= 2 {
        // "/set model url <color> <url>" - set URL for specific model
        let color_str = parts[0].to_lowercase();
        let url = parts[1];

        match color_str.as_str() {
            "blu" | "blue" => {
                chat.client_config.set_api_url(ModelColor::BluModel, Some(url.to_string()));
                print_heart_red(&format!("{} API URL set for blu model: {}", "✓".bright_green(), url), true);
            }
            "grn" | "green" => {
                chat.client_config.set_api_url(ModelColor::GrnModel, Some(url.to_string()));
                print_heart_red(&format!("{} API URL set for grn model: {}", "✓".bright_green(), url), true);
            }
            "red" => {
                chat.client_config.set_api_url(ModelColor::RedModel, Some(url.to_string()));
                print_heart_red(&format!("{} API URL set for red model: {}", "✓".bright_green(), url), true);
            }
            _ => {
                print_heart_yellow(&format!("{} Invalid model color: '{}'. Available: blu, grn, red", "❌".bright_red(), color_str), true);
            }
        }
    } else {
        print_heart_yellow(&format!("{} Invalid command syntax. Use /set model url help for usage.", "❌".bright_red()), true);
    }

    CommandResult::Handled
}

/// Helper function to display URL in a user-friendly format
fn display_url(url: Option<&String>) -> String {
    match url {
        Some(u) => format!("{}", u.bright_blue()),
        None => "not set".bright_black().to_string(),
    }
}
