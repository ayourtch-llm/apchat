// WebexWebSocketRouter - Real-time message delivery via Mercury WebSocket
use anyhow::{Context, Result};
use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use apchat_mspc::{MspcChannel, MspcMessage};
use apchat_vty::print_heart_yellow;
use crate::client::WebexClient;
use crate::types::{MercuryEvent};

pub struct WebexWebSocketRouter {
    client: Arc<WebexClient>,
    access_token: String,
    authorized_user_email: String,
    bot_email: String,
    mspc_channel: Arc<MspcChannel>,
    device_id: String,
    websocket_url: String,
    room_id: String,
    seen_message_ids: Arc<Mutex<HashSet<String>>>,
    last_user_message_id: Arc<Mutex<Option<String>>>,
}

impl WebexWebSocketRouter {
    /// Create a new WebexWebSocketRouter
    pub async fn new(
        access_token: String,
        user_email: String,
        mspc_channel: Arc<MspcChannel>,
    ) -> Result<Self> {
        let token = access_token.clone();
        let client = WebexClient::new(access_token);

        // Get bot's own email
        let me = client
            .get_me()
            .await
            .context("Failed to get bot details")?;

        let bot_email = me.emails
            .first()
            .ok_or_else(|| anyhow::anyhow!("Bot has no email addresses"))?
            .clone();

        // Register device to get WebSocket URL
        let registration = client
            .register_device()
            .await
            .context("Failed to register device")?;

        // Send startup message and get initial messages
        let message = client
            .send_message_to_email(&user_email, "APchat ready (WebSocket mode)")
            .await
            .context("Failed to send startup message")?;

        let room_id = message.room_id;

        // Fetch existing messages to mark as seen
        let initial_messages = client
            .get_messages(&room_id)
            .await
            .context("Failed to fetch initial messages")?;

        let seen_ids: HashSet<String> = initial_messages
            .into_iter()
            .map(|msg| msg.id)
            .collect();

        print_heart_yellow(&format!("🔍 WebSocket router initialized with {} seen messages", seen_ids.len()), true);

        Ok(Self {
            client: Arc::new(client),
            access_token: token,
            authorized_user_email: user_email.clone(),
            bot_email,
            mspc_channel,
            device_id: format!("webex-{}", &user_email),
            websocket_url: registration.web_socket_url,
            room_id,
            seen_message_ids: Arc::new(Mutex::new(seen_ids)),
            last_user_message_id: Arc::new(Mutex::new(None)),
        })
    }

    /// Get a shared reference to the last user message ID (for threading replies)
    pub fn last_user_message_id(&self) -> Arc<Mutex<Option<String>>> {
        Arc::clone(&self.last_user_message_id)
    }

