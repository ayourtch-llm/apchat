//! MCP (Model Context Protocol) server implementation

pub mod handler;
pub mod protocol;
pub mod tools;

pub use handler::McpHandler;
pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
pub use tools::tool_definitions;