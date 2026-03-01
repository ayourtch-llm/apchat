use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

use apchat_terminal::TerminalManager;

// ---------------------------------------------------------------------------
// JSON-RPC types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

// ---------------------------------------------------------------------------
// MCP tool schema helper
// ---------------------------------------------------------------------------

fn prop(ty: &str, desc: &str) -> Value {
    json!({ "type": ty, "description": desc })
}

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

fn tool_definitions() -> Vec<Value> {
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
            vec!["session_id", "keys", "raw"],
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
            "Request user to interact directly with a PTY terminal session. Displays the current screen and message, then allows user to provide input. Use this when the LLM needs human assistance (e.g., password entry, manual debugging).",
            json!({
                "session_id": prop("string", "Session ID to hand over to user"),
                "message": prop("string", "Message to display to the user explaining what's needed"),
                "timeout_seconds": prop("integer", "Timeout in seconds (default: 300/5 minutes)"),
            }),
            vec!["session_id", "message"],
        ),
        tool_def(
            "pty_send_credential_keys",
            "Send credentials from ~/.okaychat/credentials.toml to a PTY terminal session. Reads the credentials file, matches device hostname using regex patterns, and types the credential (password or enable_secret) into the session. IMPORTANT: Verifies credentials don't echo back to the terminal.",
            json!({
                "session_id": prop("string", "PTY session ID to send credentials to"),
                "device_hostname": prop("string", "Hostname of the device (matched against credential 'key' patterns as regex)"),
                "credential_type": prop("string", "Type of credential to send: 'password' or 'enable_secret'"),
            }),
            vec!["session_id", "device_hostname", "credential_type"],
        ),
    ]
}

// ---------------------------------------------------------------------------
// Tool execution
// ---------------------------------------------------------------------------

async fn handle_tool_call(
    manager: &Arc<Mutex<TerminalManager>>,
    work_dir: &PathBuf,
    name: &str,
    args: &Value,
) -> Result<Value> {
    match name {
        "pty_launch" => handle_pty_launch(manager, work_dir, args).await,
        "pty_send_keys" => handle_pty_send_keys(manager, args).await,
        "pty_get_screen" => handle_pty_get_screen(manager, args).await,
        "pty_list" => handle_pty_list(manager).await,
        "pty_kill" => handle_pty_kill(manager, args).await,
        "pty_get_cursor" => handle_pty_get_cursor(manager, args).await,
        "pty_resize" => handle_pty_resize(manager, args).await,
        "pty_set_scrollback" => handle_pty_set_scrollback(manager, args).await,
        "pty_start_capture" => handle_pty_start_capture(manager, args).await,
        "pty_stop_capture" => handle_pty_stop_capture(manager, args).await,
        "pty_request_user_input" => handle_pty_request_user_input(manager, args).await,
        "pty_send_credential_keys" => handle_pty_send_credential_keys(manager, args).await,
        _ => anyhow::bail!("Unknown tool: {}", name),
    }
}

fn get_str(args: &Value, key: &str) -> Result<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter '{}'", key))
}

fn get_optional_str(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn get_optional_bool(args: &Value, key: &str, default: bool) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}

fn get_optional_i64(args: &Value, key: &str, default: i64) -> i64 {
    args.get(key).and_then(|v| v.as_i64()).unwrap_or(default)
}

fn get_i64(args: &Value, key: &str) -> Result<i64> {
    args.get(key)
        .and_then(|v| v.as_i64())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter '{}'", key))
}