    /// Process a Mercury event and route to MSPC if it's a user message
    async fn process_event(&self, event: MercuryEvent) -> Result<()> {
        // Only process conversation.activity events
        if event.data.event_type != "conversation.activity" {
            return Ok(());
        }

        let activity = match event.data.activity {
            Some(act) => act,
            None => return Ok(()),
        };

        // Only process "post" verbs (new messages)
        if activity.verb != "post" {
            return Ok(());
        }

        // Check who posted the message
        let actor_email = activity.actor
            .as_ref()
            .map(|a| a.id.as_str())
            .unwrap_or("");

        print_heart_yellow(&format!("🔍 Mercury notification - new activity from {}", actor_email), true);

        // Skip Mercury notifications for bot's own messages
        // This prevents unnecessary API calls and avoids triggering "new messages" indicator
        if actor_email == self.authorized_user_email {
            print_heart_yellow("🔍 Actor is authorized user, processing...", true);
        } else if actor_email == self.bot_email {
            print_heart_yellow("🔍 Skipping Mercury notification for bot's own message", true);
            return Ok(());
        } else {
            print_heart_yellow(&format!("🔍 Skipping Mercury notification from {}", actor_email), true);
            return Ok(());
        }

        // Mercury sends encrypted content - we need to fetch the actual messages via REST API
        // Note: Mercury activity.id != REST API message.id (different ID formats)
        // So we just fetch recent messages and process any unseen ones from authorized user
        print_heart_yellow("🔍 Fetching recent messages from REST API...", true);
        let messages = self.client
            .get_messages(&self.room_id)
            .await
            .context("Failed to fetch messages after Mercury notification")?;

        // Process all unseen messages from authorized user
        let mut processed_count = 0;
        for msg in messages {
            // Check if already seen
            let is_new = {
                let mut seen = self.seen_message_ids.lock().await;
                if seen.contains(&msg.id) {
                    false
                } else {
                    seen.insert(msg.id.clone());
                    true
                }
            };

            if !is_new {
                continue;
            }

            print_heart_yellow(&format!("🔍 New message [{}] from {} @ {}: {}",
                msg.id.chars().take(8).collect::<String>(),
                msg.person_email,
                msg.created,
                msg.text.as_ref().map(|t| t.chars().take(50).collect::<String>()).unwrap_or_else(|| "[no text]".to_string())
            ), true);

            // Filter: only messages from authorized user, not from bot
            if msg.person_email == self.authorized_user_email && msg.person_email != self.bot_email {
                if let Some(text) = msg.text {
                    print_heart_yellow(&format!("📨 Received Webex message from {}: {}", msg.person_email, text), true);

                    // Track this message ID for threading replies
                    {
                        let mut last_id = self.last_user_message_id.lock().await;
                        *last_id = Some(msg.id.clone());
                    }

                    // Send to MSPC channel
                    let message = MspcMessage::UserInput(
                        text,
                        Some(format!("webex:{}", msg.person_email)),
                    );

                    if let Err(e) = self.mspc_channel.send(message).await {
                        print_heart_yellow(&format!("⚠️ Failed to send message to MSPC channel: {}", e), true);
                    }

                    processed_count += 1;
                }
            } else if msg.person_email == self.bot_email {
                print_heart_yellow(&format!("🔍 Skipping bot's own message: {}",
                    msg.text.as_ref().map(|t| t.chars().take(50).collect::<String>()).unwrap_or_else(|| "[no text]".to_string())), true);
            } else {
                print_heart_yellow(&format!("🔍 Skipping message from {}: {}",
                    msg.person_email,
                    msg.text.as_ref().map(|t| t.chars().take(50).collect::<String>()).unwrap_or_else(|| "[no text]".to_string())), true);
            }
        }

        print_heart_yellow(&format!("🔍 Processed {} new messages from authorized user", processed_count), true);

        Ok(())
    }

    /// Poll for missed messages after reconnection
    async fn recover_message_gap(&self) -> Result<()> {
        print_heart_yellow("🔍 Checking for missed messages during disconnect...", true);

        let messages = self.client
            .get_messages(&self.room_id)
            .await
            .context("Failed to poll for missed messages")?;

        let mut new_count = 0;
        let mut new_messages = Vec::new();

        for msg in messages {
            let mut seen = self.seen_message_ids.lock().await;
            if seen.contains(&msg.id) {
                // Hit already-seen message, stop
                break;
            }

            seen.insert(msg.id.clone());
            drop(seen); // Release lock before processing

            // Collect messages instead of processing immediately
            if msg.person_email == self.authorized_user_email
                && msg.person_email != self.bot_email
            {
                new_messages.push(msg);
            }
        }

        // Process in reverse order (oldest first)
        for msg in new_messages.into_iter().rev() {
            if let Some(text) = msg.text {
                print_heart_yellow(&format!("📨 Recovered missed message from {}: {}", msg.person_email, text), true);

                let message = MspcMessage::UserInput(
                    text,
                    Some(format!("webex:{}", msg.person_email)),
                );

                if let Err(e) = self.mspc_channel.send(message).await {
                    print_heart_yellow(&format!("⚠️ Failed to send recovered message to MSPC: {}", e), true);
                }

                new_count += 1;
            }
        }

        if new_count > 0 {
            print_heart_yellow(&format!("🔍 Recovered {} missed messages", new_count), true);
        } else {
            print_heart_yellow("🔍 No missed messages during disconnect", true);
        }

        Ok(())
    }

