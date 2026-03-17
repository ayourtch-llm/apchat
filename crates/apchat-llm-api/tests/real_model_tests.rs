//! Integration tests that call a real LLM model.
//!
//! These tests are **skipped by default**. To run them, set:
//!
//!   APCHAT_TEST_MODEL=modelname@backend(url)
//!
//! Examples:
//!   APCHAT_TEST_MODEL=qwen3.5-long@llama(http://127.0.0.1:4000/v1/)
//!   APCHAT_TEST_MODEL=llama3.1@llama(http://localhost:11434/v1/)
//!   APCHAT_TEST_MODEL=claude-3-5-sonnet-20241022@anthropic
//!
//! The syntax is the same as the --model CLI flag.

use std::sync::Arc;
use apchat_llm_api::{
    ClientFactory, ChatMessage, LlmClient, ToolDefinition,
    parse_model_attings,
};
use apchat_models::types::ContentPart;

/// Parse APCHAT_TEST_MODEL and create a real LLM client.
/// Returns None if the env var is not set (test should be skipped).
fn get_test_client() -> Option<Arc<dyn LlmClient>> {
    let spec = std::env::var("APCHAT_TEST_MODEL").ok()?;
    let (model, backend, url) = parse_model_attings(&spec);
    let backend = backend.expect("APCHAT_TEST_MODEL must include @backend, e.g. model@llama(url)");
    Some(ClientFactory::create(backend, None, model, url, Some("test".to_string())))
}

/// Skip macro — returns early if no test model is configured.
macro_rules! require_test_model {
    () => {
        match get_test_client() {
            Some(client) => client,
            None => {
                eprintln!("APCHAT_TEST_MODEL not set, skipping real model test");
                return;
            }
        }
    };
}

/// Extract text content from a ChatMessage
fn response_text(msg: &ChatMessage) -> String {
    msg.content
        .iter()
        .filter_map(|p| match p {
            ContentPart::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

fn user_msg(text: &str) -> ChatMessage {
    ChatMessage {
        role: "user".to_string(),
        content: vec![ContentPart::Text(text.to_string())],
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning: None,
    }
}

fn system_msg(text: &str) -> ChatMessage {
    ChatMessage {
        role: "system".to_string(),
        content: vec![ContentPart::Text(text.to_string())],
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning: None,
    }
}

// ─── Basic connectivity ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_real_model_simple_completion() {
    let client = require_test_model!();

    let messages = vec![
        system_msg("You are a helpful assistant. Reply in one short sentence."),
        user_msg("What is 2 + 2?"),
    ];

    let result = client.chat_completion(&messages).await;
    assert!(result.is_ok(), "chat_completion failed: {:?}", result.err());

    let response = result.unwrap();
    assert!(!response.is_empty(), "Response should not be empty");
    eprintln!("Response: {}", response);

    // The response should mention "4" somewhere
    assert!(
        response.contains('4'),
        "Expected response to contain '4', got: {}",
        response
    );
}

// ─── Chat with tool definitions ──────────────────────────────────────────────

#[tokio::test]
async fn test_real_model_chat_with_tools() {
    let client = require_test_model!();

    let messages = vec![
        system_msg("You are a helpful assistant with access to tools. Use the get_weather tool to answer weather questions."),
        user_msg("What's the weather in Paris?"),
    ];

    let tools = vec![ToolDefinition {
        name: "get_weather".to_string(),
        description: "Get the current weather for a city".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "city": {
                    "type": "string",
                    "description": "The city name"
                }
            },
            "required": ["city"]
        }),
    }];

    let result = client.chat(messages, tools).await;
    assert!(result.is_ok(), "chat with tools failed: {:?}", result.err());

    let response = result.unwrap();
    let text = response_text(&response.message);
    let tool_calls = &response.message.tool_calls;
    eprintln!("Response text: {}", text);
    eprintln!("Tool calls: {:?}", tool_calls);

    // The model should either call the tool or mention weather
    let has_tool_call = tool_calls.as_ref().map_or(false, |tc| !tc.is_empty());
    let mentions_weather = text.to_lowercase().contains("weather")
        || text.to_lowercase().contains("paris");

    assert!(
        has_tool_call || mentions_weather,
        "Expected tool call or weather mention, got text: '{}', tool_calls: {:?}",
        text,
        tool_calls
    );

    // If tool was called, verify the arguments
    if has_tool_call {
        let tool_calls = tool_calls.as_ref().unwrap();
        let call = &tool_calls[0];
        assert_eq!(call.function.name, "get_weather");

        let args: serde_json::Value = serde_json::from_str(&call.function.arguments)
            .expect("Tool call arguments should be valid JSON");
        let city = args.get("city").and_then(|v| v.as_str()).unwrap_or("");
        eprintln!("Tool called with city: {}", city);
        assert!(
            city.to_lowercase().contains("paris"),
            "Expected city to contain 'paris', got: {}",
            city
        );
    }
}