async fn handle_pty_launch(
    manager: &Arc<Mutex<TerminalManager>>,
    work_dir: &PathBuf,
    args: &Value,
) -> Result<Value> {
    let command = get_optional_str(args, "command");
    let working_dir_str = get_optional_str(args, "working_dir");
    let cols = get_optional_i64(args, "cols", 80) as u16;
    let rows = get_optional_i64(args, "rows", 24) as u16;

    let working_dir = if let Some(dir_str) = &working_dir_str {
        Some(work_dir.join(dir_str).display().to_string())
    } else {
        Some(work_dir.display().to_string())
    };

    let session_id = format!(
        "pty-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    let command_for_display = command
        .clone()
        .unwrap_or_else(|| "default shell (user's SHELL or /bin/bash)".to_string());
    let mut mgr = manager.lock().await;
    let returned_id = mgr
        .create_session(session_id, command, working_dir.clone(), cols, rows)
        .await?;

    Ok(json!({
        "session_id": returned_id,
        "command": command_for_display,
        "working_dir": working_dir.unwrap_or_else(|| work_dir.display().to_string()),
        "size": [cols, rows],
    }))
}

async fn handle_pty_send_keys(
    manager: &Arc<Mutex<TerminalManager>>,
    args: &Value,
) -> Result<Value> {
    let session_id = get_str(args, "session_id")?;
    let mut keys = get_str(args, "keys")?;
    let raw = get_optional_bool(args, "raw", false);

    let mut mgr = manager.lock().await;
    if !raw {
        keys = format!("{}\n", keys);
    }
    mgr.send_input(&session_id, &keys).await?;
    Ok(json!({ "status": format!("Keys sent to session {}", session_id) }))
}

async fn handle_pty_get_screen(
    manager: &Arc<Mutex<TerminalManager>>,
    args: &Value,
) -> Result<Value> {
    let session_id = get_str(args, "session_id")?;
    let include_colors = get_optional_bool(args, "include_colors", false);
    let include_cursor = get_optional_bool(args, "include_cursor", true);

    let mgr = manager.lock().await;
    let contents = mgr
        .get_screen(&session_id, include_colors, include_cursor)
        .await?;
    let cursor = mgr
        .get_cursor_position(&session_id)
        .await
        .unwrap_or((0, 0));
    let size = match mgr.list_sessions().await {
        Ok(sessions) => sessions
            .iter()
            .find(|s| s.id == session_id)
            .map(|s| [s.cols, s.rows])
            .unwrap_or([80, 24]),
        Err(_) => [80, 24],
    };

    Ok(json!({
        "session_id": session_id,
        "contents": contents,
        "cursor_position": [cursor.1, cursor.0],
        "size": size,
    }))
}

async fn handle_pty_list(manager: &Arc<Mutex<TerminalManager>>) -> Result<Value> {
    let mgr = manager.lock().await;
    let sessions = mgr.list_sessions().await?;
    let sessions_info: Vec<Value> = sessions
        .iter()
        .map(|s| {
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
        })
        .collect();

    Ok(json!({
        "sessions": sessions_info,
        "count": sessions_info.len(),
    }))
}

async fn handle_pty_kill(
    manager: &Arc<Mutex<TerminalManager>>,
    args: &Value,
) -> Result<Value> {
    let session_id = get_str(args, "session_id")?;
    let mut mgr = manager.lock().await;
    mgr.kill_session(&session_id).await?;
    Ok(json!({ "status": format!("Session {} killed successfully", session_id) }))
}

async fn handle_pty_get_cursor(
    manager: &Arc<Mutex<TerminalManager>>,
    args: &Value,
) -> Result<Value> {
    let session_id = get_str(args, "session_id")?;
    let mgr = manager.lock().await;
    let (row, col) = mgr.get_cursor_position(&session_id).await?;
    Ok(json!({
        "session_id": session_id,
        "position": [col, row],
    }))
}

async fn handle_pty_resize(
    manager: &Arc<Mutex<TerminalManager>>,
    args: &Value,
) -> Result<Value> {
    let session_id = get_str(args, "session_id")?;
    let cols = get_i64(args, "cols")? as u16;
    let rows = get_i64(args, "rows")? as u16;

    let mut mgr = manager.lock().await;
    mgr.resize_session(&session_id, rows, cols).await?;
    Ok(json!({
        "session_id": session_id,
        "size": [cols, rows],
    }))
}

async fn handle_pty_set_scrollback(
    manager: &Arc<Mutex<TerminalManager>>,
    args: &Value,
) -> Result<Value> {
    let session_id = get_str(args, "session_id")?;
    let lines = get_i64(args, "lines")? as usize;

    let mut mgr = manager.lock().await;
    mgr.set_scrollback(&session_id, lines).await?;
    Ok(json!({
        "session_id": session_id,
        "scrollback_lines": lines,
    }))
}

async fn handle_pty_start_capture(
    manager: &Arc<Mutex<TerminalManager>>,
    args: &Value,
) -> Result<Value> {
    let session_id = get_str(args, "session_id")?;
    let mut mgr = manager.lock().await;
    mgr.capture_start(&session_id, String::new()).await?;
    Ok(json!({
        "session_id": session_id,
        "status": "capturing",
    }))
}

async fn handle_pty_stop_capture(
    manager: &Arc<Mutex<TerminalManager>>,
    args: &Value,
) -> Result<Value> {
    let session_id = get_str(args, "session_id")?;
    let mut mgr = manager.lock().await;
    let (capture_file, bytes, duration) = mgr.capture_stop(&session_id).await?;
    Ok(json!({
        "session_id": session_id,
        "capture_file": capture_file,
        "bytes_captured": bytes,
        "duration_seconds": duration,
    }))
}

async fn handle_pty_request_user_input(
    manager: &Arc<Mutex<TerminalManager>>,
    args: &Value,
) -> Result<Value> {
    let session_id = get_str(args, "session_id")?;
    let message = get_str(args, "message")?;
    let timeout_seconds = get_optional_i64(args, "timeout_seconds", 300);

    let mgr = manager.lock().await;
    let screen_contents = mgr.get_screen(&session_id, false, true).await?;

    let (working_dir, command) = match mgr.list_sessions().await {
        Ok(sessions) => sessions
            .iter()
            .find(|s| s.id == session_id)
            .map(|s| {
                (
                    s.working_dir
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                    s.command.clone(),
                )
            })
            .unwrap_or_else(|| ("unknown".to_string(), "unknown".to_string())),
        Err(_) => ("unknown".to_string(), "unknown".to_string()),
    };

    Ok(json!({
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
    }))
}

#[derive(Debug, Deserialize, Clone)]
struct CredentialEntry {
    key: String,
    password: Option<String>,
    enable_secret: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CredentialsFile {
    credentials: Vec<CredentialEntry>,
}

async fn handle_pty_send_credential_keys(
    manager: &Arc<Mutex<TerminalManager>>,
    args: &Value,
) -> Result<Value> {
    let session_id = get_str(args, "session_id")?;
    let device_hostname = get_str(args, "device_hostname")?;
    let credential_type = get_str(args, "credential_type")?;

    if credential_type != "password" && credential_type != "enable_secret" {
        anyhow::bail!(
            "Invalid credential_type '{}'. Must be 'password' or 'enable_secret'",
            credential_type
        );
    }

    let home_dir = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;

    let credentials_path = home_dir.join(".okaychat").join("credentials.toml");

    if !credentials_path.exists() {
        anyhow::bail!(
            "Credentials file not found at: {}",
            credentials_path.display()
        );
    }

    let credentials_content = std::fs::read_to_string(&credentials_path)?;
    let credentials_file: CredentialsFile = toml::from_str(&credentials_content)?;

    let mut matched_credential: Option<CredentialEntry> = None;
    for cred in credentials_file.credentials {
        let pattern = regex::Regex::new(&cred.key)?;
        if pattern.is_match(&device_hostname) {
            matched_credential = Some(cred);
            break;
        }
    }

    let credential_entry = matched_credential
        .ok_or_else(|| anyhow::anyhow!("No credential found matching hostname '{}'", device_hostname))?;

    let credential_value = if credential_type == "password" {
        credential_entry
            .password
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Credential matching '{}' has no password field", device_hostname))?
    } else {
        credential_entry
            .enable_secret
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Credential matching '{}' has no enable_secret field", device_hostname))?
    };

    // Get screen before
    let mgr = manager.lock().await;
    let screen_before = mgr.get_screen(&session_id, false, false).await?;
    drop(mgr);

    // Send credential
    let mut mgr = manager.lock().await;
    let credential_with_enter = format!("{}\n", credential_value);
    mgr.send_input(&session_id, &credential_with_enter).await?;
    drop(mgr);

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Verify no echo
    let mgr = manager.lock().await;
    let screen_after = mgr.get_screen(&session_id, false, false).await?;

    if screen_after.contains(&credential_value) && !screen_before.contains(&credential_value) {
        anyhow::bail!(
            "SECURITY WARNING: Credential echoed back in terminal! The {} was visible in the terminal output.",
            credential_type
        );
    }

    Ok(json!({
        "session_id": session_id,
        "device_hostname": device_hostname,
        "credential_type": credential_type,
        "matched_pattern": credential_entry.key,
        "status": "credential sent successfully",
        "echo_check": "passed - credential did not echo back"
    }))
}

// ---------------------------------------------------------------------------
// MCP protocol handler
// ---------------------------------------------------------------------------

fn mcp_success(id: Value, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(result),
        error: None,
    }
}

fn mcp_error(id: Value, code: i64, message: String) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(JsonRpcError { code, message }),
    }
}

