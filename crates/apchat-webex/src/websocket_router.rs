// WebexWebSocketRouter - Real-time message delivery via Mercury WebSocket
use anyhow::{Context, Result};
use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration, Instant};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use apchat_mspc::{MspcChannel, MspcMessage};
use apchat_core::WebexDebugState;
use apchat_vty::print_heart_yellow;
use crate::client::WebexClient;
use crate::types::{MercuryEvent};

/// Reason why a WebSocket connection ended and needs reconnection
#[derive(Debug)]
enum ReconnectReason {
    /// First connection attempt
    Initial,
    /// Connection closed cleanly by the server
    ServerClose,
    /// Connection error (network, protocol, etc.)
    Error(String),
    /// Connection exceeded the configured maximum age
    MaxAgeExceeded { age_secs: u64 },
    /// No data received within the activity timeout
    ActivityTimeout { silent_secs: u64 },
}

impl std::fmt::Display for ReconnectReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReconnectReason::Initial => write!(f, "initial connection"),
            ReconnectReason::ServerClose => write!(f, "server closed connection"),
            ReconnectReason::Error(e) => write!(f, "error: {}", e),
            ReconnectReason::MaxAgeExceeded { age_secs } => {
                write!(f, "connection age {}s exceeded max", age_secs)
            }
            ReconnectReason::ActivityTimeout { silent_secs } => {
                write!(f, "no activity for {}s", silent_secs)
            }
        }
    }
}

/// Interval between client-initiated WebSocket pings (seconds)
const PING_INTERVAL_SECS: u64 = 60;

/// Default proactive reconnection interval (hours)
pub const DEFAULT_RECONNECT_HOURS: u64 = 12;

pub struct WebexWebSocketRouter {
    client: Arc<WebexClient>,
    access_token: String,
    authorized_user_email: String,
    bot_email: String,
    mspc_channel: Arc<MspcChannel>,
    device_id: String,
    websocket_url: Arc<Mutex<String>>,
    room_id: String,
    seen_message_ids: Arc<Mutex<HashSet<String>>>,
    last_user_message_id: Arc<Mutex<Option<String>>>,
    /// Maximum connection age before proactive reconnect (None = disabled)
    reconnect_interval: Option<Duration>,
    /// Total number of connections made (for diagnostics)
    connection_count: Arc<Mutex<u64>>,
    /// Optional debug state for toggling debug output
    debug_state: Option<WebexDebugState>,
}

impl WebexWebSocketRouter {
    /// Helper method to print debug message if debug is enabled
    fn debug_log(&self, message: &str) {
        if let Some(ref state) = self.debug_state {
            if state.is_enabled() {
                print_heart_yellow(message, true);
            }
        }
    }

