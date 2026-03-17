//! Integration tests for the chat() session loop using mockllm.
//!
//! These tests verify the core tool-calling loop by pointing APChat at a
//! MockLlmServer that returns scripted responses (tool calls, text, etc.).

use apchat::{APChat, config::ClientConfig};
use apchat::chat::session;
use apchat_models::ModelColor;
use apchat_models::types::ContentPart;
use apchat_toolcore::{Tool, ToolParameters, ToolResult, ParameterDefinition, ToolRegistry, param};
use async_trait::async_trait;
use mockllm::{MockLlmServer, Response};
use tempfile::TempDir;

// ─── Test tools ─────────────────────────────────────────────────────────────

/// A simple tool that echoes its input — used to test argument passing.
struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str { "echo" }
    fn description(&self) -> &str { "Echoes the input message" }
    fn parameters(&self) -> std::collections::HashMap<String, ParameterDefinition> {
        std::collections::HashMap::from([
            param!("message", "string", "The message to echo", required),
        ])
    }
    async fn execute(&self, params: ToolParameters, _ctx: &apchat_toolcore::tool_context::ToolContext) -> ToolResult {
        match params.get_required::<String>("message") {
            Ok(msg) => ToolResult::success(format!("echo: {}", msg)),
            Err(e) => ToolResult::error(e.to_string()),
        }
    }
}

/// A tool that always returns a fixed result — simulates read_file etc.
struct FixedTool { tool_name: String, result: String }

#[async_trait]
impl Tool for FixedTool {
    fn name(&self) -> &str { &self.tool_name }
    fn description(&self) -> &str { "Returns a fixed result" }
    fn parameters(&self) -> std::collections::HashMap<String, ParameterDefinition> {
        std::collections::HashMap::from([
            param!("file_path", "string", "Path", required),
        ])
    }
    async fn execute(&self, _params: ToolParameters, _ctx: &apchat_toolcore::tool_context::ToolContext) -> ToolResult {
        ToolResult::success(self.result.clone())
    }
}

// ─── Test helpers ────────────────────────────────────────────────────────────

/// Get the base URL (without /v1) for configuring APChat.
/// APChat's normalize_api_url will append /v1/chat/completions.
fn mock_base_url(mock: &MockLlmServer) -> String {
    format!("http://127.0.0.1:{}/", mock.port())
}

fn create_test_chat(mock_url: &str) -> APChat {
    let temp_dir = TempDir::new().unwrap();
    let work_dir = temp_dir.path().to_path_buf();

    let mut client_config = ClientConfig::new();
    for color in [ModelColor::BluModel, ModelColor::GrnModel, ModelColor::RedModel] {
        client_config.set_api_url(color, Some(mock_url.to_string()));
        client_config.set_model_override(color, Some("mock-model".to_string()));
    }

    let mut registry = ToolRegistry::new();
    registry.register(EchoTool);
    registry.register(FixedTool {
        tool_name: "read_file".to_string(),
        result: "fn main() { println!(\"hello\"); }".to_string(),
    });
    registry.register(FixedTool {
        tool_name: "search_files".to_string(),
        result: "Found 3 matches in src/".to_string(),
    });

    let mut chat = APChat::new_for_test(work_dir, client_config, registry);
    chat.push_message(apchat_models::Message {
        role: "system".to_string(),
        content: vec![ContentPart::Text("You are a test assistant.".to_string())],
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning: None,
    });
    chat.set_task_completion_marker(Some("__TEST_DONE__".to_string()));
    chat
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_chat_simple_text_response() {
    let mock = MockLlmServer::builder()
        .next(Response::text("Hello! I'm here to help. __TEST_DONE__"))
        .build()
        .await;

    let mut chat = create_test_chat(&mock_base_url(&mock));
    let result = session::chat(&mut chat, "Hi there", None).await;

    assert!(result.is_ok(), "chat() should succeed: {:?}", result.err());
    let response = result.unwrap();
    assert!(response.contains("Hello"), "Should contain greeting, got: {}", response);
    assert_eq!(mock.request_count(), 1);
}

#[tokio::test]
async fn test_chat_single_tool_call_then_text() {
    let mock = MockLlmServer::builder()
        .next(Response::tool_call("read_file", serde_json::json!({"file_path": "src/main.rs"})))
        .next(Response::text("The file contains a main function. __TEST_DONE__"))
        .build()
        .await;

    let mut chat = create_test_chat(&mock_base_url(&mock));
    let result = session::chat(&mut chat, "Read src/main.rs", None).await;

    assert!(result.is_ok(), "chat() should succeed: {:?}", result.err());
    let response = result.unwrap();
    assert!(response.contains("main function"), "Response: {}", response);

    // Should have made 2 API calls (tool call + final response)
    assert_eq!(mock.request_count(), 2);

    // Second request should contain the tool result
    let msgs = mock.request_messages(1);
    let tool_msg = msgs.iter().find(|m| m["role"] == "tool");
    assert!(tool_msg.is_some(), "Second request should contain tool result message");
}

#[tokio::test]
async fn test_chat_multiple_tool_calls_in_sequence() {
    let mock = MockLlmServer::builder()
        .next(Response::tool_call("read_file", serde_json::json!({"file_path": "a.rs"})))
        .next(Response::tool_call("search_files", serde_json::json!({"file_path": "pattern"})))
        .next(Response::text("Found everything. __TEST_DONE__"))
        .build()
        .await;

    let mut chat = create_test_chat(&mock_base_url(&mock));
    let result = session::chat(&mut chat, "Analyze the code", None).await;

    assert!(result.is_ok(), "chat() should succeed: {:?}", result.err());
    assert_eq!(mock.request_count(), 3);
}

#[tokio::test]
async fn test_chat_echo_tool_passes_arguments() {
    let mock = MockLlmServer::builder()
        .next(Response::tool_call("echo", serde_json::json!({"message": "test123"})))
        .next(Response::text("Got the echo result. __TEST_DONE__"))
        .build()
        .await;

    let mut chat = create_test_chat(&mock_base_url(&mock));
    let result = session::chat(&mut chat, "Echo test", None).await;

    assert!(result.is_ok());
    assert_eq!(mock.request_count(), 2);

    // The second request should contain the echo result in the tool message
    let msgs = mock.request_messages(1);
    let tool_result = msgs.iter()
        .find(|m| m["role"] == "tool")
        .expect("Should have tool result");

    let content = if let Some(s) = tool_result["content"].as_str() {
        s.to_string()
    } else if let Some(arr) = tool_result["content"].as_array() {
        arr.iter().filter_map(|p| p["text"].as_str()).collect::<Vec<_>>().join("")
    } else {
        String::new()
    };
    assert!(content.contains("echo: test123"), "Tool result should contain echo output, got: {}", content);
}

#[tokio::test]
async fn test_chat_task_mode_completion_marker() {
    let mock = MockLlmServer::builder()
        // First response: no marker → should nudge
        .next(Response::text("Working on it..."))
        // Second response: has marker → done
        .next(Response::text("All done! __TEST_DONE__"))
        .build()
        .await;

    let mut chat = create_test_chat(&mock_base_url(&mock));
    let result = session::chat(&mut chat, "Do something", None).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.contains("All done"), "Response: {}", response);
    assert!(!response.contains("__TEST_DONE__"), "Marker should be stripped");
}

