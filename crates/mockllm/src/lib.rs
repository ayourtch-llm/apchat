//! # mockllm
//!
//! A deterministic mock LLM server for testing. Speaks the OpenAI-compatible
//! `/v1/chat/completions` API with scripted response sequences, tool calling,
//! and streaming (SSE).
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use mockllm::{MockLlmServer, Response};
//!
//! #[tokio::test]
//! async fn test_tool_calling() {
//!     let mock = MockLlmServer::builder()
//!         .next(Response::tool_call("read_file", serde_json::json!({"path": "a.rs"})))
//!         .next(Response::text("Done reading."))
//!         .build()
//!         .await;
//!
//!     let url = mock.url(); // http://127.0.0.1:<port>/v1
//!     // Point your LLM client at this URL...
//!
//!     assert_eq!(mock.request_count(), 0); // no calls yet
//!     // After calling: mock.request_count() == 2
//! }
//! ```

mod response;
mod server;

pub use response::{Response, ToolCallSpec};
pub use server::MockLlmServer;