    /// Create a new WebexWebSocketRouter
    ///
    /// `reconnect_hours` controls proactive reconnection interval:
    /// - `Some(hours)` - reconnect every N hours to prevent stale connections
    /// - `None` - disable proactive reconnection (only reconnect on errors)
    pub async fn new(
        access_token: String,
        user_email: String,
        mspc_channel: Arc<MspcChannel>,
        reconnect_hours: Option<u64>,
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

        let reconnect_interval = reconnect_hours.map(|h| Duration::from_secs(h * 3600));
        let reconnect_desc = match reconnect_interval {
            Some(d) => format!("every {}h", d.as_secs() / 3600),
            None => "disabled".to_string(),
        };

        // Local helper for debug logging (no self available in static method)
        let debug_log = |msg: &str| {
            // Debug logging disabled by default in new()
        };

        debug_log(&format!(
            "🔍 WebSocket router initialized with {} seen messages, proactive reconnect: {}",
            seen_ids.len(),
            reconnect_desc,
        ));

        Ok(Self {
            client: Arc::new(client),
            access_token: token,
            authorized_user_email: user_email.clone(),
            bot_email,
            mspc_channel,
            device_id: format!("webex-{}", &user_email),
            websocket_url: Arc::new(Mutex::new(registration.web_socket_url)),
            room_id,
            seen_message_ids: Arc::new(Mutex::new(seen_ids)),
            last_user_message_id: Arc::new(Mutex::new(None)),
            reconnect_interval,
            connection_count: Arc::new(Mutex::new(0)),
            debug_state: None,
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
            // Skip other event types (e.g., conversation.highlight, membership.activity, etc.)
            self.debug_log(&format!("🔍 Skipping non-activity event type: {}", event.data.event_type));
            return Ok(());
        }

        let activity = match event.data.activity {
            Some(act) => act,
            None => {
                self.debug_log("🔍 Skipping event with no activity data");
                return Ok(());
            }
        };

        // Only process "post" verbs (new messages)
        if activity.verb != "post" {
            self.debug_log(&format!("🔍 Skipping non-post verb: {}", activity.verb));
            return Ok(());
        }

        // Check who posted the message
        let actor_email = activity.actor
            .as_ref()
            .map(|a| a.id.as_str())
            .unwrap_or("");

        self.debug_log(&format!("🔍 Mercury notification - new activity from {}", actor_email));

        // Skip Mercury notifications for bot's own messages
        // This prevents unnecessary API calls and avoids triggering "new messages" indicator
        if actor_email.is_empty() {
            self.debug_log("🔍 Skipping event with empty actor ID");
            return Ok(());
        } else if actor_email == self.bot_email {
            self.debug_log("🔍 Skipping Mercury notification for bot's own message");
            return Ok(());
        } else if actor_email == self.authorized_user_email {
            self.debug_log("🔍 Actor is authorized user, processing...");
        } else {
            self.debug_log(&format!("🔍 Skipping Mercury notification from {}", actor_email));
            return Ok(());
        }

        // Mercury sends encrypted content - we need to fetch the actual messages via REST API
        // Note: Mercury activity.id != REST API message.id (different ID formats)
        // So we just fetch recent messages and process any unseen ones from authorized user
        self.debug_log("🔍 Fetching recent messages from REST API...");
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

            self.debug_log(&format!("🔍 New message [{}] from {} @ {}: {}",
                msg.id.chars().take(8).collect::<String>(),
                msg.person_email.as_deref().unwrap_or("unknown"),
                msg.created,
                msg.text.as_ref().map(|t| t.chars().take(50).collect::<String>()).unwrap_or_else(|| "[no text]".to_string())
            ));

            // Filter: only messages from authorized user, not from bot
            // NOTE: This logic must stay in sync with input_router.rs (see line ~140 for similar implementation)
            if msg.person_email == Some(self.authorized_user_email.clone()) && msg.person_email != Some(self.bot_email.clone()) {
                if let Some(text) = msg.text {
                    let person_email_str = msg.person_email.as_deref().unwrap_or("unknown");
                    print_heart_yellow(&format!("📨 Received Webex message from {}: {}", person_email_str, text), true);

                    // Track this message ID for threading replies
                    {
                        let mut last_id = self.last_user_message_id.lock().await;
                        *last_id = Some(msg.id.clone());
                    }

                    // Prefix the message with sender email and tool reference for clarity
                    let prefixed_text = format!("webex message from {}: [Use send_webex_message tool to respond] {}", person_email_str, text);

                    // Send to MSPC channel
                    let message = MspcMessage::UserInput(
                        prefixed_text,
                        Some(format!("webex:{}", person_email_str)),
                    );

                    if let Err(e) = self.mspc_channel.send(message).await {
                        print_heart_yellow(&format!("⚠️ Failed to send message to MSPC channel: {}", e), true);
                    }

                    processed_count += 1;
                }
            } else if msg.person_email == Some(self.bot_email.clone()) {
                self.debug_log(&format!("🔍 Skipping bot's own message: {}",
                    msg.text.as_ref().map(|t| t.chars().take(50).collect::<String>()).unwrap_or_else(|| "[no text]".to_string())));
            } else {
                self.debug_log(&format!("🔍 Skipping message from {}: {}",
                    msg.person_email.as_deref().unwrap_or("unknown"),
                    msg.text.as_ref().map(|t| t.chars().take(50).collect::<String>()).unwrap_or_else(|| "[no text]".to_string())));
            }
        }

        self.debug_log(&format!("🔍 Processed {} new messages from authorized user", processed_count));

        Ok(())
    }