async fn handle_request(
    req: JsonRpcRequest,
    manager: &Arc<Mutex<TerminalManager>>,
    work_dir: &PathBuf,
) -> Option<JsonRpcResponse> {
    let id = match &req.id {
        Some(id) => id.clone(),
        None => {
            // Notification – no response
            return None;
        }
    };

    match req.method.as_str() {
        "initialize" => Some(mcp_success(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "apchat-mcp-pty-server",
                    "version": "0.1.0"
                }
            }),
        )),

        "tools/list" => {
            let tools = tool_definitions();
            Some(mcp_success(id, json!({ "tools": tools })))
        }

        "tools/call" => {
            let tool_name = req.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = req.params.get("arguments").cloned().unwrap_or(json!({}));

            match handle_tool_call(manager, work_dir, tool_name, &arguments).await {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Some(mcp_success(
                        id,
                        json!({
                            "content": [{ "type": "text", "text": text }],
                            "isError": false
                        }),
                    ))
                }
                Err(e) => Some(mcp_success(
                    id,
                    json!({
                        "content": [{ "type": "text", "text": e.to_string() }],
                        "isError": true
                    }),
                )),
            }
        }

        _ => Some(mcp_error(
            id,
            -32601,
            format!("Method not found: {}", req.method),
        )),
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    let log_dir = std::env::var("PTY_LOG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("apchat-mcp-pty-logs"));
    std::fs::create_dir_all(&log_dir)?;

    let work_dir = std::env::var("PTY_WORK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let manager = Arc::new(Mutex::new(TerminalManager::new(log_dir)));

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let mut reader = BufReader::new(stdin);
    let mut writer = stdout;

    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break; // EOF
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => {
                let resp = mcp_error(Value::Null, -32700, format!("Parse error: {}", e));
                let msg = serde_json::to_string(&resp)? + "\n";
                writer.write_all(msg.as_bytes()).await?;
                writer.flush().await?;
                continue;
            }
        };

        if let Some(resp) = handle_request(req, &manager, &work_dir).await {
            let msg = serde_json::to_string(&resp)? + "\n";
            writer.write_all(msg.as_bytes()).await?;
            writer.flush().await?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definitions_count() {
        let tools = tool_definitions();
        assert_eq!(tools.len(), 12);
    }

    #[test]
    fn test_tool_definitions_names() {
        let tools = tool_definitions();
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();

        assert!(names.contains(&"pty_launch"));
        assert!(names.contains(&"pty_send_keys"));
        assert!(names.contains(&"pty_get_screen"));
        assert!(names.contains(&"pty_list"));
        assert!(names.contains(&"pty_kill"));
        assert!(names.contains(&"pty_get_cursor"));
        assert!(names.contains(&"pty_resize"));
        assert!(names.contains(&"pty_set_scrollback"));
        assert!(names.contains(&"pty_start_capture"));
        assert!(names.contains(&"pty_stop_capture"));
        assert!(names.contains(&"pty_request_user_input"));
        assert!(names.contains(&"pty_send_credential_keys"));
    }

    #[test]
    fn test_tool_definitions_have_input_schema() {
        let tools = tool_definitions();
        for tool in &tools {
            let schema = tool.get("inputSchema").expect("tool missing inputSchema");
            assert_eq!(schema.get("type").and_then(|v| v.as_str()), Some("object"));
            assert!(schema.get("properties").is_some());
        }
    }

    #[test]
    fn test_initialize_response() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let log_dir = std::env::temp_dir().join("mcp-pty-test-init");
            let _ = std::fs::create_dir_all(&log_dir);
            let manager = Arc::new(Mutex::new(TerminalManager::new(log_dir)));
            let work_dir = PathBuf::from("/tmp");

            let req = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(1)),
                method: "initialize".to_string(),
                params: json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "test", "version": "0.1.0" }
                }),
            };

            let resp = handle_request(req, &manager, &work_dir).await.unwrap();
            let result = resp.result.unwrap();
            assert_eq!(
                result.get("protocolVersion").and_then(|v| v.as_str()),
                Some("2024-11-05")
            );
            assert!(result.get("capabilities").is_some());
            assert_eq!(
                result
                    .get("serverInfo")
                    .and_then(|s| s.get("name"))
                    .and_then(|n| n.as_str()),
                Some("apchat-mcp-pty-server")
            );
        });
    }

    #[test]
    fn test_tools_list_response() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let log_dir = std::env::temp_dir().join("mcp-pty-test-list");
            let _ = std::fs::create_dir_all(&log_dir);
            let manager = Arc::new(Mutex::new(TerminalManager::new(log_dir)));
            let work_dir = PathBuf::from("/tmp");

            let req = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(2)),
                method: "tools/list".to_string(),
                params: json!({}),
            };

            let resp = handle_request(req, &manager, &work_dir).await.unwrap();
            let result = resp.result.unwrap();
            let tools = result.get("tools").and_then(|t| t.as_array()).unwrap();
            assert_eq!(tools.len(), 12);
        });
    }

    #[test]
    fn test_unknown_method() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let log_dir = std::env::temp_dir().join("mcp-pty-test-unknown");
            let _ = std::fs::create_dir_all(&log_dir);
            let manager = Arc::new(Mutex::new(TerminalManager::new(log_dir)));
            let work_dir = PathBuf::from("/tmp");

            let req = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(3)),
                method: "nonexistent/method".to_string(),
                params: json!({}),
            };

            let resp = handle_request(req, &manager, &work_dir).await.unwrap();
            assert!(resp.error.is_some());
            assert_eq!(resp.error.unwrap().code, -32601);
        });
    }

    #[test]
    fn test_notification_no_response() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let log_dir = std::env::temp_dir().join("mcp-pty-test-notify");
            let _ = std::fs::create_dir_all(&log_dir);
            let manager = Arc::new(Mutex::new(TerminalManager::new(log_dir)));
            let work_dir = PathBuf::from("/tmp");

            let req = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: None,
                method: "notifications/initialized".to_string(),
                params: json!({}),
            };

            let resp = handle_request(req, &manager, &work_dir).await;
            assert!(resp.is_none());
        });
    }

    #[test]
    fn test_unknown_tool_call() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let log_dir = std::env::temp_dir().join("mcp-pty-test-unknown-tool");
            let _ = std::fs::create_dir_all(&log_dir);
            let manager = Arc::new(Mutex::new(TerminalManager::new(log_dir)));
            let work_dir = PathBuf::from("/tmp");

            let req = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(4)),
                method: "tools/call".to_string(),
                params: json!({
                    "name": "nonexistent_tool",
                    "arguments": {}
                }),
            };

            let resp = handle_request(req, &manager, &work_dir).await.unwrap();
            let result = resp.result.unwrap();
            assert_eq!(result.get("isError").and_then(|v| v.as_bool()), Some(true));
        });
    }
}
