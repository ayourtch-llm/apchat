// WebexInputRouter - Monitors Webex DM for messages from authorized user
// and routes them to the MSPC channel

use anyhow::{Context, Result};
use std::sync::Arc;
use webex::Webex;
use apchat::mspc::{MspcChannel, MspcMessage};

pub struct WebexInputRouter {
    webex_client: Arc<Webex>,
    room_id: String,
    authorized_user_id: String,
    mspc_channel: Arc<MspcChannel>,
}

impl WebexInputRouter {
    /// Create a new WebexInputRouter
    ///
    /// This will:
    /// 1. Create Webex client with access token
    /// 2. Look up the person ID from the user email
    /// 3. Get or create a direct message room with that person
    /// 4. Send "APchat ready" startup message
    pub async fn new(
        access_token: String,
        user_email: String,
        mspc_channel: Arc<MspcChannel>,
    ) -> Result<Self> {
        // Create Webex client
        let webex_client = Webex::new(&access_token)
            .context("Failed to create Webex client")?;

        // Get person ID from email
        let person = webex_client
            .get_person_by_email(&user_email)
            .await
            .with_context(|| format!("Failed to get person ID for email: {}", user_email))?;

        let person_id = person.id.clone();

        // Get or create direct room with this person
        let room_id = Self::get_or_create_direct_room(&webex_client, &person_id)
            .await
            .context("Failed to get or create direct room")?;

        // Send startup message
        webex_client
            .send_message(&room_id, "APchat ready")
            .await
            .context("Failed to send startup message")?;

        Ok(Self {
            webex_client: Arc::new(webex_client),
            room_id,
            authorized_user_id: person_id,
            mspc_channel,
        })
    }

    /// Get existing direct room or create a new one
    async fn get_or_create_direct_room(
        webex_client: &Webex,
        person_id: &str,
    ) -> Result<String> {
        // List rooms filtered by type=direct
        let rooms = webex_client
            .list_rooms()
            .await
            .context("Failed to list rooms")?;

        // Find existing direct room with this person
        for room in rooms {
            if room.room_type == Some("direct".to_string()) {
                // Check if this room is with the target person
                // Note: Webex API doesn't directly expose who the direct room is with,
                // so we'll try to find a room that contains our person
                // In practice, we might need to check memberships
                // For now, assume we need to create a new room if not found
            }
        }

        // Create new direct room
        let room = webex_client
            .create_room(&format!("Direct with {}", person_id), Some("direct"))
            .await
            .context("Failed to create direct room")?;

        Ok(room.id)
    }

    /// Run the input router - monitors Webex event stream for messages
    ///
    /// This will continuously listen for messages from the authorized user
    /// and route them to the MSPC channel
    pub async fn run(&self) -> Result<()> {
        println!("🌐 Webex bot starting - monitoring messages from authorized user");

        // Start event stream
        let mut event_stream = self
            .webex_client
            .event_stream()
            .await
            .context("Failed to start Webex event stream")?;

        // Process events
        while let Some(event) = event_stream.next().await {
            match event {
                Ok(webex::types::Event::Message(msg)) => {
                    // Filter: only messages from authorized user in our room
                    if msg.person_id == self.authorized_user_id
                        && msg.room_id == self.room_id
                    {
                        // Don't process messages from the bot itself
                        if msg.person_email == Some(self.webex_client.get_bot_email()) {
                            continue;
                        }

                        println!(
                            "📨 Received Webex message from {}: {}",
                            self.authorized_user_id,
                            msg.text.as_deref().unwrap_or("<no text>")
                        );

                        // Convert to MSPC message
                        let content = msg.text.unwrap_or_default();
                        let message = MspcMessage::UserInput(
                            content,
                            Some(format!("webex:{}", self.authorized_user_id)),
                        );

                        // Send to MSPC channel
                        if let Err(e) = self.mspc_channel.send(message).await {
                            eprintln!("⚠️ Failed to send Webex message to MSPC channel: {}", e);
                        }
                    }
                }
                Ok(_other_event) => {
                    // Ignore other event types for now
                }
                Err(e) => {
                    eprintln!("⚠️ Webex event stream error: {}", e);
                    // Continue processing - don't crash on transient errors
                }
            }
        }

        println!("🌐 Webex event stream ended");
        Ok(())
    }

    /// Get the room ID for this router
    pub fn room_id(&self) -> &str {
        &self.room_id
    }

    /// Get the Webex client (for sharing with output sink)
    pub fn webex_client(&self) -> Arc<Webex> {
        Arc::clone(&self.webex_client)
    }
}
