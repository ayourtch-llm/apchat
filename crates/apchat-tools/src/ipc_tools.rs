use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use apchat_mspc::ipc::{get_ipc_msg_dir, get_socket_path};

/// Tool to discover all running apchat agents.
pub struct GetAgentTreeTool;

#[async_trait]
impl Tool for GetAgentTreeTool {
    fn name(&self) -> &str { "get_agent_tree" }

    fn description(&self) -> &str {
        "List all running apchat agents with their PIDs, tasks, state, and command lines. \
         Use this to discover agents for inter-agent communication via send_agent_message."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> { HashMap::new() }

    async fn execute(&self, _params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let msg_dir = get_ipc_msg_dir();
        if !msg_dir.exists() {
            return ToolResult::success("[]".to_string());
        }

        let mut agents = Vec::new();
        let our_pid = std::process::id();

        if let Ok(entries) = std::fs::read_dir(&msg_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Some(pid_str) = name.strip_prefix("apchat_pid_").and_then(|s| s.strip_suffix(".sock")) {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        #[cfg(unix)]
                        let alive = unsafe { libc::kill(pid as i32, 0) == 0 };
                        #[cfg(not(unix))]
                        let alive = true;

                        if !alive {
                            let _ = std::fs::remove_file(entry.path());
                            continue;
                        }

                        let cmdline = get_process_cmdline(pid);
                        let task = extract_task_from_cmdline(&cmdline);

                        agents.push(serde_json::json!({
                            "pid": pid,
                            "cmdline": cmdline,
                            "task": task,
                            "state": if pid == our_pid { "active (self)" } else { "active" },
                            "is_self": pid == our_pid,
                        }));
                    }
                }
            }
        }

        agents.sort_by_key(|a| a["pid"].as_u64().unwrap_or(0));
        ToolResult::success(serde_json::to_string_pretty(&agents).unwrap_or("[]".to_string()))
    }
}

/// Tool to send a message to another apchat agent via Unix datagram socket.
pub struct SendAgentMessageTool;

#[async_trait]
impl Tool for SendAgentMessageTool {
    fn name(&self) -> &str { "send_agent_message" }

    fn description(&self) -> &str {
        "Send a message to another running apchat agent by PID. The message will be injected \
         into the target agent's conversation. Use get_agent_tree first to discover available agents."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("target_pid", "integer", "PID of the target apchat agent", required),
            param!("message", "string", "Message to send to the target agent", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let target_pid = match params.get_required::<i64>("target_pid") {
            Ok(pid) => pid as u32,
            Err(e) => return ToolResult::error(e.to_string()),
        };
        let message = match params.get_required::<String>("message") {
            Ok(msg) => msg,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let our_pid = std::process::id();
        let target_path = get_socket_path(target_pid);

        if !target_path.exists() {
            return ToolResult::error(format!(
                "No agent found with PID {}. Use get_agent_tree to discover available agents.",
                target_pid
            ));
        }

        // Format with reply instructions
        let full_msg = format!(
            "Message from PID {}, please use send_agent_message to reply: {}",
            our_pid, message
        );

        let payload = serde_json::json!({
            "sender_pid": our_pid,
            "content": full_msg,
        });
        let data = payload.to_string();

        // Send via our bound socket so receiver can identify sender from address
        let our_path = get_socket_path(our_pid);
        let result = if our_path.exists() {
            // Use our existing bound socket
            match std::os::unix::net::UnixDatagram::unbound() {
                Ok(sock) => sock.send_to(data.as_bytes(), &target_path),
                Err(e) => Err(e),
            }
        } else {
            // Fallback: unbound send
            match std::os::unix::net::UnixDatagram::unbound() {
                Ok(sock) => sock.send_to(data.as_bytes(), &target_path),
                Err(e) => Err(e),
            }
        };

        match result {
            Ok(_) => ToolResult::success(format!("Message sent to agent PID {}", target_pid)),
            Err(e) => ToolResult::error(format!("Failed to send to PID {}: {}", target_pid, e)),
        }
    }
}

fn get_process_cmdline(pid: u32) -> String {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string(format!("/proc/{}/cmdline", pid))
            .unwrap_or_default()
            .replace('\0', " ")
            .trim()
            .to_string()
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "command="])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default()
            .trim()
            .to_string()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    { format!("(unknown, pid={})", pid) }
}

fn extract_task_from_cmdline(cmdline: &str) -> Option<String> {
    let parts: Vec<&str> = cmdline.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == "--task" {
            return parts.get(i + 1).map(|s| s.to_string());
        }
    }
    None
}
