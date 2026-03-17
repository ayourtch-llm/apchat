//! Scripted response definitions for the mock LLM server.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A tool call to include in an assistant response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallSpec {
    pub name: String,
    pub arguments: Value,
}

/// A scripted response the mock server will return.
///
/// Each call to `/v1/chat/completions` consumes the next response in the queue.
/// If the queue is exhausted, the server returns a default "no more responses" message.
#[derive(Debug, Clone)]
pub struct Response {
    /// Text content of the assistant message (can be empty if only tool calls).
    pub text: String,
    /// Optional tool calls to include in the response.
    pub tool_calls: Vec<ToolCallSpec>,
    /// Finish reason: "stop" for text-only, "tool_calls" when tool calls present.
    pub finish_reason: String,
    /// Optional usage to report.
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

impl Response {
    /// Create a text-only response.
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            text: content.into(),
            tool_calls: vec![],
            finish_reason: "stop".to_string(),
            prompt_tokens: 10,
            completion_tokens: 5,
        }
    }

    /// Create a response with a single tool call.
    pub fn tool_call(name: impl Into<String>, arguments: Value) -> Self {
        Self {
            text: String::new(),
            tool_calls: vec![ToolCallSpec {
                name: name.into(),
                arguments,
            }],
            finish_reason: "tool_calls".to_string(),
            prompt_tokens: 10,
            completion_tokens: 5,
        }
    }

    /// Create a response with text and a tool call.
    pub fn text_and_tool_call(
        content: impl Into<String>,
        name: impl Into<String>,
        arguments: Value,
    ) -> Self {
        Self {
            text: content.into(),
            tool_calls: vec![ToolCallSpec {
                name: name.into(),
                arguments,
            }],
            finish_reason: "tool_calls".to_string(),
            prompt_tokens: 10,
            completion_tokens: 5,
        }
    }

    /// Create a response with multiple tool calls.
    pub fn tool_calls(calls: Vec<(impl Into<String>, Value)>) -> Self {
        Self {
            text: String::new(),
            tool_calls: calls
                .into_iter()
                .map(|(name, args)| ToolCallSpec {
                    name: name.into(),
                    arguments: args,
                })
                .collect(),
            finish_reason: "tool_calls".to_string(),
            prompt_tokens: 10,
            completion_tokens: 5,
        }
    }

    /// Set custom token usage.
    pub fn with_usage(mut self, prompt: u32, completion: u32) -> Self {
        self.prompt_tokens = prompt;
        self.completion_tokens = completion;
        self
    }

    /// Build the OpenAI-compatible JSON response body.
    pub(crate) fn to_json(&self, call_index: usize) -> Value {
        let mut message = serde_json::json!({
            "role": "assistant",
            "content": self.text,
        });

        if !self.tool_calls.is_empty() {
            let tc: Vec<Value> = self
                .tool_calls
                .iter()
                .enumerate()
                .map(|(i, tc)| {
                    serde_json::json!({
                        "id": format!("call_mock_{}_{}", call_index, i),
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": tc.arguments.to_string(),
                        }
                    })
                })
                .collect();
            message["tool_calls"] = Value::Array(tc);
        }

        let total = self.prompt_tokens + self.completion_tokens;
        serde_json::json!({
            "id": format!("chatcmpl-mock-{}", call_index),
            "object": "chat.completion",
            "created": 1700000000 + call_index as u64,
            "model": "mock-model",
            "choices": [{
                "index": 0,
                "message": message,
                "finish_reason": self.finish_reason,
            }],
            "usage": {
                "prompt_tokens": self.prompt_tokens,
                "completion_tokens": self.completion_tokens,
                "total_tokens": total,
            }
        })
    }

    /// Build SSE streaming chunks for this response.
    pub(crate) fn to_sse_chunks(&self, call_index: usize) -> Vec<String> {
        let mut chunks = Vec::new();

        // First chunk: role
        chunks.push(format!(
            "data: {}\n\n",
            serde_json::json!({
                "id": format!("chatcmpl-mock-{}", call_index),
                "object": "chat.completion.chunk",
                "model": "mock-model",
                "choices": [{
                    "index": 0,
                    "delta": { "role": "assistant" },
                    "finish_reason": null,
                }]
            })
        ));

        // Text content chunks (split into ~20 char pieces)
        if !self.text.is_empty() {
            for chunk_text in self.text.as_bytes().chunks(20) {
                let text = String::from_utf8_lossy(chunk_text);
                chunks.push(format!(
                    "data: {}\n\n",
                    serde_json::json!({
                        "id": format!("chatcmpl-mock-{}", call_index),
                        "object": "chat.completion.chunk",
                        "model": "mock-model",
                        "choices": [{
                            "index": 0,
                            "delta": { "content": text },
                            "finish_reason": null,
                        }]
                    })
                ));
            }
        }

        // Tool call chunks
        for (i, tc) in self.tool_calls.iter().enumerate() {
            // Tool call start
            chunks.push(format!(
                "data: {}\n\n",
                serde_json::json!({
                    "id": format!("chatcmpl-mock-{}", call_index),
                    "object": "chat.completion.chunk",
                    "model": "mock-model",
                    "choices": [{
                        "index": 0,
                        "delta": {
                            "tool_calls": [{
                                "index": i,
                                "id": format!("call_mock_{}_{}", call_index, i),
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": "",
                                }
                            }]
                        },
                        "finish_reason": null,
                    }]
                })
            ));

            // Tool call arguments (in one chunk for simplicity)
            chunks.push(format!(
                "data: {}\n\n",
                serde_json::json!({
                    "id": format!("chatcmpl-mock-{}", call_index),
                    "object": "chat.completion.chunk",
                    "model": "mock-model",
                    "choices": [{
                        "index": 0,
                        "delta": {
                            "tool_calls": [{
                                "index": i,
                                "function": {
                                    "arguments": tc.arguments.to_string(),
                                }
                            }]
                        },
                        "finish_reason": null,
                    }]
                })
            ));
        }

        // Final chunk with finish_reason and usage
        let total = self.prompt_tokens + self.completion_tokens;
        chunks.push(format!(
            "data: {}\n\n",
            serde_json::json!({
                "id": format!("chatcmpl-mock-{}", call_index),
                "object": "chat.completion.chunk",
                "model": "mock-model",
                "choices": [{
                    "index": 0,
                    "delta": {},
                    "finish_reason": self.finish_reason,
                }],
                "usage": {
                    "prompt_tokens": self.prompt_tokens,
                    "completion_tokens": self.completion_tokens,
                    "total_tokens": total,
                }
            })
        ));

        // Done marker
        chunks.push("data: [DONE]\n\n".to_string());

        chunks
    }
}
