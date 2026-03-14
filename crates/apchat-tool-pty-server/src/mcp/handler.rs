//! MCP request handler

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use apchat_terminal::TerminalManager;

use crate::mcp::{JsonRpcRequest, JsonRpcResponse};

/// Handler for MCP requests
pub struct McpHandler {
    manager: Arc<Mutex<TerminalManager>>,
    work_dir: PathBuf,
}

impl McpHandler {
    /// Create a new handler
    pub fn new(manager: Arc<Mutex<TerminalManager>>, work_dir: PathBuf) -> Self {
        Self { manager, work_dir }
    }

    /// Handle an incoming JSON-RPC request
    pub async fn handle_request(
        &self,
        req: JsonRpcRequest,
    ) -> Result<Option<JsonRpcResponse>> {
        let id = req.id.clone().unwrap_or(json!(null));

        // Handle initialize request
        if req.method == "initialize" {
            return Ok(Some(self.handle_initialize(req.params, id).await?));
        }

        // Handle tools/list request
        if req.method == "tools/list" {
            return Ok(Some(self.handle_tools_list(id).await?));
        }

        // Handle tools/call request
        if req.method == "tools/call" {
            return Ok(Some(self.handle_tools_call(req.params, id).await?));
        }

        // Handle ping request
        if req.method == "ping" {
            return Ok(Some(self.handle_ping(id.clone()).await?));
        }

        // Unknown method
        Ok(Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: id.clone(),
            result: None,
            error: Some(crate::mcp::JsonRpcError::method_not_found(
                id,
                req.method,
            )),
        }))
    }

    /// Handle initialize request
    async fn handle_initialize(&self, params: Value, id: Value) -> Result<JsonRpcResponse> {
        let protocol_version = params
            .get("protocolVersion")
            .and_then(|v| v.as_str())
            .unwrap_or("2024-11-05");

        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: id.clone(),
            result: Some(json!({
                "protocolVersion": protocol_version,
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "apchat-mcp-pty-server",
                    "version": "0.1.0"
                }
            })),
            error: None,
        })
    }

    /// Handle tools/list request
    async fn handle_tools_list(&self, id: Value) -> Result<JsonRpcResponse> {
        use crate::mcp::tool_definitions;

        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: id.clone(),
            result: Some(json!({
                "tools": tool_definitions()
            })),
            error: None,
        })
    }

    /// Handle tools/call request
    async fn handle_tools_call(&self, params: Value, id: Value) -> Result<JsonRpcResponse> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing tool name"))?;

        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(json!({}));

        let result = match name {
            "pty_launch" => self.handle_pty_launch(&arguments).await,
            "pty_send_keys" => self.handle_pty_send_keys(&arguments).await,
            "pty_get_screen" => self.handle_pty_get_screen(&arguments).await,
            "pty_list" => self.handle_pty_list().await,
            "pty_kill" => self.handle_pty_kill(&arguments).await,
            "pty_get_cursor" => self.handle_pty_get_cursor(&arguments).await,
            "pty_resize" => self.handle_pty_resize(&arguments).await,
            "pty_set_scrollback" => self.handle_pty_set_scrollback(&arguments).await,
            "pty_start_capture" => self.handle_pty_start_capture(&arguments).await,
            "pty_stop_capture" => self.handle_pty_stop_capture(&arguments).await,
            "pty_request_user_input" => self.handle_pty_request_user_input(&arguments).await,
            "pty_send_credential_keys" => self.handle_pty_send_credential_keys(&arguments).await,
            _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
        };

        match result {
            Ok(result_value) => Ok(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: id.clone(),
                result: Some(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": result_value.to_string()
                        }
                    ]
                })),
                error: None,
            }),
            Err(e) => {
                let err_resp = crate::mcp::JsonRpcError::internal_error(
                    id.clone(),
                    e.to_string(),
                );
                Ok(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(err_resp),
                })
            }
        }
    }

    /// Handle ping request
    async fn handle_ping(&self, id: Value) -> Result<JsonRpcResponse> {
        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: id.clone(),
            result: Some(json!({})),
            error: None,
        })
    }

    // Tool handlers
    async fn handle_pty_launch(&self, args: &Value) -> Result<Value> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let working_dir_str = args
            .get("working_dir")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let cols = args
            .get("cols")
            .and_then(|v| v.as_i64())
            .unwrap_or(80) as u16;
        let rows = args
            .get("rows")
            .and_then(|v| v.as_i64())
            .unwrap_or(24) as u16;

        let working_dir = if let Some(dir_str) = &working_dir_str {
            Some(self.work_dir.join(dir_str).display().to_string())
        } else {
            Some(self.work_dir.display().to_string())
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

        let mut mgr = self.manager.lock().await;
        let returned_id = mgr
            .create_session(
                session_id.clone(),
                command,
                working_dir.clone(),
                cols,
                rows,
            )
            .await?;

        Ok(json!({
            "session_id": returned_id,
            "command": command_for_display,
            "working_dir": working_dir.unwrap_or_else(|| self.work_dir.display().to_string()),
            "size": [cols, rows],
        }))
    }

    async fn handle_pty_send_keys(&self, args: &Value) -> Result<Value> {
        let session_id = args
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing session_id"))?
            .to_string();
        let mut keys = args
            .get("keys")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing keys"))?
            .to_string();
        let raw = args
            .get("raw")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut mgr = self.manager.lock().await;
        if !raw {
            keys = format!("{}\n", keys);
        }
        mgr.send_input(&session_id, &keys).await?;

        Ok(json!({ "status": format!("Keys sent to session {}", session_id) }))
    }

    async fn handle_pty_get_screen(&self, args: &Value) -> Result<Value> {
        let session_id = args
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing session_id"))?
            .to_string();
        let include_colors = args
            .get("include_colors")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let include_cursor = args
            .get("include_cursor")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let mgr = self.manager.lock().await;
        let contents = mgr.get_screen(&session_id, include_colors, include_cursor).await?;
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

    async fn handle_pty_list(&self) -> Result<Value> {
        let mgr = self.manager.lock().await;
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

    async fn handle_pty_kill(&self, args: &Value) -> Result<Value> {
        let session_id = args
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing session_id"))?
            .to_string();

        let mut mgr = self.manager.lock().await;
        mgr.kill_session(&session_id).await?;

        Ok(json!({ "status": format!("Session {} killed successfully", session_id) }))
    }

    async fn handle_pty_get_cursor(&self, args: &Value) -> Result<Value> {
        let session_id = args
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing session_id"))?
            .to_string();

        let mgr = self.manager.lock().await;
        let (row, col) = mgr.get_cursor_position(&session_id).await?;

        Ok(json!({
            "session_id": session_id,
            "position": [col, row],
        }))
    }

    async fn handle_pty_resize(&self, args: &Value) -> Result<Value> {
        let session_id = args
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing session_id"))?
            .to_string();
        let cols = args
            .get("cols")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| anyhow::anyhow!("Missing cols"))? as u16;
        let rows = args
            .get("rows")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| anyhow::anyhow!("Missing rows"))? as u16;

        let mut mgr = self.manager.lock().await;
        mgr.resize_session(&session_id, rows, cols).await?;

        Ok(json!({
            "session_id": session_id,
            "size": [cols, rows],
        }))
    }

    async fn handle_pty_set_scrollback(&self, args: &Value) -> Result<Value> {
        let session_id = args
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing session_id"))?
            .to_string();
        let lines = args
            .get("lines")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| anyhow::anyhow!("Missing lines"))? as usize;

        let mut mgr = self.manager.lock().await;
        mgr.set_scrollback(&session_id, lines).await?;

        Ok(json!({
            "session_id": session_id,
            "scrollback_lines": lines,
        }))
    }

    async fn handle_pty_start_capture(&self, args: &Value) -> Result<Value> {
        let session_id = args
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing session_id"))?
            .to_string();

        let mut mgr = self.manager.lock().await;
        // Generate a default output file path
        let output_file = std::path::PathBuf::from(format!("/tmp/pty-capture-{}.log", session_id));
        mgr.capture_start(&session_id, output_file).await?;

        Ok(json!({ "status": format!("Capture started for session {}", session_id) }))
    }

    async fn handle_pty_stop_capture(&self, args: &Value) -> Result<Value> {
        let session_id = args
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing session_id"))?
            .to_string();

        let mut mgr = self.manager.lock().await;
        let capture_path = std::path::PathBuf::from(format!("/tmp/pty-capture-{}.log", session_id));
        let (capture_file, bytes, duration) = mgr.capture_stop(&session_id, capture_path).await?;

        Ok(json!({
            "status": format!("Capture stopped for session {}", session_id),
            "file_path": capture_file,
            "bytes_captured": bytes,
            "duration_seconds": duration,
        }))
    }

    async fn handle_pty_request_user_input(
        &self,
        _args: &Value,
    ) -> Result<Value> {
        // This tool requires user interaction and cannot be handled in MCP context
        Ok(json!({
            "error": "pty_request_user_input requires interactive user input and is not available in MCP mode"
        }))
    }

    async fn handle_pty_send_credential_keys(&self, _args: &Value) -> Result<Value> {
        // This tool requires credential file access and cannot be handled in MCP context
        Ok(json!({
            "error": "pty_send_credential_keys requires credential file access and is not available in MCP mode"
        }))
    }
}