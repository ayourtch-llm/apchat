// WebexOutputSink - Broadcasts AI responses to Webex DM

use anyhow::{Context, Result};
use std::sync::Arc;
use webex::Webex;

pub struct WebexOutputSink {
    webex_client: Arc<Webex>,
    room_id: String,
}

impl WebexOutputSink {
    /// Create a new WebexOutputSink
    ///
    /// Takes a shared Webex client and room ID to send messages to
    pub fn new(webex_client: Arc<Webex>, room_id: String) -> Self {
        Self {
            webex_client,
            room_id,
        }
    }

    /// Send a response message to Webex
    ///
    /// This broadcasts the AI's response to the Webex DM space
    pub async fn send_response(&self, text: &str) -> Result<()> {
        self.webex_client
            .send_message(&self.room_id, text)
            .await
            .context("Failed to send message to Webex")?;

        Ok(())
    }

    /// Send a response with optional markdown formatting
    ///
    /// Webex supports markdown formatting in messages
    pub async fn send_response_markdown(&self, text: &str, markdown: &str) -> Result<()> {
        self.webex_client
            .send_message_markdown(&self.room_id, text, markdown)
            .await
            .context("Failed to send markdown message to Webex")?;

        Ok(())
    }
}
