use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};

pub mod anthropic;
pub mod groq;
pub mod llama_cpp;

/// Chat message structure (OpenAI-compatible format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
}

/// Tool call structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub function: FunctionCall,
}

/// Function call structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Tool call event in streaming
#[derive(Debug, Clone)]
pub enum ToolCallEvent {
    /// Start of a new tool call
    Start {
        index: usize,
        id: String,
        name: String,
    },
    /// Delta of tool call arguments (JSON fragment)
    Delta {
        index: usize,
        arguments_delta: String,
    },
}

/// Streaming chunk for LLM responses
#[derive(Debug, Clone)]
pub struct StreamingChunk {
    pub content: String,
    pub delta: String,
    pub finish_reason: Option<String>,
    pub tool_call_event: Option<ToolCallEvent>,
}

/// Temporary overrides for LLM request parameters.
/// Used by the self-regulate tool to let the LLM adjust its own parameters
/// for a specified number of upcoming calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequestOverrides {
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<u32>,
    pub remaining_calls: u32,
}

impl LlmRequestOverrides {
    /// Apply overrides to a JSON request body by inserting/overwriting the relevant fields.
    pub fn apply_to_request(&self, request: &mut serde_json::Value) {
        if let Some(temp) = self.temperature {
            request["temperature"] = serde_json::json!(temp);
        }
        if let Some(top_p) = self.top_p {
            request["top_p"] = serde_json::json!(top_p);
        }
        if let Some(max_tokens) = self.max_tokens {
            request["max_tokens"] = serde_json::json!(max_tokens);
        }
    }
}

/// LLM client trait - unified interface for all LLM providers
#[async_trait]
pub trait LlmClient: Send + Sync + std::fmt::Debug {
    /// Chat with tools support (non-streaming)
    async fn chat(&self, messages: Vec<ChatMessage>, tools: Vec<ToolDefinition>) -> Result<LlmResponse>;

    /// Simple chat completion without tools
    async fn chat_completion(&self, messages: &[ChatMessage]) -> Result<String>;

    /// Streaming chat completion - returns a stream of chunks
    async fn chat_streaming(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolDefinition>,
    ) -> Result<Box<dyn Stream<Item = Result<StreamingChunk>> + Send + Unpin>> {
        // Default implementation falls back to non-streaming
        Err(anyhow::anyhow!("Streaming not implemented for this client"))
    }

    /// Set temporary request parameter overrides (temperature, top_p, max_tokens).
    /// These are consumed on the next chat/chat_streaming call.
    fn set_request_overrides(&self, _overrides: Option<LlmRequestOverrides>) {
        // Default: no-op for clients that don't support overrides
    }
}

/// LLM response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub message: ChatMessage,
    pub usage: Option<TokenUsage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}
