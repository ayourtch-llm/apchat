use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use serde_json;


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
            param!("working_directory", "string", "Optional working directory for the subagent (defaults to current directory)", optional),
            param!("timeout_seconds", "integer", "Optional timeout in seconds (defaults to 300)", optional),
            param!("auto_confirm", "boolean", "Whether to auto-confirm all actions without prompting (defaults to true)", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let task = match params.get_required::<String>("task") {
            Ok(task) => task,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let working_directory = params.get_optional::<String>("working_directory")
            .unwrap_or_else(|_| Some(context.work_dir.to_string_lossy().to_string()))
            .unwrap_or_else(|| context.work_dir.to_string_lossy().to_string());

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

        // Add model parameter if current model string is available
        if let Some(model_string) = &context.current_model_string {
            cmd.arg("--model").arg(model_string);
        }

        if working_directory != context.work_dir.to_string_lossy() {
            cmd.current_dir(&working_directory);
        }

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

        // Read stdout and stderr in real-time and print with PID prefix
        let stdout_reader = BufReader::new(child.stdout.take().unwrap());
        let stderr_reader = BufReader::new(child.stderr.take().unwrap());

        // Use spawn_blocking to read output since we're in async context
        let result = tokio::task::spawn_blocking(move || {
            let mut stdout_content = String::new();
            let mut stderr_content = String::new();

            // Read stdout
            for line_result in stdout_reader.lines() {
                if let Ok(line) = line_result {
                    println!("{}{}", pid_prefix, line);
                    stdout_content.push_str(&line);
                    stdout_content.push('\n');
                }
            }

            // Read stderr
            for line_result in stderr_reader.lines() {
                if let Ok(line) = line_result {
                    eprintln!("{}{}", pid_prefix, line);
                    stderr_content.push_str(&line);
                    stderr_content.push('\n');
                }
            }

            (stdout_content, stderr_content)
        }).await;

        let (stdout, stderr) = match result {
            Ok(result) => result,
            Err(e) => return ToolResult::error(format!("Failed to read subagent output: {}", e)),
        };

        // Wait for process to complete
        let status = match child.wait() {
            Ok(status) => status,
            Err(e) => return ToolResult::error(format!("Failed to wait for subagent: {}", e)),
        };

        // Parse the JSON output
        if status.success() {
            // Try to parse as JSON
            match serde_json::from_str::<serde_json::Value>(&stdout) {
                Ok(json_result) => {
                    // Format the result nicely
                    let formatted_result = format!(
                        "✅ **Subagent Task Completed Successfully**\n\n**Task:** {}\n\n**Result:** {}",
                        task,
                        serde_json::to_string_pretty(&json_result).unwrap_or_else(|_| stdout.to_string())
                    );
                    ToolResult::success(formatted_result)
                }
                Err(_) => {
                    // If not valid JSON, return as plain text
                    let result = format!(
                        "✅ **Subagent Task Completed**\n\n**Task:** {}\n\n**Output:**\n{}",
                        task,
                        stdout
                    );
                    ToolResult::success(result)
                }
            }
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
            param!("working_directory", "string", "Optional working directory for the subagent (defaults to current directory)", optional),
            param!("timeout_seconds", "integer", "Optional timeout in seconds (defaults to 300)", optional),
            param!("auto_confirm", "boolean", "Whether to auto-confirm all actions without prompting (defaults to true)", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let task = match params.get_required::<String>("task") {
            Ok(task) => task,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let working_directory = params.get_optional::<String>("working_directory")
            .unwrap_or_else(|_| Some(context.work_dir.to_string_lossy().to_string()))
            .unwrap_or_else(|| context.work_dir.to_string_lossy().to_string());

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

        // Add model parameter if current model string is available
        if let Some(model_string) = &context.current_model_string {
            cmd.arg("--model").arg(model_string);
        }

        if working_directory != context.work_dir.to_string_lossy() {
            cmd.current_dir(&working_directory);
        }

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

        // Read stdout and stderr in real-time and print with PID prefix
        let stdout_reader = BufReader::new(child.stdout.take().unwrap());
        let stderr_reader = BufReader::new(child.stderr.take().unwrap());

        // Use spawn_blocking to read output since we're in async context
        let result = tokio::task::spawn_blocking(move || {
            let mut stdout_content = String::new();
            let mut stderr_content = String::new();

            // Read stdout
            for line_result in stdout_reader.lines() {
                if let Ok(line) = line_result {
                    println!("{}{}", pid_prefix, line);
                    stdout_content.push_str(&line);
                    stdout_content.push('\n');
                }
            }

            // Read stderr
            for line_result in stderr_reader.lines() {
                if let Ok(line) = line_result {
                    eprintln!("{}{}", pid_prefix, line);
                    stderr_content.push_str(&line);
                    stderr_content.push('\n');
                }
            }

            (stdout_content, stderr_content)
        }).await;

        let (stdout, stderr) = match result {
            Ok(result) => result,
            Err(e) => return ToolResult::error(format!("Failed to read subagent output: {}", e)),
        };

        // Wait for process to complete
        let status = match child.wait() {
            Ok(status) => status,
            Err(e) => return ToolResult::error(format!("Failed to wait for subagent: {}", e)),
        };

        // Parse the JSON output
        if status.success() {
            // Try to parse as JSON
            match serde_json::from_str::<serde_json::Value>(&stdout) {
                Ok(json_result) => {
                    // Format the result nicely
                    let formatted_result = format!(
                        "✅ **Subagent Task Completed Successfully**\n\n**Task:** {}\n\n**Pretty JSON Result:**\n```json\n{}\n```",
                        task,
                        serde_json::to_string_pretty(&json_result).unwrap_or_else(|_| stdout.to_string())
                    );
                    ToolResult::success(formatted_result)
                }
                Err(_) => {
                    // If not valid JSON, return as plain text
                    let result = format!(
                        "✅ **Subagent Task Completed**\n\n**Task:** {}\n\n**Output:**\n```\n{}\n```",
                        task,
                        stdout
                    );
                    ToolResult::success(result)
                }
            }
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