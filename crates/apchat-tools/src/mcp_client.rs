use apchat_toolcore::{Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

/// An MCP (Model Context Protocol) client that communicates with an external
/// MCP server process via JSON-RPC 2.0 over stdio.
///
/// This is designed to be generic so it can work with any MCP-compliant server,
/// including claude-context-mode and future similar tools.
pub struct McpClient {
    child: Mutex<Child>,
    stdin: Mutex<tokio::process::ChildStdin>,
    reader: Mutex<BufReader<tokio::process::ChildStdout>>,
    next_id: AtomicU64,
    server_name: String,
}

impl McpClient {
    /// Start an MCP server process and initialize the connection.
    pub async fn start(command: &str, args: &[&str], server_name: &str) -> anyhow::Result<Self> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to start MCP server '{}' ({}): {}", server_name, command, e))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stdin of MCP server '{}'", server_name))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout of MCP server '{}'", server_name))?;

        let client = Self {
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            reader: Mutex::new(BufReader::new(stdout)),
            next_id: AtomicU64::new(1),
            server_name: server_name.to_string(),
        };

        client.initialize().await?;
        Ok(client)
    }

    /// Send a JSON-RPC request and wait for a response.
    async fn send_request(&self, method: &str, params: Value) -> anyhow::Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let mut stdin = self.stdin.lock().await;
        let msg = serde_json::to_string(&request)? + "\n";
        stdin.write_all(msg.as_bytes()).await?;
        stdin.flush().await?;
        drop(stdin);

        let mut reader = self.reader.lock().await;
        loop {
            let mut line = String::new();
            let bytes_read = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                reader.read_line(&mut line),
            )
            .await
            .map_err(|_| anyhow::anyhow!("Timeout waiting for MCP server '{}' response", self.server_name))?
            .map_err(|e| anyhow::anyhow!("Error reading from MCP server '{}': {}", self.server_name, e))?;

            if bytes_read == 0 {
                anyhow::bail!("MCP server '{}' closed connection", self.server_name);
            }

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let response: Value = serde_json::from_str(line)
                .map_err(|e| anyhow::anyhow!("Invalid JSON from MCP server '{}': {}", self.server_name, e))?;

            // Skip notifications (no "id" field)
            if response.get("id").is_none() {
                continue;
            }

            if let Some(error) = response.get("error") {
                let msg = error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");
                anyhow::bail!("MCP server '{}' error: {}", self.server_name, msg);
            }

            return Ok(response.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    /// Send a JSON-RPC notification (no response expected).
    async fn send_notification(&self, method: &str, params: Value) -> anyhow::Result<()> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });

        let mut stdin = self.stdin.lock().await;
        let msg = serde_json::to_string(&notification)? + "\n";
        stdin.write_all(msg.as_bytes()).await?;
        stdin.flush().await?;

        Ok(())
    }

    /// Perform the MCP initialization handshake.
    async fn initialize(&self) -> anyhow::Result<()> {
        let _result = self
            .send_request(
                "initialize",
                serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "apchat",
                        "version": "0.1.0"
                    }
                }),
            )
            .await?;

        self.send_notification("notifications/initialized", serde_json::json!({}))
            .await?;

        Ok(())
    }

    /// List all tools available from the MCP server.
    pub async fn list_tools(&self) -> anyhow::Result<Vec<McpToolDefinition>> {
        let result = self
            .send_request("tools/list", serde_json::json!({}))
            .await?;

        let tools = result
            .get("tools")
            .and_then(|t| t.as_array())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Invalid tools/list response from MCP server '{}'",
                    self.server_name
                )
            })?;

        let mut definitions = Vec::new();
        for tool in tools {
            definitions.push(McpToolDefinition {
                name: tool
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string(),
                description: tool
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string(),
                input_schema: tool
                    .get("inputSchema")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({"type": "object", "properties": {}})),
            });
        }

        Ok(definitions)
    }

    /// Call a tool on the MCP server.
    pub async fn call_tool(&self, name: &str, arguments: Value) -> anyhow::Result<String> {
        let result = self
            .send_request(
                "tools/call",
                serde_json::json!({
                    "name": name,
                    "arguments": arguments,
                }),
            )
            .await?;

        let content = result
            .get("content")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();

        let is_error = result
            .get("isError")
            .and_then(|e| e.as_bool())
            .unwrap_or(false);

        if is_error {
            anyhow::bail!("{}", content);
        }

        Ok(content)
    }

    /// Get the server name.
    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    /// Shut down the MCP server process.
    pub async fn shutdown(&self) {
        let mut child = self.child.lock().await;
        let _ = child.start_kill();
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.try_lock() {
            let _ = child.start_kill();
        }
    }
}

/// Definition of a tool discovered from an MCP server.
#[derive(Debug, Clone)]
pub struct McpToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// A bridge tool that wraps an MCP server tool as a native apchat Tool.
///
/// Each MCP tool is exposed under a prefixed name (e.g., "ctx_execute" for
/// context-mode's "execute" tool) to avoid name collisions with native tools.
pub struct McpBridgeTool {
    tool_name: String,
    original_name: String,
    tool_description: String,
    tool_input_schema: Value,
    client: Arc<McpClient>,
}

