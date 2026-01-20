use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::process::Command as AsyncCommand;
use colored::Colorize;
use std::io::Write;
use apchat_logging::vty_output::print_heart_red;

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
        print_heart_red(&format!("{} {} ", "Run command:".yellow(), command.cyan()), false);
        std::io::stdout().flush().ok();

        let (approved, rejection_reason) = match context.check_permission(
            apchat_policy::ActionType::CommandExecution,
            &command,
            "Execute? (y/N):"
        ) {
            Ok((approved, reason)) => (approved, reason),
            Err(e) => return ToolResult::error(format!("Permission check failed: {}", e)),
        };

        if !approved {
            let error_msg = if let Some(reason) = rejection_reason {
                format!("Command cancelled by user: {}", reason)
            } else {
                "Command cancelled by user or policy".to_string()
            };
            return ToolResult::error(error_msg);
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
        let output = match tokio::time::timeout(timeout_duration, async {
            AsyncCommand::new("bash")
                .args(["-c", &orig_command])
                .current_dir(&context.work_dir)
                .output()
                .await
        }).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return ToolResult::error(format!("Failed to execute command: {}", e));
            }
            Err(_) => {
                return ToolResult::error(format!("Command timed out after {} seconds", timeout_secs));
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let result = if !stderr.is_empty() {
            format!(
                "Command: {}\nExit code: {}\nSTDOUT:\n{}\nSTDERR:\n{}",
                command,
                output.status.code().unwrap_or(-1),
                stdout,
                stderr
            )
        } else {
            format!(
                "Command: {}\nExit code: {}\nSTDOUT:\n{}",
                command,
                output.status.code().unwrap_or(-1),
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