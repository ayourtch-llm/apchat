//! JSON-RPC 2.0 protocol types

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

impl JsonRpcError {
    /// Create a parse error
    pub fn parse_error() -> Self {
        Self {
            code: -32700,
            message: "Parse error".to_string(),
        }
    }

    /// Create a invalid request error
    pub fn invalid_request(_id: Value, message: String) -> Self {
        Self {
            code: -32600,
            message,
        }
    }

    /// Create a method not found error
    pub fn method_not_found(_id: Value, method: String) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {}", method),
        }
    }

    /// Create a invalid params error
    pub fn invalid_params(_id: Value, message: String) -> Self {
        Self {
            code: -32602,
            message,
        }
    }

    /// Create an internal error
    pub fn internal_error(_id: Value, message: String) -> Self {
        Self {
            code: -32603,
            message,
        }
    }
}