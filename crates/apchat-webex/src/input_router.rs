// WebexInputRouter - Monitors Webex DM for messages from authorized user
// and routes them to the MSPC channel

use anyhow::{Context, Result};
use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use apchat_mspc::{MspcChannel, MspcMessage};
use apchat_vty::print_heart_yellow;
use apchat_core::WebexDebugState;
use crate::client::WebexClient;

pub struct WebexInputRouter {
    client: Arc<WebexClient>,
    room_id: String,
    authorized_user_email: String,
    bot_email: String,
    mspc_channel: Arc<MspcChannel>,
    initial_seen_ids: HashSet<String>,
    last_user_message_id: Arc<Mutex<Option<String>>>,
    debug_state: Option<WebexDebugState>,
}

impl WebexInputRouter {
    /// Helper method to print debug message if debug is enabled
    fn debug_log(&self, message: &str) {
        if let Some(ref state) = self.debug_state {
            if state.is_enabled() {
                print_heart_yellow(message, true);
            }
        }
    }

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

        // Send startup message to create/find the direct room
        // This automatically creates/uses the direct room and marks it as seen
        let startup_message = format!("🤖 APChat ready (PID: {})", std::process::id());
        let message = client
            .send_message_to_email(&user_email, &startup_message)
            .await
            .context("Failed to send startup message")?;

        // Extract the room ID from the message response
        let room_id = message.room_id.clone();

        // Local helper for debug logging (no self available in static method)
        let debug_log = |msg: &str| {
            // Debug logging disabled by default in new()
        };

        debug_log("🔍 Webex room initialized successfully");
        debug_log("🔍 Fetching existing messages to mark as seen...");

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

        debug_log(&format!("🔍 Marked {} existing messages as seen", seen_message_ids.len()));

        Ok(Self {
            client: Arc::new(client),
            room_id,
            authorized_user_email: user_email,
            bot_email,
            mspc_channel,
            initial_seen_ids: seen_message_ids,
            last_user_message_id: Arc::new(Mutex::new(None)),
            debug_state: None,
        })
    }

    /// Get a shared reference to the last user message ID (for threading replies)
    pub fn last_user_message_id(&self) -> Arc<Mutex<Option<String>>> {
        Arc::clone(&self.last_user_message_id)
    }

    /// Run the input router - polls Webex for new messages
    ///
    /// This will continuously poll for messages from the authorized user
    /// and route them to the MSPC channel
    pub async fn run(&self) -> Result<()> {
        self.debug_log(&format!("🌐 Webex bot starting - monitoring messages from {}", self.authorized_user_email));
        self.debug_log(&format!("🔍 Room ID: {}", self.room_id));
        self.debug_log(&format!("🔍 Bot email: {}", self.bot_email));

        // Start with all messages that existed at startup already marked as seen
        let mut seen_message_ids = self.initial_seen_ids.clone();
        self.debug_log(&format!("🔍 Starting with {} messages already marked as seen", seen_message_ids.len()));

        let mut poll_count = 0;

        loop {
            poll_count += 1;
            self.debug_log(&format!("🔍 Poll #{} - Checking for messages...", poll_count));

            // Poll for messages (API returns newest first)
            match self.client.get_messages(&self.room_id).await {
                Ok(messages) => {
                    self.debug_log(&format!("🔍 Poll #{} - Got {} messages", poll_count, messages.len()));

                    for msg in messages {
                        let person_email_str = msg.person_email.as_deref().unwrap_or("unknown");
                        self.debug_log(&format!("🔍 Message ID: {}, from: {}, already seen: {}",
                            msg.id, person_email_str, seen_message_ids.contains(&msg.id)));

                        // If we've already processed this message, we've caught up
                        // Since messages are newest-first, we can stop here
                        if seen_message_ids.contains(&msg.id) {
                            self.debug_log("🔍 Hit already-seen message, stopping iteration");
                            break;
                        }

                        let authorized_str = self.authorized_user_email.as_str();
                        let bot_str = self.bot_email.as_str();
                        self.debug_log(&format!("🔍 Message from {} (authorized: {}, bot: {})",
                            person_email_str, authorized_str, bot_str));

                        // Mark as seen immediately (before filtering) so we don't reprocess it
                        let msg_id = msg.id.clone();
                        seen_message_ids.insert(msg_id);

                        // Filter: only messages from authorized user, not from bot
                        // NOTE: This logic must stay in sync with websocket_router.rs (see line ~225 for similar implementation)
                        if msg.person_email == Some(self.authorized_user_email.clone())
                            && msg.person_email != Some(self.bot_email.clone())
                        {
                            if let Some(text) = msg.text {
                                let person_email_str = msg.person_email.as_deref().unwrap_or("unknown");
                                self.debug_log(&format!("📨 Received Webex message from {}: {}", person_email_str, text));

                                // Track this message ID for threading replies
                                {
                                    let mut last_id = self.last_user_message_id.lock().await;
                                    *last_id = Some(msg.id.clone());
                                }

                                // Prefix the message with sender email and tool reference for clarity
                                let prefixed_text = format!("webex message from {}: [Use send_webex_message tool to respond] {}", person_email_str, text);

                                // Convert to MSPC message
                                let message = MspcMessage::UserInput(
                                    prefixed_text,
                                    Some(format!("webex:{}", person_email_str)),
                                );

                                // Send to MSPC channel
                                if let Err(e) = self.mspc_channel.send(message).await {
                                    print_heart_yellow(&format!("⚠️ Failed to send Webex message to MSPC channel: {}", e), true);
                                } else {
                                    self.debug_log("🔍 Successfully sent message to MSPC channel");
                                }
                            }
                        } else {
                            self.debug_log("🔍 Message filtered out (wrong sender or from bot)");
                        }
                    }
                }
                Err(e) => {
                    print_heart_yellow(&format!("⚠️ Failed to poll Webex messages: {}", e), true);
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
