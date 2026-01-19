// WebexOutputSink - Broadcasts AI responses to Webex DM

use anyhow::Result;
use std::sync::Arc;
use crate::client::WebexClient;

pub struct WebexOutputSink {
    client: Arc<WebexClient>,
    room_id: String,
}

impl WebexOutputSink {
    /// Create a new WebexOutputSink
    ///
    /// Takes a shared Webex client and room ID to send messages to
    pub fn new(client: Arc<WebexClient>, room_id: String) -> Self {
        Self {
            client,
            room_id,
        }
    }

    /// Send a response message to Webex
    ///
    /// This broadcasts the AI's response to the Webex DM space
    pub async fn send_response(&self, text: &str) -> Result<()> {
        self.client
            .send_message(&self.room_id, text)
            .await?;

        Ok(())
    }
}