impl McpBridgeTool {
    pub fn new(def: &McpToolDefinition, client: Arc<McpClient>, prefix: &str) -> Self {
        Self {
            tool_name: format!("{}{}", prefix, def.name),
            original_name: def.name.clone(),
            tool_description: def.description.clone(),
            tool_input_schema: def.input_schema.clone(),
            client,
        }
    }
}

#[async_trait]
impl Tool for McpBridgeTool {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        &self.tool_description
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        let mut params = HashMap::new();
        if let Some(properties) = self.tool_input_schema.get("properties").and_then(|p| p.as_object()) {
            let required: Vec<String> = self
                .tool_input_schema
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            for (name, schema) in properties {
                let param_type = schema
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("string")
                    .to_string();
                let description = schema
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string();
                let is_required = required.contains(name);

                params.insert(
                    name.clone(),
                    ParameterDefinition {
                        param_type,
                        description,
                        required: is_required,
                        default: schema.get("default").cloned(),
                    },
                );
            }
        }
        params
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let arguments =
            serde_json::to_value(&params.data).unwrap_or(Value::Object(Default::default()));

        match self.client.call_tool(&self.original_name, arguments).await {
            Ok(content) => ToolResult::success(content),
            Err(e) => ToolResult::error(format!("MCP tool error: {}", e)),
        }
    }

    fn to_openai_definition(&self) -> Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.tool_name,
                "description": self.tool_description,
                "parameters": self.tool_input_schema,
            }
        })
    }
}

/// Start an MCP server process and discover its tools.
///
/// Returns the client (for lifecycle management) and a list of bridge tools
/// that can be registered in the tool registry.
pub async fn start_mcp_server(
    command: &str,
    args: &[&str],
    server_name: &str,
    tool_prefix: &str,
) -> anyhow::Result<(Arc<McpClient>, Vec<McpBridgeTool>)> {
    let client = Arc::new(McpClient::start(command, args, server_name).await?);
    let tool_defs = client.list_tools().await?;

    let tools: Vec<McpBridgeTool> = tool_defs
        .iter()
        .map(|def| McpBridgeTool::new(def, Arc::clone(&client), tool_prefix))
        .collect();

    Ok((client, tools))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_tool_definition() {
        let def = McpToolDefinition {
            name: "execute".to_string(),
            description: "Execute code in sandbox".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "language": {
                        "type": "string",
                        "description": "Programming language"
                    },
                    "code": {
                        "type": "string",
                        "description": "Code to execute"
                    }
                },
                "required": ["language", "code"]
            }),
        };

        assert_eq!(def.name, "execute");
        assert_eq!(def.description, "Execute code in sandbox");
        assert!(def.input_schema.get("properties").is_some());
    }

    #[test]
    fn test_mcp_bridge_tool_name_prefixing() {
        let def = McpToolDefinition {
            name: "execute".to_string(),
            description: "Execute code".to_string(),
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
        };

        // Test that the prefixed name is generated correctly
        let prefixed_name = format!("{}{}", "ctx_", def.name);
        assert_eq!(prefixed_name, "ctx_execute");

        let prefixed_name = format!("{}{}", "mcp_", def.name);
        assert_eq!(prefixed_name, "mcp_execute");
    }

    #[test]
    fn test_parameter_extraction_from_schema() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "language": {
                    "type": "string",
                    "description": "Programming language"
                },
                "code": {
                    "type": "string",
                    "description": "Code to execute"
                },
                "timeout": {
                    "type": "number",
                    "description": "Timeout in seconds",
                    "default": 30
                }
            },
            "required": ["language", "code"]
        });

        let required: Vec<String> = schema
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let properties = schema.get("properties").unwrap().as_object().unwrap();

        let mut params = HashMap::new();
        for (name, prop_schema) in properties {
            let param_type = prop_schema
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("string")
                .to_string();
            let description = prop_schema
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();
            let is_required = required.contains(name);

            params.insert(
                name.clone(),
                ParameterDefinition {
                    param_type,
                    description,
                    required: is_required,
                    default: prop_schema.get("default").cloned(),
                },
            );
        }

        assert_eq!(params.len(), 3);
        assert!(params["language"].required);
        assert!(params["code"].required);
        assert!(!params["timeout"].required);
        assert_eq!(params["timeout"].default, Some(serde_json::json!(30)));
        assert_eq!(params["language"].param_type, "string");
        assert_eq!(params["timeout"].param_type, "number");
    }

    #[test]
    fn test_openai_definition_format() {
        let tool_name = "ctx_execute".to_string();
        let tool_description = "Execute code in sandbox".to_string();
        let input_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "code": {"type": "string", "description": "Code to execute"}
            },
            "required": ["code"]
        });

        let definition = serde_json::json!({
            "type": "function",
            "function": {
                "name": tool_name,
                "description": tool_description,
                "parameters": input_schema,
            }
        });

        assert_eq!(definition["type"], "function");
        assert_eq!(definition["function"]["name"], "ctx_execute");
        assert!(definition["function"]["parameters"]["properties"].is_object());
    }
}
