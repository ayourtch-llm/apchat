use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use serde_json;
use apchat_vty::print_heart_red;
use apchat_models::types::ModelColor;
use apchat_llm_api::client::ChatMessage;


/// Summarize subagent output using an LLM call
async fn summarize_subagent_output(context: &ToolContext, task: &str, output: &str) -> Option<String> {
    let prompt = format!(
        "You are summarizing the output of a subagent that was given a task. \
        Extract and return ONLY the most critically important parts of the content. \
        Be concise but preserve all essential information, key results, file paths, \
        error messages, and actionable details.\n\n\
        **Original Task:** {}\n\n\
        **Subagent Output:**\n{}", task, output
    );

    let message = ChatMessage {
        role: "user".to_string(),
        content: prompt,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning: None,
    };

    // Try each model color in order of preference
    for color in &[ModelColor::GrnModel, ModelColor::BluModel, ModelColor::RedModel] {
        if let Some(client) = context.get_llm_client(color) {
            match client.chat_completion(&[message.clone()]).await {
                Ok(summary) => return Some(summary),
                Err(_) => continue,
            }
        }
    }
    None
}


/// Tool for launching a subagent to execute a task independently
pub struct LaunchSubagentTool;

#[async_trait]
impl Tool for LaunchSubagentTool {
    fn name(&self) -> &str {
        "launch_subagent"
    }

