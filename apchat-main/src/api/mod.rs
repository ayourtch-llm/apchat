use std::sync::Arc;
use apchat_models::{ModelColor, Message, Tool};
use crate::config::ClientConfig;

/// Parameters needed for a stateless API call
#[derive(Clone)]
pub struct ApiCallParams {
    pub messages: Vec<Message>,
    pub current_model: ModelColor,
    pub client_config: ClientConfig,
    pub api_key: String,
    pub tools: Vec<Tool>,
    pub stream_responses: bool,
    pub verbose: bool,
    pub debug_level: u32,
    pub http_client: reqwest::Client,
}

impl ApiCallParams {
    /// Create from APChat (convenience method during migration)
    pub fn from_apchat(chat: &crate::APChat) -> Self {
        Self {
            messages: chat.messages.clone(),
            current_model: chat.current_model.clone(),
            client_config: chat.client_config.clone(),
            api_key: chat.api_key.clone(),
            tools: chat.get_tools(),
            stream_responses: chat.stream_responses,
            verbose: chat.verbose,
            debug_level: chat.debug_level,
            http_client: chat.client.clone(),
        }
    }

    pub fn should_show_debug(&self, level: u32) -> bool {
        self.debug_level & (1 << (level - 1)) != 0
    }
}

mod streaming;
mod client;

pub(crate) use streaming::{call_api_streaming, call_api_streaming_with_llm_client, StreamingMetrics};
pub(crate) use client::{call_api, call_api_with_llm_client, call_api_stateless, call_api_with_llm_client_stateless};
