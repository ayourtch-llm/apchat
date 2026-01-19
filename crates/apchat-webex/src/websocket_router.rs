// WebexWebSocketRouter - Real-time message delivery via Mercury WebSocket
use anyhow::{Context, Result};
use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
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
            authorized_user_email: user_email,
            bot_email,
            mspc_channel,
            device_id: registration.id,
            websocket_url: registration.web_socket_url,
            seen_message_ids: Arc::new(Mutex::new(seen_ids)),
        })
    }

    /// Run the WebSocket router
    pub async fn run(&self) -> Result<()> {
        println!("🌐 Webex WebSocket bot starting - monitoring messages from {}", self.authorized_user_email);
        eprintln!("🔍 DEBUG: Device ID: {}", self.device_id);
        eprintln!("🔍 DEBUG: WebSocket URL: {}", self.websocket_url);

        // Connect to Mercury WebSocket
        let (ws_stream, _) = connect_async(&self.websocket_url)
            .await
            .context("Failed to connect to Mercury WebSocket")?;

        eprintln!("🔍 DEBUG: WebSocket connected");

        let (mut write, mut read) = ws_stream.split();

        // Listen for messages
        while let Some(msg_result) = read.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    eprintln!("🔍 DEBUG: Received WebSocket message: {}",
                        text.chars().take(100).collect::<String>());

                    // Parse Mercury event
                    match serde_json::from_str::<MercuryEvent>(&text) {
                        Ok(event) => {
                            eprintln!("🔍 DEBUG: Parsed event type: {:?}", event.data.event_type);
                            // TODO: Process event (next task)
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
                    eprintln!("⚠️ WebSocket error: {}", e);
                    break;
                }
            }
        }

        eprintln!("🔍 DEBUG: WebSocket connection ended");
        Ok(())
    }

    /// Get the Webex client (for sharing with output sink)
    pub fn client(&self) -> Arc<WebexClient> {
        Arc::clone(&self.client)
    }
}