    fn description(&self) -> &str {
        "Launch a subagent to execute a task independently and return structured JSON results. Use this when you need to delegate a self-contained task that should be completed without interaction. The subagent runs in non-interactive mode and returns clean JSON output with task results, files modified, and execution metadata."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("task", "string", "The task description for the subagent to execute", required),
            param!("timeout_seconds", "integer", "Optional timeout in seconds (defaults to 300)", optional),
            param!("auto_confirm", "boolean", "Whether to auto-confirm all actions without prompting (defaults to true)", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let task = match params.get_required::<String>("task") {
            Ok(task) => task,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let timeout_seconds = params.get_optional::<i64>("timeout_seconds")
            .unwrap_or_else(|_| Some(300))
            .unwrap_or(300);
        let auto_confirm = params.get_optional::<bool>("auto_confirm")
            .unwrap_or_else(|_| Some(true))
            .unwrap_or(true);

        // Build the apchat command
        let mut cmd = Command::new("apchat");
        cmd.arg("--task")
           .arg(&task)
           .arg("--auto-confirm"); // Always use auto-confirm for subagent calls

        // Propagate --no-summarize-subagents if summarization is disabled
        if !context.summarize_subagents {
            cmd.arg("--no-summarize-subagents");
        }

        // Add model parameter if current model string is available
        if let Some(model_string) = &context.current_model_string {
            cmd.arg("--model").arg(model_string);
        }

        // Propagate --searxng URL if available
        if let Some(ref searxng_url) = context.searxng_url {
            cmd.arg("--searxng").arg(searxng_url);
        }

        // Propagate --python-sandbox flag if enabled
        if context.python_sandbox {
            cmd.arg("--python-sandbox");
        }

        // Use the same working directory as the main agent
        cmd.current_dir(&context.work_dir);

        // Set environment variables for timeout if needed
        if timeout_seconds != 300 {
            cmd.env("APCHAT_TIMEOUT", timeout_seconds.to_string());
        }

        // Spawn the process to get PID and capture output
        let mut child = match cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => return ToolResult::error(format!("Failed to spawn subagent: {}", e)),
        };

        let pid = child.id();
        let pid_prefix = format!("[{}] ", pid);

        // Read stdout and stderr concurrently to avoid pipe deadlock.
        // Reading them sequentially can deadlock if the child fills the stderr
        // pipe buffer while we're still reading stdout.
        let stdout_reader = BufReader::new(child.stdout.take().unwrap());
        let stderr_reader = BufReader::new(child.stderr.take().unwrap());

        let stdout_pid_prefix = pid_prefix.clone();
        let stderr_pid_prefix = pid_prefix.clone();

        let stdout_handle = std::thread::spawn(move || {
            let mut stdout_content = String::new();
            for line_result in stdout_reader.lines() {
                if let Ok(line) = line_result {
                    print_heart_red(&format!("{}{}", stdout_pid_prefix, line), true);
                    stdout_content.push_str(&line);
                    stdout_content.push('\n');
                }
            }
            stdout_content
        });

        let stderr_handle = std::thread::spawn(move || {
            let mut stderr_content = String::new();
            for line_result in stderr_reader.lines() {
                if let Ok(line) = line_result {
                    eprintln!("{}{}", stderr_pid_prefix, line);
                    stderr_content.push_str(&line);
                    stderr_content.push('\n');
                }
            }
            stderr_content
        });

        let stdout = stdout_handle.join().unwrap_or_else(|e| {
            eprintln!("Subagent stdout reader thread panicked: {:?}", e);
            String::new()
        });
        let stderr = stderr_handle.join().unwrap_or_else(|e| {
            eprintln!("Subagent stderr reader thread panicked: {:?}", e);
            String::new()
        });

        // Wait for process to complete
        let status = match child.wait() {
            Ok(status) => status,
            Err(e) => return ToolResult::error(format!("Failed to wait for subagent: {}", e)),
        };

        // Parse the JSON output
        if status.success() {
            // Try to parse as JSON
            let formatted_result = match serde_json::from_str::<serde_json::Value>(&stdout) {
                Ok(json_result) => {
                    format!(
                        "✅ **Subagent Task Completed Successfully**\n\n**Task:** {}\n\n**Result:** {}",
                        task,
                        serde_json::to_string_pretty(&json_result).unwrap_or_else(|_| stdout.to_string())
                    )
                }
                Err(_) => {
                    format!(
                        "✅ **Subagent Task Completed**\n\n**Task:** {}\n\n**Output:**\n{}",
                        task,
                        stdout
                    )
                }
            };

            if context.summarize_subagents {
                if let Some(summary) = summarize_subagent_output(context, &task, &formatted_result).await {
                    return ToolResult::success(format!(
                        "✅ **Subagent Task Completed (Summarized)**\n\n**Task:** {}\n\n**Summary:**\n{}",
                        task, summary
                    ));
                }
            }

            ToolResult::success(formatted_result)
        } else {
            let error_msg = format!(
                "❌ **Subagent Task Failed**\n\n**Task:** {}\n\n**Error:**\n{}\n\n**Output:**\n{}",
                task,
                stderr,
                stdout
            );
            
            ToolResult::error(error_msg)
        }
    }
}

/// Tool for running subagent with pretty JSON output
pub struct LaunchSubagentPrettyTool;

#[async_trait]
impl Tool for LaunchSubagentPrettyTool {
    fn name(&self) -> &str {
        "launch_subagent_pretty"
    }

