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
    initial_seen_ids: HashSet<String>,
}

impl WebexInputRouter {
    /// Create a new WebexInputRouter
    ///
    /// This will:
    /// 1. Create Webex client with access token
    /// 2. Send initial message to user (creates/finds direct room automatically)
    /// 3. Use the room ID from the message response for future polling
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

        // Send startup message directly to user's email
        // This automatically creates/uses the direct room
        let message = client
            .send_message_to_email(&user_email, "APchat ready")
            .await
            .context("Failed to send startup message")?;

        // Extract the room ID from the message response
        let room_id = message.room_id.clone();

        eprintln!("🔍 DEBUG: Fetching existing messages to mark as seen...");

        // Fetch existing messages and build the "seen" set
        // This prevents processing old messages from before bot startup
        // Note: This uses max=20, which should be enough for initial sync
        let initial_messages = client
            .get_messages(&room_id)
            .await
            .context("Failed to fetch initial messages")?;

        let seen_message_ids: HashSet<String> = initial_messages
            .into_iter()
            .map(|msg| msg.id)
            .collect();

        eprintln!("🔍 DEBUG: Marked {} existing messages as seen", seen_message_ids.len());

        Ok(Self {
            client: Arc::new(client),
            room_id,
            authorized_user_email: user_email,
            bot_email,
            mspc_channel,
            initial_seen_ids: seen_message_ids,
        })
    }

    /// Run the input router - polls Webex for new messages
    ///
    /// This will continuously poll for messages from the authorized user
    /// and route them to the MSPC channel
    pub async fn run(&self) -> Result<()> {
        println!("🌐 Webex bot starting - monitoring messages from {}", self.authorized_user_email);
        eprintln!("🔍 DEBUG: Room ID: {}", self.room_id);
        eprintln!("🔍 DEBUG: Bot email: {}", self.bot_email);

        // Start with all messages that existed at startup already marked as seen
        let mut seen_message_ids = self.initial_seen_ids.clone();
        eprintln!("🔍 DEBUG: Starting with {} messages already marked as seen", seen_message_ids.len());

        let mut poll_count = 0;

        loop {
            poll_count += 1;
            eprintln!("🔍 DEBUG: Poll #{} - Checking for messages...", poll_count);

            // Poll for messages (API returns newest first)
            match self.client.get_messages(&self.room_id).await {
                Ok(messages) => {
                    eprintln!("🔍 DEBUG: Poll #{} - Got {} messages", poll_count, messages.len());

                    for msg in messages {
                        eprintln!("🔍 DEBUG: Message ID: {}, from: {}, already seen: {}",
                            msg.id, msg.person_email, seen_message_ids.contains(&msg.id));

                        // If we've already processed this message, we've caught up
                        // Since messages are newest-first, we can stop here
                        if seen_message_ids.contains(&msg.id) {
                            eprintln!("🔍 DEBUG: Hit already-seen message, stopping iteration");
                            break;
                        }

                        eprintln!("🔍 DEBUG: Message from {} (authorized: {}, bot: {})",
                            msg.person_email, self.authorized_user_email, self.bot_email);

                        // Mark as seen immediately (before filtering) so we don't reprocess it
                        let msg_id = msg.id.clone();
                        seen_message_ids.insert(msg_id);

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
                                } else {
                                    eprintln!("🔍 DEBUG: Successfully sent message to MSPC channel");
                                }
                            }
                        } else {
                            eprintln!("🔍 DEBUG: Message filtered out (wrong sender or from bot)");
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
