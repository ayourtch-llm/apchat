use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use base64::Engine;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "apchat-ocr", about = "OCR tool using GLM-OCR")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse one or more images for OCR
    Parse {
        /// Image file paths or URLs to parse
        #[arg(required = true)]
        images: Vec<String>,

        /// GLM-OCR API base URL (e.g. http://localhost:8080)
        #[arg(long, env = "GLM_OCR_API_URL", default_value = "http://localhost:8080")]
        api_url: String,

        /// API key (if required)
        #[arg(long, env = "GLM_OCR_API_KEY")]
        api_key: Option<String>,

        /// Model name served by the endpoint
        #[arg(long, default_value = "glm-ocr")]
        model: String,

        /// Output format: markdown, json, or text (raw response)
        #[arg(long, default_value = "markdown")]
        format: String,

        /// Output directory (writes results to files instead of stdout)
        #[arg(long, short)]
        output: Option<PathBuf>,
    },

    /// Run as an MCP server (JSON-RPC 2.0 over stdio)
    Serve {
        /// GLM-OCR API base URL
        #[arg(long, env = "GLM_OCR_API_URL", default_value = "http://localhost:8080")]
        api_url: String,

        /// API key (if required)
        #[arg(long, env = "GLM_OCR_API_KEY")]
        api_key: Option<String>,

        /// Model name served by the endpoint
        #[arg(long, default_value = "glm-ocr")]
        model: String,
    },
}

// ---------------------------------------------------------------------------
// GLM-OCR client (OpenAI-compatible chat completions API)
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct OcrClient {
    http: reqwest::Client,
    api_url: String,
    api_key: Option<String>,
    model: String,
}

impl OcrClient {
    fn new(api_url: String, api_key: Option<String>, model: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_url,
            api_key,
            model,
        }
    }

    async fn parse_image(&self, image_source: &str) -> Result<String> {
        let image_url = self.resolve_image_url(image_source).await?;

        let body = json!({
            "model": self.model,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "image_url",
                            "image_url": { "url": image_url }
                        },
                        {
                            "type": "text",
                            "text": "OCR this image. Return the full text content."
                        }
                    ]
                }
            ],
            "max_tokens": 16384,
            "temperature": 0.01
        });

        let url = format!(
            "{}/v1/chat/completions",
            self.api_url.trim_end_matches('/')
        );

        let mut req = self.http.post(&url).json(&body);
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }

        let resp = req
            .send()
            .await
            .context("Failed to connect to GLM-OCR API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("GLM-OCR API returned {}: {}", status, text);
        }

        let data: ChatCompletionResponse = resp
            .json()
            .await
            .context("Failed to parse GLM-OCR API response")?;

        data.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| anyhow::anyhow!("GLM-OCR API returned no choices"))
    }

    async fn resolve_image_url(&self, source: &str) -> Result<String> {
        if source.starts_with("http://") || source.starts_with("https://") {
            return Ok(source.to_string());
        }

        if source.starts_with("data:") {
            return Ok(source.to_string());
        }

        // Local file – read and base64-encode as a data URI
        let path = PathBuf::from(source);
        let data = tokio::fs::read(&path)
            .await
            .with_context(|| format!("Failed to read image file: {}", path.display()))?;

        let mime = mime_from_extension(&path);
        let encoded = base64::engine::general_purpose::STANDARD.encode(&data);
        Ok(format!("data:{};base64,{}", mime, encoded))
    }
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: String,
}

fn mime_from_extension(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("tiff") | Some("tif") => "image/tiff",
        Some("svg") => "image/svg+xml",
        Some("pdf") => "application/pdf",
        _ => "application/octet-stream",
    }
}

// ---------------------------------------------------------------------------
// CLI parse command
// ---------------------------------------------------------------------------

async fn run_parse(
    images: Vec<String>,
    api_url: String,
    api_key: Option<String>,
    model: String,
    format: String,
    output: Option<PathBuf>,
) -> Result<()> {
    let client = OcrClient::new(api_url, api_key, model);

    if let Some(ref dir) = output {
        tokio::fs::create_dir_all(dir)
            .await
            .with_context(|| format!("Failed to create output directory: {}", dir.display()))?;
    }

    for (i, image) in images.iter().enumerate() {
        let result = client
            .parse_image(image)
            .await
            .with_context(|| format!("Failed to parse image: {}", image))?;

        let text = format_result(&result, &format);

        if let Some(ref dir) = output {
            let ext = match format.as_str() {
                "json" => "json",
                _ => "md",
            };
            let stem = PathBuf::from(image)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("result_{}", i));
            let out_path = dir.join(format!("{}.{}", stem, ext));
            tokio::fs::write(&out_path, &text)
                .await
                .with_context(|| format!("Failed to write output: {}", out_path.display()))?;
            eprintln!("Wrote: {}", out_path.display());
        } else {
            if images.len() > 1 {
                println!("--- {} ---", image);
            }
            println!("{}", text);
        }
    }

    Ok(())
}