    fn description(&self) -> &str {
        "Launch a subagent with pretty-printed JSON output. Same as launch_subagent but formats the JSON result for better readability."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("task", "string", "The task description for the subagent to execute", required),
            param!("timeout_seconds", "integer", "Optional timeout in seconds (defaults to 300)", optional),
            param!("auto_confirm", "boolean", "Whether to auto-confirm all actions without prompting (defaults to true)", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let task = match params.get_required::<String>("task") {
            Ok(task) => task,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let timeout_seconds = params.get_optional::<i64>("timeout_seconds")
            .unwrap_or_else(|_| Some(300))
            .unwrap_or(300);
        let auto_confirm = params.get_optional::<bool>("auto_confirm")
            .unwrap_or_else(|_| Some(true))
            .unwrap_or(true);

        // Build the apchat command with --pretty flag
        let mut cmd = Command::new("apchat");
        cmd.arg("--task")
           .arg(&task)
           .arg("--pretty")
           .arg("--auto-confirm");

        // Propagate --no-summarize-subagents if summarization is disabled
        if !context.summarize_subagents {
            cmd.arg("--no-summarize-subagents");
        }

        // Add model parameter if current model string is available
        if let Some(model_string) = &context.current_model_string {
            cmd.arg("--model").arg(model_string);
        }

        // Propagate --searxng URL if available
        if let Some(ref searxng_url) = context.searxng_url {
            cmd.arg("--searxng").arg(searxng_url);
        }

        // Propagate --python-sandbox flag if enabled
        if context.python_sandbox {
            cmd.arg("--python-sandbox");
        }

        // Use the same working directory as the main agent
        cmd.current_dir(&context.work_dir);

        // Set environment variables for timeout if needed
        if timeout_seconds != 300 {
            cmd.env("APCHAT_TIMEOUT", timeout_seconds.to_string());
        }

        // Spawn the process to get PID and capture output
        let mut child = match cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => return ToolResult::error(format!("Failed to spawn subagent: {}", e)),
        };

        let pid = child.id();
        let pid_prefix = format!("[{}] ", pid);

        // Read stdout and stderr concurrently to avoid pipe deadlock.
        // Reading them sequentially can deadlock if the child fills the stderr
        // pipe buffer while we're still reading stdout.
        let stdout_reader = BufReader::new(child.stdout.take().unwrap());
        let stderr_reader = BufReader::new(child.stderr.take().unwrap());

        let stdout_pid_prefix = pid_prefix.clone();
        let stderr_pid_prefix = pid_prefix.clone();

        let stdout_handle = std::thread::spawn(move || {
            let mut stdout_content = String::new();
            for line_result in stdout_reader.lines() {
                if let Ok(line) = line_result {
                    print_heart_red(&format!("{}{}", stdout_pid_prefix, line), true);
                    stdout_content.push_str(&line);
                    stdout_content.push('\n');
                }
            }
            stdout_content
        });

        let stderr_handle = std::thread::spawn(move || {
            let mut stderr_content = String::new();
            for line_result in stderr_reader.lines() {
                if let Ok(line) = line_result {
                    eprintln!("{}{}", stderr_pid_prefix, line);
                    stderr_content.push_str(&line);
                    stderr_content.push('\n');
                }
            }
            stderr_content
        });

        let stdout = stdout_handle.join().unwrap_or_else(|e| {
            eprintln!("Subagent stdout reader thread panicked: {:?}", e);
            String::new()
        });
        let stderr = stderr_handle.join().unwrap_or_else(|e| {
            eprintln!("Subagent stderr reader thread panicked: {:?}", e);
            String::new()
        });

        // Wait for process to complete
        let status = match child.wait() {
            Ok(status) => status,
            Err(e) => return ToolResult::error(format!("Failed to wait for subagent: {}", e)),
        };

        // Parse the JSON output
        if status.success() {
            // Try to parse as JSON
            let formatted_result = match serde_json::from_str::<serde_json::Value>(&stdout) {
                Ok(json_result) => {
                    format!(
                        "✅ **Subagent Task Completed Successfully**\n\n**Task:** {}\n\n**Pretty JSON Result:**\n```json\n{}\n```",
                        task,
                        serde_json::to_string_pretty(&json_result).unwrap_or_else(|_| stdout.to_string())
                    )
                }
                Err(_) => {
                    format!(
                        "✅ **Subagent Task Completed**\n\n**Task:** {}\n\n**Output:**\n```\n{}\n```",
                        task,
                        stdout
                    )
                }
            };

            if context.summarize_subagents {
                if let Some(summary) = summarize_subagent_output(context, &task, &formatted_result).await {
                    return ToolResult::success(format!(
                        "✅ **Subagent Task Completed (Summarized)**\n\n**Task:** {}\n\n**Summary:**\n{}",
                        task, summary
                    ));
                }
            }

            ToolResult::success(formatted_result)
        } else {
            let error_msg = format!(
                "❌ **Subagent Task Failed**\n\n**Task:** {}\n\n**Error:**\n```\n{}\n```\n\n**Output:**\n```\n{}\n```",
                task,
                stderr,
                stdout
            );
            
            ToolResult::error(error_msg)
        }
    }
}