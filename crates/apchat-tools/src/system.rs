use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::process::Command as AsyncCommand;
use colored::Colorize;
use std::io::Write;
use apchat_vty::print_heart_red;

/// Maximum allowed timeout (120 seconds)
const MAX_TIMEOUT: u64 = 120;

/// Default timeout (30 seconds)
const DEFAULT_TIMEOUT: u64 = 30;

/// Tool for running shell commands
pub struct RunCommandTool;

#[async_trait]
impl Tool for RunCommandTool {
    fn name(&self) -> &str {
        "run_command"
    }

    fn description(&self) -> &str {
        "Run a shell command and return the output"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("command", "string", "Shell command to execute", required),
            param!("timeout", "number", "Request timeout in seconds (default: 30, max: 120)", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let command = match params.get_required::<String>("command") {
            Ok(command) => command,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        // Parse optional timeout parameter
        let timeout_secs = params.get_optional::<u64>("timeout")
            .ok()
            .flatten()
            .unwrap_or(DEFAULT_TIMEOUT);

        if timeout_secs > MAX_TIMEOUT {
            return ToolResult::error(format!("Timeout {} exceeds maximum of {} seconds", timeout_secs, MAX_TIMEOUT));
        }

        // Basic security checks - prevent dangerous commands
        let dangerous_patterns = [
            "rm -rf /",
            "sudo rm",
            ":(){ :|:& };:",
            "chmod -R 777 /",
            "dd if=",
        ];

        for pattern in &dangerous_patterns {
            if command.contains(pattern) {
                return ToolResult::error(format!("Command blocked for security reasons: contains dangerous pattern '{}'", pattern));
            }
        }

        // Check permission using policy system
        print_heart_red(&format!("{} {} ", "Run command:".yellow(), command.cyan()), true);
        std::io::stdout().flush().ok();

        // In non-interactive mode, skip confirmation (already approved via web UI)
        // In interactive mode, check permission using policy system
        let approved = if context.non_interactive {
            true
        } else {
            let (result, _) = match context.check_permission_async(
                apchat_policy::ActionType::CommandExecution,
                &command,
                "Execute? (y/N):"
            ).await {
                Ok((approved, reason)) => (approved, reason),
                Err(e) => return ToolResult::error(format!("Permission check failed: {}", e)),
            };
            result
        };

        if !approved {
            return ToolResult::error("Command cancelled by user or policy".to_string());
        }

        print_heart_red(&format!("{} {} {}ms", "Running:".green(), command.cyan(), timeout_secs * 1000), true);

        // Parse command and arguments
        let orig_command = command.clone();
        let parts: Vec<&str> = command.trim().split_whitespace().collect();
        if parts.is_empty() {
            return ToolResult::error("Empty command".to_string());
        }

        let (cmd, args) = parts.split_first().unwrap();

        // Execute command in work directory with timeout
        let timeout_duration = std::time::Duration::from_secs(timeout_secs);
        let (stdout, stderr, exit_code) = match tokio::time::timeout(timeout_duration, async {
            // Spawn the process
            let mut child = match AsyncCommand::new("bash")
                .args(["-c", &orig_command])
                .current_dir(&context.work_dir)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
            {
                Ok(child) => child,
                Err(e) => return Err(format!("Failed to execute command: {}", e)),
            };

            let stdout = child.stdout.take().expect("Failed to capture stdout");
            let stderr = child.stderr.take().expect("Failed to capture stderr");

            // Read stdout and stderr concurrently using stream merging
            use tokio::io::{AsyncBufReadExt, BufReader};
            use tokio::select;

            let mut stdout_reader = BufReader::new(stdout).lines();
            let mut stderr_reader = BufReader::new(stderr).lines();
            let mut stdout_lines = Vec::new();
            let mut stderr_lines = Vec::new();

            // Drain both streams concurrently
            loop {
                select! {
                    // Read stdout line
                    result = stdout_reader.next_line() => {
                        match result {
                            Ok(Some(line)) => stdout_lines.push(line),
                            Ok(None) => {}, // stdout closed
                            Err(e) => return Err(format!("Error reading stdout: {}", e)),
                        }
                    }
                    // Read stderr line
                    result = stderr_reader.next_line() => {
                        match result {
                            Ok(Some(line)) => stderr_lines.push(line),
                            Ok(None) => {}, // stderr closed
                            Err(e) => return Err(format!("Error reading stderr: {}", e)),
                        }
                    }
                    // Wait for process to exit if both streams are done
                    _ = child.wait() => {
                        break;
                    }
                };
            }

            // Drain any remaining lines
            while let Ok(Some(line)) = stdout_reader.next_line().await {
                stdout_lines.push(line);
            }
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                stderr_lines.push(line);
            }

            // Wait for process to finish and get exit code
            let status = child.wait().await.map_err(|e| format!("Failed to wait for command: {}", e))?;
            let exit_code = status.code().unwrap_or(-1);

            Ok::<(String, String, i32), String>((
                stdout_lines.join("\n"),
                stderr_lines.join("\n"),
                exit_code
            ))
        }).await {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => {
                return ToolResult::error(e);
            }
            Err(_) => {
                return ToolResult::error(format!("Command timed out after {} seconds", timeout_secs));
            }
        };

        let result = if !stderr.is_empty() {
            format!(
                "Command: {}\nExit code: {}\nSTDOUT:\n{}\nSTDERR:\n{}",
                command,
                exit_code,
                stdout,
                stderr
            )
        } else {
            format!(
                "Command: {}\nExit code: {}\nSTDOUT:\n{}",
                command,
                exit_code,
                stdout
            )
        };

        ToolResult::success(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parameters_includes_timeout() {
        let tool = RunCommandTool;
        let params = tool.parameters();
        
        // Check that timeout parameter exists
        assert!(params.contains_key("timeout"));
        
        // Check parameter details
        let timeout_param = &params["timeout"];
        assert_eq!(timeout_param.param_type, "number");
        assert_eq!(timeout_param.description, "Request timeout in seconds (default: 30, max: 120)");
        assert_eq!(timeout_param.required, false);
    }
    
    #[test]
    fn test_timeout_constants() {
        assert_eq!(DEFAULT_TIMEOUT, 30);
        assert_eq!(MAX_TIMEOUT, 120);
    }
}