fn format_result(raw: &str, format: &str) -> String {
    match format {
        "json" => {
            // Try to extract JSON from the response, or wrap as JSON
            if let Ok(v) = serde_json::from_str::<Value>(raw) {
                serde_json::to_string_pretty(&v).unwrap_or_else(|_| raw.to_string())
            } else {
                serde_json::to_string_pretty(&json!({ "text": raw }))
                    .unwrap_or_else(|_| raw.to_string())
            }
        }
        "text" => raw.to_string(),
        _ => raw.to_string(), // markdown is the raw output from GLM-OCR
    }
}

// ---------------------------------------------------------------------------
// JSON-RPC types (MCP protocol)
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
// MCP tool schema helpers
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
    vec![tool_def(
        "ocr_parse",
        "Parse an image using GLM-OCR and return the extracted text. Accepts a local file path, a URL, or a base64 data URI.",
        json!({
            "image": prop("string", "Image source: local file path, HTTP(S) URL, or data URI"),
            "format": prop("string", "Output format: markdown (default), json, or text"),
        }),
        vec!["image"],
    )]
}

// ---------------------------------------------------------------------------
// MCP tool execution
// ---------------------------------------------------------------------------

async fn handle_tool_call(client: &OcrClient, name: &str, args: &Value) -> Result<Value> {
    match name {
        "ocr_parse" => {
            let image = args
                .get("image")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing required parameter 'image'"))?;

            let format = args
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("markdown");

            let raw = client.parse_image(image).await?;
            let text = format_result(&raw, format);
            Ok(json!({ "text": text }))
        }
        _ => anyhow::bail!("Unknown tool: {}", name),
    }
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

async fn handle_request(req: JsonRpcRequest, client: &OcrClient) -> Option<JsonRpcResponse> {
    let id = match &req.id {
        Some(id) => id.clone(),
        None => return None, // notification
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
                    "name": "apchat-ocr",
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

            match handle_tool_call(client, tool_name, &arguments).await {
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
// MCP serve loop
// ---------------------------------------------------------------------------

async fn run_serve(api_url: String, api_key: Option<String>, model: String) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let client = OcrClient::new(api_url, api_key, model);

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

        if let Some(resp) = handle_request(req, &client).await {
            let msg = serde_json::to_string(&resp)? + "\n";
            writer.write_all(msg.as_bytes()).await?;
            writer.flush().await?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse {
            images,
            api_url,
            api_key,
            model,
            format,
            output,
        } => run_parse(images, api_url, api_key, model, format, output).await,
        Commands::Serve {
            api_url,
            api_key,
            model,
        } => run_serve(api_url, api_key, model).await,
    }
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
        assert_eq!(tools.len(), 1);
    }

    #[test]
    fn test_tool_definitions_names() {
        let tools = tool_definitions();
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();
        assert!(names.contains(&"ocr_parse"));
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
    fn test_ocr_parse_required_params() {
        let tools = tool_definitions();
        let ocr_tool = &tools[0];
        let required = ocr_tool
            .get("inputSchema")
            .and_then(|s| s.get("required"))
            .and_then(|r| r.as_array())
            .unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0].as_str(), Some("image"));
    }

    #[test]
    fn test_initialize_response() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = OcrClient::new(
                "http://localhost:8080".to_string(),
                None,
                "glm-ocr".to_string(),
            );

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

            let resp = handle_request(req, &client).await.unwrap();
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
                Some("apchat-ocr")
            );
        });
    }

    #[test]
    fn test_tools_list_response() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = OcrClient::new(
                "http://localhost:8080".to_string(),
                None,
                "glm-ocr".to_string(),
            );

            let req = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(2)),
                method: "tools/list".to_string(),
                params: json!({}),
            };

            let resp = handle_request(req, &client).await.unwrap();
            let result = resp.result.unwrap();
            let tools = result.get("tools").and_then(|t| t.as_array()).unwrap();
            assert_eq!(tools.len(), 1);
        });
    }

    #[test]
    fn test_unknown_method() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = OcrClient::new(
                "http://localhost:8080".to_string(),
                None,
                "glm-ocr".to_string(),
            );

            let req = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(3)),
                method: "nonexistent/method".to_string(),
                params: json!({}),
            };

            let resp = handle_request(req, &client).await.unwrap();
            assert!(resp.error.is_some());
            assert_eq!(resp.error.unwrap().code, -32601);
        });
    }

    #[test]
    fn test_notification_no_response() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = OcrClient::new(
                "http://localhost:8080".to_string(),
                None,
                "glm-ocr".to_string(),
            );

            let req = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: None,
                method: "notifications/initialized".to_string(),
                params: json!({}),
            };

            let resp = handle_request(req, &client).await;
            assert!(resp.is_none());
        });
    }

    #[test]
    fn test_unknown_tool_call() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = OcrClient::new(
                "http://localhost:8080".to_string(),
                None,
                "glm-ocr".to_string(),
            );

            let req = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(4)),
                method: "tools/call".to_string(),
                params: json!({
                    "name": "nonexistent_tool",
                    "arguments": {}
                }),
            };

            let resp = handle_request(req, &client).await.unwrap();
            let result = resp.result.unwrap();
            assert_eq!(result.get("isError").and_then(|v| v.as_bool()), Some(true));
        });
    }

    #[test]
    fn test_mime_from_extension() {
        assert_eq!(mime_from_extension(&PathBuf::from("test.png")), "image/png");
        assert_eq!(
            mime_from_extension(&PathBuf::from("test.jpg")),
            "image/jpeg"
        );
        assert_eq!(
            mime_from_extension(&PathBuf::from("test.jpeg")),
            "image/jpeg"
        );
        assert_eq!(
            mime_from_extension(&PathBuf::from("test.gif")),
            "image/gif"
        );
        assert_eq!(
            mime_from_extension(&PathBuf::from("test.webp")),
            "image/webp"
        );
        assert_eq!(
            mime_from_extension(&PathBuf::from("test.pdf")),
            "application/pdf"
        );
        assert_eq!(
            mime_from_extension(&PathBuf::from("test.xyz")),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_format_result_markdown() {
        let raw = "# Hello\n\nSome text";
        assert_eq!(format_result(raw, "markdown"), raw);
    }

    #[test]
    fn test_format_result_text() {
        let raw = "plain text content";
        assert_eq!(format_result(raw, "text"), raw);
    }

    #[test]
    fn test_format_result_json_valid() {
        let raw = r#"{"key": "value"}"#;
        let result = format_result(raw, "json");
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.get("key").and_then(|v| v.as_str()), Some("value"));
    }

    #[test]
    fn test_format_result_json_wraps_plain_text() {
        let raw = "not json";
        let result = format_result(raw, "json");
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.get("text").and_then(|v| v.as_str()), Some("not json"));
    }

    #[test]
    fn test_resolve_url_passthrough() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = OcrClient::new(
                "http://localhost:8080".to_string(),
                None,
                "glm-ocr".to_string(),
            );
            let url = "https://example.com/image.png";
            let resolved = client.resolve_image_url(url).await.unwrap();
            assert_eq!(resolved, url);
        });
    }

    #[test]
    fn test_resolve_data_uri_passthrough() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = OcrClient::new(
                "http://localhost:8080".to_string(),
                None,
                "glm-ocr".to_string(),
            );
            let uri = "data:image/png;base64,iVBOR";
            let resolved = client.resolve_image_url(uri).await.unwrap();
            assert_eq!(resolved, uri);
        });
    }

    #[test]
    fn test_resolve_local_file() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = OcrClient::new(
                "http://localhost:8080".to_string(),
                None,
                "glm-ocr".to_string(),
            );

            // Create a temp file with known content
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("test.png");
            std::fs::write(&file, b"fake png data").unwrap();

            let resolved = client
                .resolve_image_url(file.to_str().unwrap())
                .await
                .unwrap();
            assert!(resolved.starts_with("data:image/png;base64,"));

            // Verify base64 content
            let b64_part = resolved.strip_prefix("data:image/png;base64,").unwrap();
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(b64_part)
                .unwrap();
            assert_eq!(decoded, b"fake png data");
        });
    }
}