#[tokio::test]
async fn test_chat_cancellation() {
    let mock = MockLlmServer::builder()
        .next(Response::text("Should not see this"))
        .build()
        .await;

    let mut chat = create_test_chat(&mock_base_url(&mock));
    let token = tokio_util::sync::CancellationToken::new();
    token.cancel();

    let result = session::chat(&mut chat, "Hello", Some(token)).await;
    assert!(result.is_err(), "Should error on cancellation");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("interrupted") || err.contains("cancelled"),
        "Error should mention interruption: {}", err);
}

#[tokio::test]
async fn test_chat_unknown_tool_returns_error() {
    let mock = MockLlmServer::builder()
        .next(Response::tool_call("nonexistent_tool", serde_json::json!({"x": 1})))
        .next(Response::text("Tool failed, moving on. __TEST_DONE__"))
        .build()
        .await;

    let mut chat = create_test_chat(&mock_base_url(&mock));
    let result = session::chat(&mut chat, "Use nonexistent tool", None).await;

    assert!(result.is_ok(), "chat() should handle tool errors gracefully: {:?}", result.err());
    assert_eq!(mock.request_count(), 2);
}

#[tokio::test]
async fn test_chat_message_history_accumulates() {
    let mock = MockLlmServer::builder()
        .next(Response::tool_call("echo", serde_json::json!({"message": "hi"})))
        .next(Response::text("Done echoing. __TEST_DONE__"))
        .build()
        .await;

    let mut chat = create_test_chat(&mock_base_url(&mock));
    let _ = session::chat(&mut chat, "Echo hi", None).await;

    let roles: Vec<&str> = chat.messages().iter().map(|m| m.role.as_str()).collect();
    assert!(roles.contains(&"system"), "Should have system message");
    assert!(roles.contains(&"user"), "Should have user message");
    assert!(roles.contains(&"tool"), "Should have tool result");

    let last = chat.messages().last().unwrap();
    assert_eq!(last.role, "assistant");
}

#[tokio::test]
async fn test_chat_token_usage_tracked() {
    let mock = MockLlmServer::builder()
        .next(Response::text("Hello! __TEST_DONE__").with_usage(50, 20))
        .build()
        .await;

    let mut chat = create_test_chat(&mock_base_url(&mock));
    let _ = session::chat(&mut chat, "Hi", None).await;

    assert!(chat.total_tokens_used() > 0, "Should track token usage, got {}", chat.total_tokens_used());
}
