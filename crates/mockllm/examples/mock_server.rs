//! Standalone mock LLM server — llama.cpp / vLLM / OpenAI compatible.
//!
//! Run:
//!   cargo run -p mockllm --example mock_server
//!   cargo run -p mockllm --example mock_server -- --port 8080
//!
//! Then point any OpenAI-compatible client at:
//!   http://127.0.0.1:3456/v1/chat/completions
//!
//! The server returns a fixed greeting, then loops through scripted responses
//! demonstrating text, tool calls, and multi-tool patterns.

use mockllm::{MockLlmServer, Response};

fn build_demo_responses() -> Vec<Response> {
    vec![
        Response::text("Hello! I'm a mock LLM. I can simulate tool calls and multi-turn conversations."),
        Response::tool_call("read_file", serde_json::json!({"file_path": "src/main.rs"})),
        Response::text("I read the file. It contains a main function that initializes the application."),
        Response::tool_calls(vec![
            ("search_files", serde_json::json!({"pattern": "*.rs", "directory": "src/"})),
            ("list_files", serde_json::json!({"path": "src/"})),
        ]),
        Response::text("Found 15 Rust source files in the src/ directory."),
        Response::text_and_tool_call(
            "Let me check the configuration...",
            "read_file",
            serde_json::json!({"file_path": "config.toml"}),
        ),
        Response::text("Configuration looks good. The project is properly set up."),
    ]
}

#[tokio::main]
async fn main() {
    let port: u16 = std::env::args()
        .skip_while(|a| a != "--port")
        .nth(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or(3456);

    let responses = build_demo_responses();
    let n = responses.len();

    // Build with the demo responses
    let mut builder = MockLlmServer::builder().port(port);
    for r in responses {
        builder = builder.next(r);
    }
    let mock = builder.build().await;

    eprintln!("mockllm server running on http://127.0.0.1:{}/v1", mock.port());
    eprintln!("  {} scripted responses loaded (then returns fallback)", n);
    eprintln!("  Endpoints:");
    eprintln!("    POST /v1/chat/completions  (streaming & non-streaming)");
    eprintln!("    GET  /v1/models");
    eprintln!();
    eprintln!("  Example:");
    eprintln!("    curl -s http://127.0.0.1:{}/v1/chat/completions \\", mock.port());
    eprintln!("      -H 'Content-Type: application/json' \\");
    eprintln!("      -d '{{\"model\":\"mock\",\"messages\":[{{\"role\":\"user\",\"content\":\"hello\"}}]}}'");
    eprintln!();
    eprintln!("Press Ctrl-C to stop.");

    // Wait forever (Ctrl-C to stop)
    tokio::signal::ctrl_c().await.ok();
    eprintln!("\nShutting down. Served {} requests.", mock.request_count());
    mock.shutdown();
}
