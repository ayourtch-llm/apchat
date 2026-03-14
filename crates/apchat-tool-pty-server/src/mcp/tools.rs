//! MCP tool definitions

use serde_json::json;
use serde_json::Value;

/// Helper to create a property definition
fn prop(ty: &str, desc: &str) -> Value {
    json!({ "type": ty, "description": desc })
}

/// Helper to create a tool definition
fn tool_def(name: &str, description: &str, properties: Value, required: Vec<&str>) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
        }
    })
}

/// Get all PTY tool definitions
pub fn tool_definitions() -> Vec<Value> {
    vec![
        tool_def(
            "pty_launch",
            "Launch a new PTY terminal session with optional command, working directory, and size. Since this creates an interactive session, commands sent to it must end with \\n (newline) to be executed - this is equivalent to pressing Enter.",
            json!({
                "command": prop("string", "Command to run in the terminal (default: shell)"),
                "working_dir": prop("string", "Working directory for the session (default: current)"),
                "cols": prop("integer", "Terminal width in columns (default: 80)"),
                "rows": prop("integer", "Terminal height in rows (default: 24)"),
            }),
            vec![],
        ),
        tool_def(
            "pty_send_keys",
            "Send keystrokes to a PTY terminal session. IMPORTANT: The <enter> key is added automatically at the end of the command. If you do not want to do it (like, when sending special characters), add 'raw'=true parameter. Also supports special keys: ^C (Ctrl+C to interrupt), ^D (Ctrl+D for EOF), [UP]/[DOWN] (arrow keys), [TAB] (tab completion), etc.",
            json!({
                "session_id": prop("string", "Session ID to send keys to"),
                "keys": prop("string", "Keys to send to the terminal. Will be auto-terminated with 'Enter' key unless raw=true is supplied."),
                "raw": prop("boolean", "Do not add 'Enter' key at the end of the series of keystrokes"),
            }),
            vec!["session_id", "keys"],
        ),
        tool_def(
            "pty_get_screen",
            "Get the current screen contents of a PTY terminal session",
            json!({
                "session_id": prop("string", "Session ID to get screen from"),
                "include_colors": prop("boolean", "Include ANSI color codes (default: false)"),
                "include_cursor": prop("boolean", "Include cursor position (default: true)"),
            }),
            vec!["session_id"],
        ),
        tool_def(
            "pty_list",
            "List all active PTY terminal sessions",
            json!({}),
            vec![],
        ),
        tool_def(
            "pty_kill",
            "Kill a PTY terminal session",
            json!({
                "session_id": prop("string", "Session ID to kill"),
            }),
            vec!["session_id"],
        ),
        tool_def(
            "pty_get_cursor",
            "Get the current cursor position in a PTY terminal session",
            json!({
                "session_id": prop("string", "Session ID to get cursor from"),
            }),
            vec!["session_id"],
        ),
        tool_def(
            "pty_resize",
            "Resize a PTY terminal session",
            json!({
                "session_id": prop("string", "Session ID to resize"),
                "cols": prop("integer", "New terminal width in columns"),
                "rows": prop("integer", "New terminal height in rows"),
            }),
            vec!["session_id", "cols", "rows"],
        ),
        tool_def(
            "pty_set_scrollback",
            "Set the scrollback buffer size for a PTY terminal session",
            json!({
                "session_id": prop("string", "Session ID to configure"),
                "lines": prop("integer", "Number of scrollback lines to keep"),
            }),
            vec!["session_id", "lines"],
        ),
        tool_def(
            "pty_start_capture",
            "Start capturing PTY output to a timestamped file",
            json!({
                "session_id": prop("string", "Session ID to start capturing"),
            }),
            vec!["session_id"],
        ),
        tool_def(
            "pty_stop_capture",
            "Stop capturing PTY output and return capture file information",
            json!({
                "session_id": prop("string", "Session ID to stop capturing"),
            }),
            vec!["session_id"],
        ),
        tool_def(
            "pty_request_user_input",
            "Request user to interact directly with a PTY terminal session. Displays the current screen and message, then allows user to provide input.",
            json!({
                "session_id": prop("string", "Session ID to hand over to user"),
                "message": prop("string", "Message to display to the user explaining what's needed"),
                "timeout_seconds": prop("integer", "Timeout in seconds (default: 300/5 minutes)"),
            }),
            vec!["session_id", "message"],
        ),
        tool_def(
            "pty_send_credential_keys",
            "Send credentials from ~/.config/apchat/credentials.toml to a PTY terminal session. Reads the credentials file, matches device hostname using regex patterns, and types the credential (password or enable_secret) into the session.",
            json!({
                "session_id": prop("string", "PTY session ID to send credentials to"),
                "device_hostname": prop("string", "Hostname of the device (matched against credential 'key' patterns as regex)"),
                "credential_type": prop("string", "Type of credential to send: 'password' or 'enable_secret'"),
            }),
            vec!["session_id", "device_hostname", "credential_type"],
        ),
    ]
}