    /// Poll for missed messages after reconnection
    async fn recover_message_gap(&self) -> Result<()> {
        self.debug_log("🔍 Checking for missed messages during disconnect...");

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
            if msg.person_email == Some(self.authorized_user_email.clone())
                && msg.person_email != Some(self.bot_email.clone())
            {
                new_messages.push(msg);
            }
        }

        // Process in reverse order (oldest first)
        for msg in new_messages.into_iter().rev() {
            if let Some(text) = msg.text {
                print_heart_yellow(&format!("📨 Recovered missed message from {}: {}", msg.person_email.as_deref().unwrap_or("unknown"), text), true);

                let message = MspcMessage::UserInput(
                    text,
                    Some(format!("webex:{}", msg.person_email.as_deref().unwrap_or("unknown"))),
                );

                if let Err(e) = self.mspc_channel.send(message).await {
                    print_heart_yellow(&format!("⚠️ Failed to send recovered message to MSPC: {}", e), true);
                }

                new_count += 1;
            }
        }

        if new_count > 0 {
            self.debug_log(&format!("🔍 Recovered {} missed messages", new_count));
        } else {
            self.debug_log("🔍 No missed messages during disconnect");
        }

        Ok(())
    }

    /// Re-register device to get a fresh Mercury WebSocket URL
    async fn refresh_websocket_url(&self) -> Result<()> {
        self.debug_log("🔍 Re-registering device for fresh WebSocket URL...");
        let registration = self.client
            .register_device()
            .await
            .context("Failed to re-register device")?;
        let mut url = self.websocket_url.lock().await;
        *url = registration.web_socket_url;
        self.debug_log("🔍 Device re-registered, got fresh WebSocket URL");
        Ok(())
    }

    /// Run the WebSocket router with auto-reconnection
    pub async fn run(&self) -> Result<()> {
        self.debug_log(&format!("🌐 Webex WebSocket bot starting - monitoring messages from {}", self.authorized_user_email));
        self.debug_log(&format!("🔍 Device ID: {}", self.device_id));
        if let Some(interval) = self.reconnect_interval {
            self.debug_log(&format!("🔍 Proactive reconnect every {}h", interval.as_secs() / 3600));
        }
        self.debug_log(&format!("🔍 Client ping interval: {}s", PING_INTERVAL_SECS));

        let mut reconnect_delay = 0u64; // 0 = immediate first connect
        let mut last_reason = ReconnectReason::Initial;

        loop {
            // Reconnection backoff delay
            if reconnect_delay > 0 {
                self.debug_log(&format!(
                    "🔍 Reconnecting in {}s (reason: {})...",
                    reconnect_delay, last_reason
                ));
                sleep(Duration::from_secs(reconnect_delay)).await;

                // Re-register device to get a fresh WebSocket URL before reconnecting
                if let Err(e) = self.refresh_websocket_url().await {
                    print_heart_yellow(&format!("⚠️ Failed to refresh WebSocket URL: {} (will try with existing URL)", e), true);
                }

                // Recover any messages missed during disconnect
                if let Err(e) = self.recover_message_gap().await {
                    print_heart_yellow(&format!("⚠️ Failed to recover message gap: {}", e), true);
                }
            }

            // Track connection count
            {
                let mut count = self.connection_count.lock().await;
                *count += 1;
                self.debug_log(&format!(
                    "🔍 Starting connection #{} (reason: {})",
                    *count, last_reason
                ));
            }

            // Try to connect
            match self.run_connection().await {
                Ok(reason) => {
                    last_reason = reason;
                    match &last_reason {
                        ReconnectReason::MaxAgeExceeded { .. } | ReconnectReason::ActivityTimeout { .. } => {
                            // Proactive reconnect - use minimal delay
                            self.debug_log(&format!(
                                "🔍 Proactive reconnect triggered ({})", last_reason
                            ));
                            reconnect_delay = 1;
                        }
                        _ => {
                            self.debug_log("🔍 WebSocket connection ended cleanly");
                            reconnect_delay = 1;
                        }
                    }
                }
                Err(e) => {
                    let err_str = format!("{}", e);
                    print_heart_yellow(&format!("⚠️ WebSocket error: {}", err_str), true);
                    last_reason = ReconnectReason::Error(err_str);
                    // Exponential backoff: 1s, 2s, 4s, 8s, max 30s
                    reconnect_delay = if reconnect_delay == 0 {
                        1
                    } else {
                        (reconnect_delay * 2).min(30)
                    };
                }
            }

            self.debug_log("🔍 Attempting reconnection...");
        }
    }

    /// Handle a decoded Mercury message (text or binary converted to text)
    async fn handle_mercury_message(&self, text: &str) {
        self.debug_log(&format!("🔍 Processing message ({} bytes)", text.len()));
        self.debug_log(&format!("🔍 Message preview: {}", text.chars().take(200).collect::<String>()));

        // Parse Mercury event
        match serde_json::from_str::<MercuryEvent>(text) {
            Ok(event) => {
                self.debug_log(&format!("🔍 Parsed event type: {}", event.data.event_type));
                if let Err(e) = self.process_event(event).await {
                    print_heart_yellow(&format!("⚠️ Error processing event: {}", e), true);
                }
            }
            Err(e) => {
                print_heart_yellow(&format!("⚠️ Failed to parse Mercury event: {}", e), true);
                self.debug_log(&format!("🔍 Raw message: {}", text));
            }
        }
    }

    /// Run a single WebSocket connection with keepalive pings and age tracking
    ///
    /// Returns `Ok(reason)` with the reason for disconnection when the connection
    /// should be retried, or `Err` for connection-level failures.
    async fn run_connection(&self) -> Result<ReconnectReason> {
        let ws_url = self.websocket_url.lock().await.clone();

        // Connect to Mercury WebSocket
        let (ws_stream, _) = connect_async(&ws_url)
            .await
            .context("Failed to connect to Mercury WebSocket")?;

        let connected_at = Instant::now();
        self.debug_log("🔍 WebSocket connected");

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
        self.debug_log("🔍 Sending authorization to Mercury");
        write.send(Message::Text(auth_text)).await
            .context("Failed to send authorization message")?;

        // Timers for ping and connection age
        let ping_interval = Duration::from_secs(PING_INTERVAL_SECS);
        let mut ping_timer = tokio::time::interval(ping_interval);
        ping_timer.tick().await; // First tick completes immediately, consume it to start the interval timer

        let mut last_data_received = Instant::now();
        let mut pings_sent: u64 = 0;
        let mut pongs_received: u64 = 0;
        let mut messages_received: u64 = 0;

        // Listen for messages with periodic ping and age checks
        loop {
            tokio::select! {
                // Incoming WebSocket message
                msg_opt = read.next() => {
                    match msg_opt {
                        Some(Ok(Message::Text(text))) => {
                            last_data_received = Instant::now();
                            messages_received += 1;
                            let age = connected_at.elapsed().as_secs();
                            print_heart_yellow(&format!(
                                "🔍 Received WebSocket text message ({} bytes, conn age {}s, msg #{})",
                                text.len(), age, messages_received
                            ), true);
                            self.handle_mercury_message(&text).await;
                        }
                        Some(Ok(Message::Binary(data))) => {
                            last_data_received = Instant::now();
                            messages_received += 1;
                            let age = connected_at.elapsed().as_secs();
                            print_heart_yellow(&format!(
                                "🔍 Received binary message ({} bytes, conn age {}s, msg #{})",
                                data.len(), age, messages_received
                            ), true);
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
                        Some(Ok(Message::Close(frame))) => {
                            let age = connected_at.elapsed().as_secs();
                            print_heart_yellow(&format!(
                                "🔍 WebSocket closed by server (age {}s, msgs {}, pings {}/{}) frame: {:?}",
                                age, messages_received, pings_sent, pongs_received, frame
                            ), true);
                            return Ok(ReconnectReason::ServerClose);
                        }
                        Some(Ok(Message::Ping(data))) => {
                            last_data_received = Instant::now();
                            self.debug_log(&format!("🔍 Received ping ({} bytes), sending pong", data.len()));
                            if let Err(e) = write.send(Message::Pong(data)).await {
                                print_heart_yellow(&format!("⚠️ Failed to send pong: {}", e), true);
                            }
                        }
                        Some(Ok(Message::Pong(data))) => {
                            last_data_received = Instant::now();
                            pongs_received += 1;
                            print_heart_yellow(&format!(
                                "🔍 Received pong ({} bytes, {}/{})",
                                data.len(), pongs_received, pings_sent
                            ), true);
                        }
                        Some(Ok(msg)) => {
                            last_data_received = Instant::now();
                            self.debug_log(&format!("🔍 Received other message type: {:?}", msg));
                        }
                        Some(Err(e)) => {
                            let age = connected_at.elapsed().as_secs();
                            return Err(anyhow::anyhow!(
                                "WebSocket error after {}s (msgs {}, pings {}/{}): {}",
                                age, messages_received, pings_sent, pongs_received, e
                            ));
                        }
                        None => {
                            let age = connected_at.elapsed().as_secs();
                            print_heart_yellow(&format!(
                                "🔍 WebSocket stream ended (age {}s, msgs {})",
                                age, messages_received
                            ), true);
                            return Ok(ReconnectReason::ServerClose);
                        }
                    }
                }

                // Periodic ping and health check
                _ = ping_timer.tick() => {
                    let age = connected_at.elapsed().as_secs();
                    let silent_secs = last_data_received.elapsed().as_secs();

                    // Check connection age for proactive reconnect
                    if let Some(max_age) = self.reconnect_interval {
                        if connected_at.elapsed() >= max_age {
                            print_heart_yellow(&format!(
                                "🔍 Connection age {}s >= max {}s, triggering proactive reconnect (msgs {}, pings {}/{})",
                                age, max_age.as_secs(), messages_received, pings_sent, pongs_received
                            ), true);
                            return Ok(ReconnectReason::MaxAgeExceeded { age_secs: age });
                        }
                    }

                    // Check for missing pongs (if we've sent pings but many go unanswered)
                    if pings_sent > 0 && pings_sent - pongs_received >= 3 {
                        print_heart_yellow(&format!(
                            "⚠️ No pongs received after {} pings ({}s silent), connection may be dead",
                            pings_sent, silent_secs
                        ), true);
                        return Ok(ReconnectReason::ActivityTimeout { silent_secs });
                    }

                    // Send keepalive ping
                    pings_sent += 1;
                    let ping_data = format!("apchat-ping-{}", pings_sent);
                    match write.send(Message::Ping(ping_data.into_bytes())).await {
                        Ok(_) => {
                            print_heart_yellow(&format!(
                                "🔍 Sent ping #{} (conn age {}s, last data {}s ago)",
                                pings_sent, age, silent_secs
                            ), true);
                        }
                        Err(e) => {
                            print_heart_yellow(&format!(
                                "⚠️ Failed to send ping #{}: {} (conn age {}s)",
                                pings_sent, e, age
                            ), true);
                            return Err(anyhow::anyhow!("Failed to send keepalive ping: {}", e));
                        }
                    }
                }
            }
        }
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
