// WebexOutputSink - Broadcasts AI responses to Webex DM

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::client::WebexClient;

pub struct WebexOutputSink {
    client: Arc<WebexClient>,
    room_id: String,
    last_user_message_id: Arc<Mutex<Option<String>>>,
    typing_started: Arc<Mutex<bool>>,
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
            typing_started: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the typing indicator
    pub async fn start_typing(&self) -> Result<()> {
        let mut typing_started = self.typing_started.lock().await;
        if *typing_started {
            return Ok(()); // Already showing typing indicator
        }
        
        self.client.send_typing_indicator(&self.room_id).await?;
        *typing_started = true;
        Ok(())
    }

    /// Stop the typing indicator (clear the typing message)
    pub async fn stop_typing(&self) -> Result<()> {
        let mut typing_started = self.typing_started.lock().await;
        if !*typing_started {
            return Ok(()); // Not showing typing indicator
        }
        
        // Send a blank message to clear the typing indicator
        self.client.send_message(&self.room_id, "").await?;
        *typing_started = false;
        Ok(())
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
