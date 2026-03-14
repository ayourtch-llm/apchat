//! MCP Server Binary Entry Point
//!
//! This binary provides a standalone MCP server for PTY operations.
//! It reads JSON-RPC 2.0 requests from stdin and writes responses to stdout.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

use apchat_terminal::TerminalManager;
use apchat_tool_pty_server::mcp::{JsonRpcRequest, JsonRpcResponse};
use apchat_tool_pty_server::McpHandler;

// ---------------------------------------------------------------------------
// JSON-RPC helpers
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
        error: Some(apchat_tool_pty_server::mcp::JsonRpcError { code, message }),
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = apchat_tool_pty_server::Config::from_env();
    let log_dir = config.log_dir().clone();
    std::fs::create_dir_all(&log_dir)?;

    let work_dir = std::env::var("PTY_WORK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let manager = Arc::new(Mutex::new(TerminalManager::new(log_dir)));
    let handler = McpHandler::new(manager, work_dir);

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

        if let Some(resp) = handler.handle_request(req).await? {
            let msg = serde_json::to_string(&resp)? + "\n";
            writer.write_all(msg.as_bytes()).await?;
            writer.flush().await?;
        }
    }

    Ok(())
}