//! PTY tool implementations for apchat
//
// This module re-exports PTY tools from the apchat-tool-pty-server crate
// to maintain backward compatibility while using the shared implementation.

use apchat_toolcore::{Tool, ToolParameters, ToolResult, ParameterDefinition, tool_context::ToolContext};
use async_trait::async_trait;
use std::collections::HashMap;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

use apchat_terminal::TerminalManager;

/// Tool for launching a new PTY terminal session
pub struct PtyLaunchTool {
    terminal_manager: Arc<Mutex<TerminalManager>>,
    work_dir: std::path::PathBuf,
}

impl PtyLaunchTool {
    pub fn new(terminal_manager: Arc<Mutex<TerminalManager>>, work_dir: std::path::PathBuf) -> Self {
        Self { terminal_manager, work_dir }
    }
}

#[async_trait]
impl Tool for PtyLaunchTool {
    fn name(&self) -> &str {
        "pty_launch"
    }

    fn description(&self) -> &str {
        "Launch a new PTY terminal session with optional command, working directory, and size. Since this creates an interactive session, commands sent to it must end with \\n (newline) to be executed - this is equivalent to pressing Enter."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            apchat_toolcore::param!("command", "string", "Command to run in the terminal (default: shell)", optional),
            apchat_toolcore::param!("working_dir", "string", "Working directory for the session (default: current)", optional),
            apchat_toolcore::param!("cols", "integer", "Terminal width in columns (default: 80)", optional),
            apchat_toolcore::param!("rows", "integer", "Terminal height in rows (default: 24)", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let command = params.get_optional::<String>("command").unwrap_or(None);
        let working_dir_str = params.get_optional::<String>("working_dir").unwrap_or(None);
        let cols = params.get_optional::<i32>("cols").unwrap_or(None).map(|c| c as u16).unwrap_or(80);
        let rows = params.get_optional::<i32>("rows").unwrap_or(None).map(|r| r as u16).unwrap_or(24);

        // Resolve working directory
        let working_dir = if let Some(dir_str) = &working_dir_str {
            Some(self.work_dir.join(dir_str).display().to_string())
        } else {
            Some(self.work_dir.display().to_string())
        };

        // Generate unique session ID
        let session_id = format!("pty-{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis());

        // Get terminal manager from context
        let terminal_manager = &self.terminal_manager;

        // Create session
        let command_for_display = command.clone().unwrap_or_else(|| "default shell (user's SHELL or /bin/bash)".to_string());
        let mut manager = terminal_manager.lock().await;
        
        match manager.create_session(
            session_id.clone(),
            command,
            working_dir.clone(),
            cols,
            rows
        ).await {
            Ok(returned_id) => {
                let result = json!({
                    "session_id": returned_id,
                    "command": command_for_display,
                    "working_dir": working_dir.unwrap_or_else(|| self.work_dir.display().to_string()),
                    "size": [cols, rows],
                });
                ToolResult::success(format!("PTY session {} launched successfully\n{}", returned_id, serde_json::to_string_pretty(&result).unwrap()))
            }
            Err(e) => ToolResult::error(format!("Failed to launch PTY session: {}", e)),
        }
    }
}

/// Tool for sending keys to a PTY terminal session
pub struct PtySendKeysTool {
    terminal_manager: Arc<Mutex<TerminalManager>>,
}

impl PtySendKeysTool {
    pub fn new(terminal_manager: Arc<Mutex<TerminalManager>>) -> Self {
        Self { terminal_manager }
    }
}

#[async_trait]
impl Tool for PtySendKeysTool {
    fn name(&self) -> &str {
        "pty_send_keys"
    }

    fn description(&self) -> &str {
        "Send keystrokes to a PTY terminal session. IMPORTANT: The <enter> key is added automatically at the end of the command. If you do not want to do it (like, when sending specil characters, add 'raw'=true parameter; Also supports special keys: ^C (Ctrl+C to interrupt), ^D (Ctrl+D for EOF), [UP]/[DOWN] (arrow keys), [TAB] (tab completion), etc."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            apchat_toolcore::param!("session_id", "string", "Session ID to send keys to", required),
            apchat_toolcore::param!("keys", "string", "Keys to send to the terminal. Will be auto-terminated with 'Enter' key unless raw=true is supplied.", required),
            apchat_toolcore::param!("raw", "boolean", "Do not add 'Enter' key at the end of the series of keystrokes", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let session_id = match params.get_required::<String>("session_id") {
            Ok(id) => id,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let mut keys = match params.get_required::<String>("keys") {
            Ok(k) => k,
            Err(e) => return ToolResult::error(e.to_string()),
        };
        let raw = params.get_optional::<bool>("raw")
            .unwrap_or_else(|_| Some(false))
            .unwrap_or(false);

        // Get terminal manager from context
        let terminal_manager = &self.terminal_manager;

        // Send keys
        let mut manager = terminal_manager.lock().await;
        if ! raw {
            keys = format!("{}\n", keys);
        }
        match manager.send_input(&session_id, &keys).await {
            Ok(_) => ToolResult::success(format!("Keys sent to session {}", session_id)),
            Err(e) => ToolResult::error(format!("Failed to send keys: {}", e)),
        }
    }
}

/// Tool for getting the current screen contents of a PTY terminal session
pub struct PtyGetScreenTool {
    terminal_manager: Arc<Mutex<TerminalManager>>,
}

impl PtyGetScreenTool {
    pub fn new(terminal_manager: Arc<Mutex<TerminalManager>>) -> Self {
        Self { terminal_manager }
    }
}

#[async_trait]
impl Tool for PtyGetScreenTool {
    fn name(&self) -> &str {
        "pty_get_screen"
    }

    fn description(&self) -> &str {
        "Get the current screen contents of a PTY terminal session"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            apchat_toolcore::param!("session_id", "string", "Session ID to get screen from", required),
            apchat_toolcore::param!("include_colors", "boolean", "Include ANSI color codes (default: false)", optional),
            apchat_toolcore::param!("include_cursor", "boolean", "Include cursor position (default: true)", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let session_id = match params.get_required::<String>("session_id") {
            Ok(id) => id,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let include_colors = params.get_optional::<bool>("include_colors").unwrap_or(Some(false)).unwrap_or(false);
        let include_cursor = params.get_optional::<bool>("include_cursor").unwrap_or(Some(true)).unwrap_or(true);

        // Get terminal manager from context
        let terminal_manager = &self.terminal_manager;

        // Get screen contents and cursor position
        let manager = terminal_manager.lock().await;
        match manager.get_screen(&session_id, include_colors, include_cursor).await {
            Ok(contents) => {
                let cursor = manager.get_cursor_position(&session_id).await
                    .unwrap_or((0, 0));

                // Get session info for size
                let size = match manager.list_sessions().await {
                    Ok(sessions) => {
                        sessions.iter()
                            .find(|s| s.id == session_id)
                            .map(|s| [s.cols, s.rows])
                            .unwrap_or([80, 24])
                    }
                    Err(_) => [80, 24],
                };

                let result = json!({
                    "session_id": session_id,
                    "contents": contents,
                    "cursor_position": [cursor.1, cursor.0],
                    "size": size,
                });

                ToolResult::success(serde_json::to_string_pretty(&result).unwrap())
            }
            Err(e) => ToolResult::error(format!("Failed to get screen contents: {}", e)),
        }
    }
}

/// Tool for listing all active PTY terminal sessions
pub struct PtyListTool {
    terminal_manager: Arc<Mutex<TerminalManager>>,
}

impl PtyListTool {
    pub fn new(terminal_manager: Arc<Mutex<TerminalManager>>) -> Self {
        Self { terminal_manager }
    }
}

#[async_trait]
impl Tool for PtyListTool {
    fn name(&self) -> &str {
        "pty_list"
    }

    fn description(&self) -> &str {
        "List all active PTY terminal sessions"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::new()
    }

    async fn execute(&self, _params: ToolParameters, _context: &ToolContext) -> ToolResult {
        // Get terminal manager from context
        let terminal_manager = &self.terminal_manager;

        let manager = terminal_manager.lock().await;
        match manager.list_sessions().await {
            Ok(sessions) => {
                let sessions_info: Vec<_> = sessions.iter().map(|s| {
                    json!({
                        "id": s.id,
                        "command": s.command,
                        "working_dir": s.working_dir,
                        "status": s.status,
                        "created_at": s.created_at.duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0),
                        "size": [s.cols, s.rows],
                    })
                }).collect();

                let result = json!({
                    "sessions": sessions_info,
                    "count": sessions_info.len(),
                });

                ToolResult::success(serde_json::to_string_pretty(&result).unwrap())
            }
            Err(e) => ToolResult::error(format!("Failed to list sessions: {}", e)),
        }
    }
}

/// Tool for killing a PTY terminal session
pub struct PtyKillTool {
    terminal_manager: Arc<Mutex<TerminalManager>>,
}

impl PtyKillTool {
    pub fn new(terminal_manager: Arc<Mutex<TerminalManager>>) -> Self {
        Self { terminal_manager }
    }
}

#[async_trait]
impl Tool for PtyKillTool {
    fn name(&self) -> &str {
        "pty_kill"
    }

    fn description(&self) -> &str {
        "Kill a PTY terminal session"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            apchat_toolcore::param!("session_id", "string", "Session ID to kill", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let session_id = match params.get_required::<String>("session_id") {
            Ok(id) => id,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        // Get terminal manager from context
        let terminal_manager = &self.terminal_manager;

        let mut manager = terminal_manager.lock().await;
        match manager.kill_session(&session_id).await {
            Ok(_) => ToolResult::success(format!("Session {} killed successfully", session_id)),
            Err(e) => ToolResult::error(format!("Failed to kill session: {}", e)),
        }
    }
}

/// Tool for getting cursor position
pub struct PtyGetCursorTool {
    terminal_manager: Arc<Mutex<TerminalManager>>,
}

impl PtyGetCursorTool {
    pub fn new(terminal_manager: Arc<Mutex<TerminalManager>>) -> Self {
        Self { terminal_manager }
    }
}

#[async_trait]
impl Tool for PtyGetCursorTool {
    fn name(&self) -> &str {
        "pty_get_cursor"
    }

    fn description(&self) -> &str {
        "Get the current cursor position in a PTY terminal session"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            apchat_toolcore::param!("session_id", "string", "Session ID to get cursor from", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let session_id = match params.get_required::<String>("session_id") {
            Ok(id) => id,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let terminal_manager = &self.terminal_manager;

        let manager = terminal_manager.lock().await;
        match manager.get_cursor_position(&session_id).await {
            Ok((row, col)) => {
                let result = json!({
                    "session_id": session_id,
                    "position": [col, row],
                });

                ToolResult::success(serde_json::to_string_pretty(&result).unwrap())
            }
            Err(e) => ToolResult::error(format!("Failed to get cursor position: {}", e)),
        }
    }
}

/// Tool for resizing a PTY terminal session
pub struct PtyResizeTool {
    terminal_manager: Arc<Mutex<TerminalManager>>,
}

impl PtyResizeTool {
    pub fn new(terminal_manager: Arc<Mutex<TerminalManager>>) -> Self {
        Self { terminal_manager }
    }
}

#[async_trait]
impl Tool for PtyResizeTool {
    fn name(&self) -> &str {
        "pty_resize"
    }

    fn description(&self) -> &str {
        "Resize a PTY terminal session"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            apchat_toolcore::param!("session_id", "string", "Session ID to resize", required),
            apchat_toolcore::param!("cols", "integer", "New terminal width in columns", required),
            apchat_toolcore::param!("rows", "integer", "New terminal height in rows", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let session_id = match params.get_required::<String>("session_id") {
            Ok(id) => id,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let cols = match params.get_required::<i32>("cols") {
            Ok(c) => c as u16,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let rows = match params.get_required::<i32>("rows") {
            Ok(r) => r as u16,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let terminal_manager = &self.terminal_manager;

        let mut manager = terminal_manager.lock().await;
        match manager.resize_session(&session_id, rows, cols).await {
            Ok(_) => {
                let result = json!({
                    "session_id": session_id,
                    "size": [cols, rows],
                });
                ToolResult::success(format!("Session {} resized to {}x{}\n{}",
                    session_id, cols, rows,
                    serde_json::to_string_pretty(&result).unwrap()))
            }
            Err(e) => ToolResult::error(format!("Failed to resize session: {}", e)),
        }
    }
}

/// Tool for setting scrollback buffer size
pub struct PtySetScrollbackTool {
    terminal_manager: Arc<Mutex<TerminalManager>>,
}

impl PtySetScrollbackTool {
    pub fn new(terminal_manager: Arc<Mutex<TerminalManager>>) -> Self {
        Self { terminal_manager }
    }
}

#[async_trait]
impl Tool for PtySetScrollbackTool {
    fn name(&self) -> &str {
        "pty_set_scrollback"
    }

    fn description(&self) -> &str {
        "Set the scrollback buffer size for a PTY terminal session"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            apchat_toolcore::param!("session_id", "string", "Session ID to configure", required),
            apchat_toolcore::param!("lines", "integer", "Number of scrollback lines to keep", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let session_id = match params.get_required::<String>("session_id") {
            Ok(id) => id,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let lines = match params.get_required::<i32>("lines") {
            Ok(l) => l as usize,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let terminal_manager = &self.terminal_manager;

        let mut manager = terminal_manager.lock().await;
        match manager.set_scrollback(&session_id, lines).await {
            Ok(_) => {
                let result = json!({
                    "session_id": session_id,
                    "scrollback_lines": lines,
                });
                ToolResult::success(serde_json::to_string_pretty(&result).unwrap())
            }
            Err(e) => ToolResult::error(format!("Failed to set scrollback: {}", e)),
        }
    }
}

/// Tool for starting output capture to file
pub struct PtyStartCaptureTool {
    terminal_manager: Arc<Mutex<TerminalManager>>,
}

impl PtyStartCaptureTool {
    pub fn new(terminal_manager: Arc<Mutex<TerminalManager>>) -> Self {
        Self { terminal_manager }
    }
}

#[async_trait]
impl Tool for PtyStartCaptureTool {
    fn name(&self) -> &str {
        "pty_start_capture"
    }

    fn description(&self) -> &str {
        "Start capturing PTY output to a timestamped file"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            apchat_toolcore::param!("session_id", "string", "Session ID to start capturing", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let session_id = match params.get_required::<String>("session_id") {
            Ok(id) => id,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let terminal_manager = &self.terminal_manager;

        let mut manager = terminal_manager.lock().await;
        // Generate a default output file path
        let output_file = std::path::PathBuf::from(format!("/tmp/pty-capture-{}.log", session_id));
        match manager.capture_start(&session_id, output_file).await {
            Ok(_) => {
                let result = json!({
                    "session_id": session_id,
                    "status": "capturing",
                });
                ToolResult::success(format!("Started capturing session {}\n{}",
                    session_id,
                    serde_json::to_string_pretty(&result).unwrap()))
            }
            Err(e) => ToolResult::error(format!("Failed to start capture: {}", e)),
        }
    }
}

/// Tool for stopping output capture
pub struct PtyStopCaptureTool {
    terminal_manager: Arc<Mutex<TerminalManager>>,
}

impl PtyStopCaptureTool {
    pub fn new(terminal_manager: Arc<Mutex<TerminalManager>>) -> Self {
        Self { terminal_manager }
    }
}

#[async_trait]
impl Tool for PtyStopCaptureTool {
    fn name(&self) -> &str {
        "pty_stop_capture"
    }

    fn description(&self) -> &str {
        "Stop capturing PTY output and return capture file information"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            apchat_toolcore::param!("session_id", "string", "Session ID to stop capturing", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let session_id = match params.get_required::<String>("session_id") {
            Ok(id) => id,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let terminal_manager = &self.terminal_manager;

        let mut manager = terminal_manager.lock().await;
        // Use the same default path as capture_start
        let capture_path = std::path::PathBuf::from(format!("/tmp/pty-capture-{}.log", session_id));
        match manager.capture_stop(&session_id, capture_path).await {
            Ok((capture_file, bytes, duration)) => {
                let result = json!({
                    "session_id": session_id,
                    "capture_file": capture_file,
                    "bytes_captured": bytes,
                    "duration_seconds": duration,
                });
                ToolResult::success(serde_json::to_string_pretty(&result).unwrap())
            }
            Err(e) => ToolResult::error(format!("Failed to stop capture: {}", e)),
        }
    }
}

/// Tool for requesting user input/interaction with a PTY session
pub struct PtyRequestUserInputTool {
    terminal_manager: Arc<Mutex<TerminalManager>>,
}

impl PtyRequestUserInputTool {
    pub fn new(terminal_manager: Arc<Mutex<TerminalManager>>) -> Self {
        Self { terminal_manager }
    }
}

#[async_trait]
impl Tool for PtyRequestUserInputTool {
    fn name(&self) -> &str {
        "pty_request_user_input"
    }

    fn description(&self) -> &str {
        "Request user to interact directly with a PTY terminal session. Displays the current screen and message, then allows user to provide input. Use this when the LLM needs human assistance (e.g., password entry, manual debugging)."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            apchat_toolcore::param!("session_id", "string", "Session ID to hand over to user", required),
            apchat_toolcore::param!("message", "string", "Message to display to the user explaining what's needed", required),
            apchat_toolcore::param!("timeout_seconds", "integer", "Timeout in seconds (default: 300/5 minutes)", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let session_id = match params.get_required::<String>("session_id") {
            Ok(id) => id,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let message = match params.get_required::<String>("message") {
            Ok(msg) => msg,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let timeout_seconds = params.get_optional::<i32>("timeout_seconds")
            .unwrap_or(Some(300))
            .unwrap_or(300) as u64;

        let terminal_manager = &self.terminal_manager;

        let manager = terminal_manager.lock().await;

        // Get current screen contents
        let screen_contents = match manager.get_screen(&session_id, false, true).await {
            Ok(contents) => contents,
            Err(e) => return ToolResult::error(format!("Failed to get screen contents: {}", e)),
        };

        // Get session info
        let (working_dir, command) = match manager.list_sessions().await {
            Ok(sessions) => {
                sessions.iter()
                    .find(|s| s.id == session_id)
                    .map(|s| (
                        s.working_dir.clone().unwrap_or_else(|| "unknown".to_string()),
                        s.command.clone()
                    ))
                    .unwrap_or_else(|| ("unknown".to_string(), "unknown".to_string()))
            }
            Err(_) => ("unknown".to_string(), "unknown".to_string()),
        };

        // For now, we return information about the session state and instructions
        // A full implementation would involve complex async I/O handling to actually
        // attach the terminal to the user's stdin/stdout
        let result = json!({
            "session_id": session_id,
            "message": message,
            "timeout_seconds": timeout_seconds,
            "current_screen": screen_contents,
            "working_dir": working_dir,
            "command": command,
            "instructions": format!(
                "User assistance requested for terminal session {}.\n\n\
                Message: {}\n\n\
                Current screen state:\n{}\n\n\
                To interact with this session, use:\n\
                - pty_send_keys to send commands\n\
                - pty_get_screen to see updated output\n\n\
                Session will remain available for {} seconds.",
                session_id, message, screen_contents, timeout_seconds
            ),
        });

        ToolResult::success(serde_json::to_string_pretty(&result).unwrap())
    }
}

/// Credential entry from credentials.toml
#[derive(Debug, serde::Deserialize, Clone)]
struct CredentialEntry {
    key: String,
    password: Option<String>,
    enable_secret: Option<String>,
}

/// Root structure for credentials.toml
#[derive(Debug, serde::Deserialize)]
struct CredentialsFile {
    credentials: Vec<CredentialEntry>,
}

/// Tool for sending credentials from credentials.toml to a PTY session
pub struct PtySendCredentialKeysTool {
    terminal_manager: Arc<Mutex<TerminalManager>>,
}

impl PtySendCredentialKeysTool {
    pub fn new(terminal_manager: Arc<Mutex<TerminalManager>>) -> Self {
        Self { terminal_manager }
    }
}

#[async_trait]
impl Tool for PtySendCredentialKeysTool {
    fn name(&self) -> &str {
        "pty_send_credential_keys"
    }

    fn description(&self) -> &str {
        "Send credentials from ~/.config/apchat/credentials.toml to a PTY terminal session. Reads the credentials file, matches device hostname using regex patterns, and types the credential (password or enable_secret) into the session. IMPORTANT: Verifies credentials don't echo back to the terminal."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            apchat_toolcore::param!("session_id", "string", "PTY session ID to send credentials to", required),
            apchat_toolcore::param!("device_hostname", "string", "Hostname of the device (matched against credential 'key' patterns as regex)", required),
            apchat_toolcore::param!("credential_type", "string", "Type of credential to send: 'password' or 'enable_secret'", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        // Get parameters
        let session_id = match params.get_required::<String>("session_id") {
            Ok(id) => id,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let device_hostname = match params.get_required::<String>("device_hostname") {
            Ok(hostname) => hostname,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let credential_type = match params.get_required::<String>("credential_type") {
            Ok(ct) => ct,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        // Validate credential_type
        if credential_type != "password" && credential_type != "enable_secret" {
            return ToolResult::error(format!(
                "Invalid credential_type '{}'. Must be 'password' or 'enable_secret'",
                credential_type
            ));
        }

        // Get terminal manager
        let terminal_manager = &self.terminal_manager;

        // Read credentials file
        let credentials_path = apchat_common::ApChatPaths::credentials_file();

        if !credentials_path.exists() {
            return ToolResult::error(format!(
                "Credentials file not found at: {}",
                credentials_path.display()
            ));
        }

        let credentials_content = match std::fs::read_to_string(&credentials_path) {
            Ok(content) => content,
            Err(e) => return ToolResult::error(format!(
                "Failed to read credentials file: {}",
                e
            )),
        };

        let credentials_file: CredentialsFile = match toml::from_str(&credentials_content) {
            Ok(creds) => creds,
            Err(e) => return ToolResult::error(format!(
                "Failed to parse credentials file: {}",
                e
            )),
        };

        // Find matching credential using regex
        let mut matched_credential: Option<CredentialEntry> = None;

        for cred in credentials_file.credentials {
            match regex::Regex::new(&cred.key) {
                Ok(pattern) => {
                    if pattern.is_match(&device_hostname) {
                        matched_credential = Some(cred);
                        break;
                    }
                }
                Err(e) => {
                    return ToolResult::error(format!(
                        "Invalid regex pattern in credential key '{}': {}",
                        cred.key, e
                    ));
                }
            }
        }

        let credential_entry = match matched_credential {
            Some(cred) => cred,
            None => return ToolResult::error(format!(
                "No credential found matching hostname '{}'",
                device_hostname
            )),
        };

        // Get the actual credential value
        let credential_value = if credential_type == "password" {
            match &credential_entry.password {
                Some(pwd) => pwd.clone(),
                None => return ToolResult::error(format!(
                    "Credential matching '{}' has no password field",
                    device_hostname
                )),
            }
        } else {
            match &credential_entry.enable_secret {
                Some(secret) => secret.clone(),
                None => return ToolResult::error(format!(
                    "Credential matching '{}' has no enable_secret field",
                    device_hostname
                )),
            }
        };

        // Get screen contents before sending credential
        let manager = terminal_manager.lock().await;
        let screen_before = match manager.get_screen(&session_id, false, false).await {
            Ok(screen) => screen,
            Err(e) => return ToolResult::error(format!(
                "Failed to get screen contents before sending credential: {}",
                e
            )),
        };
        drop(manager);

        // Send the credential followed by Enter
        let mut manager = terminal_manager.lock().await;
        let credential_with_enter = format!("{}\n", credential_value);
        match manager.send_input(&session_id, &credential_with_enter).await {
            Ok(_) => {},
            Err(e) => return ToolResult::error(format!(
                "Failed to send credential to session: {}",
                e
            )),
        };
        drop(manager);

        // Check if credential echoed back
        let manager = terminal_manager.lock().await;
        let screen_after = match manager.get_screen(&session_id, false, false).await {
            Ok(screen) => screen,
            Err(e) => return ToolResult::error(format!(
                "Failed to get screen contents after sending credential: {}",
                e
            )),
        };
        drop(manager);

        if screen_after.contains(&credential_value) && !screen_before.contains(&credential_value) {
            return ToolResult::error(format!(
                "SECURITY WARNING: Credential echoed back in terminal! The {} was visible in the terminal output.",
                credential_type
            ));
        }

        let result_json = json!({
            "session_id": session_id,
            "device_hostname": device_hostname,
            "credential_type": credential_type,
            "status": "credential sent successfully",
            "echo_check": "passed - credential did not echo back"
        });
        let result_str = serde_json::to_string_pretty(&result_json).unwrap();
        ToolResult::success(result_str)
    }
}