    /// Run the WebSocket router with auto-reconnection
    pub async fn run(&self) -> Result<()> {
        print_heart_yellow(&format!("🌐 Webex WebSocket bot starting - monitoring messages from {}", self.authorized_user_email), true);
        print_heart_yellow(&format!("🔍 Device ID: {}", self.device_id), true);

        let mut reconnect_delay = 0u64; // 0 = immediate first connect

        loop {
            // Reconnection backoff delay
            if reconnect_delay > 0 {
                print_heart_yellow(&format!("🔍 Reconnecting in {}s...", reconnect_delay), true);
                sleep(Duration::from_secs(reconnect_delay)).await;

                // Recover any messages missed during disconnect
                if let Err(e) = self.recover_message_gap().await {
                    print_heart_yellow(&format!("⚠️ Failed to recover message gap: {}", e), true);
                }
            }

            // Try to connect
            match self.run_connection().await {
                Ok(_) => {
                    print_heart_yellow("🔍 WebSocket connection ended cleanly", true);
                    reconnect_delay = 1; // Start backoff on clean close
                }
                Err(e) => {
                    print_heart_yellow(&format!("⚠️ WebSocket error: {}", e), true);
                    // Exponential backoff: 1s, 2s, 4s, 8s, max 30s
                    reconnect_delay = if reconnect_delay == 0 {
                        1
                    } else {
                        (reconnect_delay * 2).min(30)
                    };
                }
            }

            print_heart_yellow("🔍 Attempting reconnection...", true);
        }
    }

    /// Handle a decoded Mercury message (text or binary converted to text)
    async fn handle_mercury_message(&self, text: &str) {
        print_heart_yellow(&format!("🔍 Processing message ({} bytes)", text.len()), true);
        print_heart_yellow(&format!("🔍 Message preview: {}", text.chars().take(200).collect::<String>()), true);

        // Parse Mercury event
        match serde_json::from_str::<MercuryEvent>(text) {
            Ok(event) => {
                print_heart_yellow(&format!("🔍 Parsed event type: {}", event.data.event_type), true);
                if let Err(e) = self.process_event(event).await {
                    print_heart_yellow(&format!("⚠️ Error processing event: {}", e), true);
                }
            }
            Err(e) => {
                print_heart_yellow(&format!("⚠️ Failed to parse Mercury event: {}", e), true);
                print_heart_yellow(&format!("🔍 Raw message: {}", text), true);
            }
        }
    }

    /// Run a single WebSocket connection (extracted for reconnection logic)
    async fn run_connection(&self) -> Result<()> {
        // Connect to Mercury WebSocket
        let (ws_stream, _) = connect_async(&self.websocket_url)
            .await
            .context("Failed to connect to Mercury WebSocket")?;

        print_heart_yellow("🔍 WebSocket connected", true);

        let (mut write, mut read) = ws_stream.split();

        // Send authorization message to Mercury
        let auth_message = serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "type": "authorization",
            "data": {
                "token": format!("Bearer {}", self.access_token)
            }
        });

        let auth_text = serde_json::to_string(&auth_message)?;
        print_heart_yellow("🔍 Sending authorization to Mercury", true);
        write.send(Message::Text(auth_text)).await
            .context("Failed to send authorization message")?;

        // Listen for messages
        while let Some(msg_result) = read.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    print_heart_yellow(&format!("🔍 Received WebSocket text message ({} bytes)", text.len()), true);
                    self.handle_mercury_message(&text).await;
                }
                Ok(Message::Binary(data)) => {
                    print_heart_yellow(&format!("🔍 Received binary message ({} bytes)", data.len()), true);
                    // Mercury sends messages as UTF-8 encoded binary
                    match String::from_utf8(data) {
                        Ok(text) => {
                            self.handle_mercury_message(&text).await;
                        }
                        Err(e) => {
                            print_heart_yellow(&format!("⚠️ Failed to decode binary message as UTF-8: {}", e), true);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    print_heart_yellow("🔍 WebSocket closed by server", true);
                    break;
                }
                Ok(Message::Ping(data)) => {
                    print_heart_yellow(&format!("🔍 Received ping ({} bytes)", data.len()), true);
                }
                Ok(Message::Pong(data)) => {
                    print_heart_yellow(&format!("🔍 Received pong ({} bytes)", data.len()), true);
                }
                Ok(msg) => {
                    print_heart_yellow(&format!("🔍 Received other message type: {:?}", msg), true);
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("WebSocket error: {}", e));
                }
            }
        }

        Ok(())
    }

    /// Get the Webex client (for sharing with output sink)
    pub fn client(&self) -> Arc<WebexClient> {
        Arc::clone(&self.client)
    }

    /// Get the room ID for this router
    pub fn room_id(&self) -> &str {
        &self.room_id
    }
}