// ─── Multi-turn conversation ─────────────────────────────────────────────────

#[tokio::test]
async fn test_real_model_multi_turn() {
    let client = require_test_model!();

    let messages = vec![
        system_msg("You are a helpful assistant. Be very brief."),
        user_msg("My name is Alice."),
    ];

    let response1 = client.chat_completion(&messages).await
        .expect("First turn failed");
    eprintln!("Turn 1: {}", response1);

    // Second turn — ask the model to recall the name
    let messages2 = vec![
        system_msg("You are a helpful assistant. Be very brief."),
        user_msg("My name is Alice."),
        ChatMessage {
            role: "assistant".to_string(),
            content: vec![ContentPart::Text(response1)],
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning: None,
        },
        user_msg("What is my name? Reply with just the name."),
    ];

    let response2 = client.chat_completion(&messages2).await
        .expect("Second turn failed");
    eprintln!("Turn 2: {}", response2);

    assert!(
        response2.to_lowercase().contains("alice"),
        "Model should remember the name 'Alice', got: {}",
        response2
    );
}

// ─── Streaming ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_real_model_streaming() {
    let client = require_test_model!();

    let messages = vec![
        system_msg("Reply with exactly: Hello World"),
        user_msg("Say hello"),
    ];

    use futures::StreamExt;
    let stream_result = client
        .chat_streaming(messages, vec![])
        .await;

    // Streaming might not be supported by all backends
    match stream_result {
        Ok(mut stream) => {
            let mut full_text = String::new();
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(c) => {
                        if !c.delta.is_empty() {
                            full_text.push_str(&c.delta);
                        }
                    }
                    Err(e) => {
                        eprintln!("Stream error: {}", e);
                        break;
                    }
                }
            }
            eprintln!("Streamed response: {}", full_text);
            assert!(!full_text.is_empty(), "Streaming should produce some text");
        }
        Err(e) => {
            eprintln!("Streaming not supported or failed: {}. Skipping.", e);
        }
    }
}

// ─── Error handling ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_real_model_empty_messages() {
    let client = require_test_model!();

    let result = client.chat_completion(&[]).await;
    // Most models will return an error for empty messages
    // We just verify it doesn't panic
    eprintln!("Empty messages result: {:?}", result.is_ok());
}

// ─── Token usage ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_real_model_returns_usage() {
    let client = require_test_model!();

    let messages = vec![
        user_msg("Say hi"),
    ];

    let result = client.chat(messages, vec![]).await;
    assert!(result.is_ok(), "chat failed: {:?}", result.err());

    let resp = result.unwrap();
    if let Some(usage) = &resp.usage {
        eprintln!(
            "Token usage: prompt={}, completion={}, total={}",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
        assert!(usage.prompt_tokens > 0, "Should have prompt tokens");
        assert!(usage.completion_tokens > 0, "Should have completion tokens");
    } else {
        eprintln!("Note: model did not return token usage info");
    }
}
