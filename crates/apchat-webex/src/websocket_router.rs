// WebexWebSocketRouter - Real-time message delivery via Mercury WebSocket
use anyhow::{Context, Result};
use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::StreamExt;
use apchat_mspc::{MspcChannel, MspcMessage};
use crate::client::WebexClient;
use crate::types::{MercuryEvent};

pub struct WebexWebSocketRouter {
    client: Arc<WebexClient>,
    authorized_user_email: String,
    bot_email: String,
    mspc_channel: Arc<MspcChannel>,
    device_id: String,
    websocket_url: String,
    room_id: String,
    seen_message_ids: Arc<Mutex<HashSet<String>>>,
}

impl WebexWebSocketRouter {
    /// Create a new WebexWebSocketRouter
    pub async fn new(
        access_token: String,
        user_email: String,
        mspc_channel: Arc<MspcChannel>,
    ) -> Result<Self> {
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

        eprintln!("🔍 DEBUG: WebSocket router initialized with {} seen messages", seen_ids.len());

        Ok(Self {
            client: Arc::new(client),
            authorized_user_email: user_email.clone(),
            bot_email,
            mspc_channel,
            device_id: format!("webex-{}", &user_email),
            websocket_url: registration.web_socket_url,
            room_id,
            seen_message_ids: Arc::new(Mutex::new(seen_ids)),
        })
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

        // Extract message data from object
        let obj = activity.object;
        let message_id = obj.get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let person_email = obj.get("personEmail")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let text = obj.get("text")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Check if already seen
        {
            let mut seen = self.seen_message_ids.lock().await;
            if seen.contains(&message_id) {
                eprintln!("🔍 DEBUG: Message {} already seen, skipping", message_id);
                return Ok(());
            }
            // Mark as seen
            seen.insert(message_id.clone());
        }

        eprintln!("🔍 DEBUG: New message from {} (authorized: {}, bot: {})",
            person_email, self.authorized_user_email, self.bot_email);

        // Filter: only messages from authorized user, not from bot
        if person_email == self.authorized_user_email && person_email != self.bot_email {
            if let Some(text) = text {
                println!("📨 Received Webex message from {}: {}", person_email, text);

                // Send to MSPC channel
                let message = MspcMessage::UserInput(
                    text,
                    Some(format!("webex:{}", person_email)),
                );

                if let Err(e) = self.mspc_channel.send(message).await {
                    eprintln!("⚠️ Failed to send message to MSPC channel: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Poll for missed messages after reconnection
    async fn recover_message_gap(&self) -> Result<()> {
        eprintln!("🔍 DEBUG: Checking for missed messages during disconnect...");

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
                println!("📨 Recovered missed message from {}: {}", msg.person_email, text);

                let message = MspcMessage::UserInput(
                    text,
                    Some(format!("webex:{}", msg.person_email)),
                );

                if let Err(e) = self.mspc_channel.send(message).await {
                    eprintln!("⚠️ Failed to send recovered message to MSPC: {}", e);
                }

                new_count += 1;
            }
        }

        if new_count > 0 {
            eprintln!("🔍 DEBUG: Recovered {} missed messages", new_count);
        } else {
            eprintln!("🔍 DEBUG: No missed messages during disconnect");
        }

        Ok(())
    }

    /// Run the WebSocket router with auto-reconnection
    pub async fn run(&self) -> Result<()> {
        println!("🌐 Webex WebSocket bot starting - monitoring messages from {}", self.authorized_user_email);
        eprintln!("🔍 DEBUG: Device ID: {}", self.device_id);

        let mut reconnect_delay = 0u64; // 0 = immediate first connect

        loop {
            // Reconnection backoff delay
            if reconnect_delay > 0 {
                eprintln!("🔍 DEBUG: Reconnecting in {}s...", reconnect_delay);
                sleep(Duration::from_secs(reconnect_delay)).await;

                // Recover any messages missed during disconnect
                if let Err(e) = self.recover_message_gap().await {
                    eprintln!("⚠️ Failed to recover message gap: {}", e);
                }
            }

            // Try to connect
            match self.run_connection().await {
                Ok(_) => {
                    eprintln!("🔍 DEBUG: WebSocket connection ended cleanly");
                    reconnect_delay = 1; // Start backoff on clean close
                }
                Err(e) => {
                    eprintln!("⚠️ WebSocket error: {}", e);
                    // Exponential backoff: 1s, 2s, 4s, 8s, max 30s
                    reconnect_delay = if reconnect_delay == 0 {
                        1
                    } else {
                        (reconnect_delay * 2).min(30)
                    };
                }
            }

            eprintln!("🔍 DEBUG: Attempting reconnection...");
        }
    }

    /// Run a single WebSocket connection (extracted for reconnection logic)
    async fn run_connection(&self) -> Result<()> {
        // Connect to Mercury WebSocket
        let (ws_stream, _) = connect_async(&self.websocket_url)
            .await
            .context("Failed to connect to Mercury WebSocket")?;

        eprintln!("🔍 DEBUG: WebSocket connected");

        let (mut _write, mut read) = ws_stream.split();

        // Listen for messages
        while let Some(msg_result) = read.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    // Parse Mercury event
                    match serde_json::from_str::<MercuryEvent>(&text) {
                        Ok(event) => {
                            if let Err(e) = self.process_event(event).await {
                                eprintln!("⚠️ Error processing event: {}", e);
                            }
                        }
                        Err(e) => {
                            eprintln!("⚠️ Failed to parse Mercury event: {}", e);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    eprintln!("🔍 DEBUG: WebSocket closed by server");
                    break;
                }
                Ok(_) => {
                    // Ignore ping/pong/binary messages
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
