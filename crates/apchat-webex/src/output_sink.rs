// WebexOutputSink - Broadcasts AI responses to Webex DM

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::client::WebexClient;

pub struct WebexOutputSink {
    client: Arc<WebexClient>,
    room_id: String,
    last_user_message_id: Arc<Mutex<Option<String>>>,
}

impl WebexOutputSink {
    /// Create a new WebexOutputSink
    ///
    /// Takes a shared Webex client, room ID, and shared reference to last user message ID
    pub fn new(client: Arc<WebexClient>, room_id: String, last_user_message_id: Arc<Mutex<Option<String>>>) -> Self {
        Self {
            client,
            room_id,
            last_user_message_id,
        }
    }

    /// Send a response message to Webex
    ///
    /// This broadcasts the AI's response to the Webex DM space, threaded as a reply
    pub async fn send_response(&self, text: &str) -> Result<()> {
        // Get the last user message ID to thread the reply
        let parent_id = {
            let last_id = self.last_user_message_id.lock().await;
            last_id.clone()
        };

        self.client
            .send_message_with_parent(&self.room_id, text, parent_id)
            .await?;

        Ok(())
    }
}
