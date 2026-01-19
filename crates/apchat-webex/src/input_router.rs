// WebexInputRouter - Monitors Webex DM for messages from authorized user
// and routes them to the MSPC channel

use anyhow::{Context, Result};
use std::sync::Arc;
use std::collections::HashSet;
use tokio::time::{sleep, Duration};
use apchat_mspc::{MspcChannel, MspcMessage};
use crate::client::WebexClient;

pub struct WebexInputRouter {
    client: Arc<WebexClient>,
    room_id: String,
    authorized_user_email: String,
    bot_email: String,
    mspc_channel: Arc<MspcChannel>,
}

impl WebexInputRouter {
    /// Create a new WebexInputRouter
    ///
    /// This will:
    /// 1. Create Webex client with access token
    /// 2. Look up the person from the user email
    /// 3. Get or create a direct message room with that person
    /// 4. Send "APchat ready" startup message
    pub async fn new(
        access_token: String,
        user_email: String,
        mspc_channel: Arc<MspcChannel>,
    ) -> Result<Self> {
        // Create Webex client
        let client = WebexClient::new(access_token);

        // Get the bot's own email to filter out our own messages
        let me = client
            .get_me()
            .await
            .context("Failed to get bot details")?;

        let bot_email = me.emails
            .first()
            .ok_or_else(|| anyhow::anyhow!("Bot has no email addresses"))?
            .clone();

        // Get person from email
        let person = client
            .get_person_by_email(&user_email)
            .await
            .context("Failed to get person by email")?;

        // Create direct room with this person
        let room = client
            .create_direct_room(&person.id)
            .await
            .context("Failed to create direct room")?;

        let room_id = room.id.clone();

        // Send startup message
        client
            .send_message(&room_id, "APchat ready")
            .await
            .context("Failed to send startup message")?;

        Ok(Self {
            client: Arc::new(client),
            room_id,
            authorized_user_email: user_email,
            bot_email,
            mspc_channel,
        })
    }

    /// Run the input router - polls Webex for new messages
    ///
    /// This will continuously poll for messages from the authorized user
    /// and route them to the MSPC channel
    pub async fn run(&self) -> Result<()> {
        println!("🌐 Webex bot starting - monitoring messages from {}", self.authorized_user_email);

        let mut seen_message_ids: HashSet<String> = HashSet::new();

        loop {
            // Poll for messages
            match self.client.get_messages(&self.room_id).await {
                Ok(messages) => {
                    for msg in messages {
                        // Skip if we've already processed this message
                        if seen_message_ids.contains(&msg.id) {
                            continue;
                        }

                        // Filter: only messages from authorized user, not from bot
                        if msg.person_email == self.authorized_user_email
                            && msg.person_email != self.bot_email
                        {
                            if let Some(text) = msg.text {
                                println!(
                                    "📨 Received Webex message from {}: {}",
                                    msg.person_email, text
                                );

                                // Convert to MSPC message
                                let message = MspcMessage::UserInput(
                                    text,
                                    Some(format!("webex:{}", msg.person_email)),
                                );

                                // Send to MSPC channel
                                if let Err(e) = self.mspc_channel.send(message).await {
                                    eprintln!("⚠️ Failed to send Webex message to MSPC channel: {}", e);
                                }

                                // Mark as seen
                                seen_message_ids.insert(msg.id);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("⚠️ Failed to poll Webex messages: {}", e);
                }
            }

            // Poll every 2 seconds
            sleep(Duration::from_secs(2)).await;
        }
    }

    /// Get the room ID for this router
    pub fn room_id(&self) -> &str {
        &self.room_id
    }

    /// Get the Webex client (for sharing with output sink)
    pub fn client(&self) -> Arc<WebexClient> {
        Arc::clone(&self.client)
    }
}
