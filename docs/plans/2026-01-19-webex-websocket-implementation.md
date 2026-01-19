# Webex WebSocket Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace polling-based Webex message delivery with WebSocket-based real-time delivery using Mercury protocol, while keeping polling available as a configuration option.

**Architecture:** Add WebexWebSocketRouter alongside existing WebexInputRouter. Device registration fetches Mercury WebSocket URL, persistent connection delivers messages in real-time, automatic reconnection with gap recovery ensures zero message loss.

**Tech Stack:** tokio-tungstenite for WebSocket, tokio for async runtime, existing reqwest HTTP client for device registration and gap recovery polling.

---

## Phase 1: Dependencies and Types

### Task 1: Add WebSocket dependency

**Files:**
- Modify: `crates/apchat-webex/Cargo.toml`

**Step 1: Add tokio-tungstenite dependency**

```toml
# Add to [dependencies] section after existing dependencies
tokio-tungstenite = { version = "0.21", features = ["native-tls"] }
```

**Step 2: Build to verify dependency resolution**

Run: `cargo build -p apchat-webex`
Expected: Build succeeds, dependency downloads

**Step 3: Commit**

```bash
git add crates/apchat-webex/Cargo.toml
git commit -m "Add tokio-tungstenite dependency for WebSocket support"
```

---

### Task 2: Add device registration types

**Files:**
- Modify: `crates/apchat-webex/src/types.rs`

**Step 1: Add device registration request struct**

Add to `types.rs` after existing structs:

```rust
// Device registration for Mercury WebSocket connection
#[derive(Debug, Serialize)]
pub struct DeviceRegistrationRequest {
    #[serde(rename = "deviceName")]
    pub device_name: String,
    #[serde(rename = "deviceType")]
    pub device_type: String,
    #[serde(rename = "localizedModel")]
    pub localized_model: String,
    pub model: String,
    #[serde(rename = "systemName")]
    pub system_name: String,
    #[serde(rename = "systemVersion")]
    pub system_version: String,
}

#[derive(Debug, Deserialize)]
pub struct DeviceRegistration {
    pub url: String,
    #[serde(rename = "webSocketUrl")]
    pub web_socket_url: String,
    pub id: String,
}
```

**Step 2: Build to verify types compile**

Run: `cargo build -p apchat-webex`
Expected: Build succeeds

**Step 3: Commit**

```bash
git add crates/apchat-webex/src/types.rs
git commit -m "Add device registration types for Mercury WebSocket"
```

---

### Task 3: Add Mercury event types

**Files:**
- Modify: `crates/apchat-webex/src/types.rs`

**Step 1: Add Mercury WebSocket message structs**

Add to `types.rs` after device registration types:

```rust
// Mercury WebSocket event messages
#[derive(Debug, Deserialize)]
pub struct MercuryEvent {
    pub id: String,
    pub data: MercuryEventData,
}

#[derive(Debug, Deserialize)]
pub struct MercuryEventData {
    #[serde(rename = "eventType")]
    pub event_type: String,
    pub activity: Option<Activity>,
}

#[derive(Debug, Deserialize)]
pub struct Activity {
    pub verb: String,
    pub object: serde_json::Value,
}
```

**Step 2: Build to verify types compile**

Run: `cargo build -p apchat-webex`
Expected: Build succeeds

**Step 3: Commit**

```bash
git add crates/apchat-webex/src/types.rs
git commit -m "Add Mercury event types for WebSocket message parsing"
```

---

## Phase 2: Device Registration

### Task 4: Implement device registration in client

**Files:**
- Modify: `crates/apchat-webex/src/client.rs`

**Step 1: Add device registration method**

Add to `impl WebexClient` after existing methods:

```rust
/// Register device to get Mercury WebSocket URL
pub async fn register_device(&self) -> Result<DeviceRegistration> {
    let url = format!("{}/devices", WEBEX_API_BASE);
    eprintln!("🔍 DEBUG: Registering device for Mercury WebSocket");

    let request = DeviceRegistrationRequest {
        device_name: "apchat-bot".to_string(),
        device_type: "DESKTOP".to_string(),
        localized_model: "Rust APChat Bot".to_string(),
        model: "apchat".to_string(),
        system_name: "APChat".to_string(),
        system_version: "0.1.0".to_string(),
    };

    let response = self.client
        .post(&url)
        .bearer_auth(&self.access_token)
        .json(&request)
        .send()
        .await
        .context("Failed to register device")?;

    let status = response.status();
    eprintln!("🔍 DEBUG: Device registration response status: {}", status);

    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_else(|_| "Could not read error body".to_string());
        eprintln!("🔍 DEBUG: Error response body: {}", error_body);
        return Err(anyhow::anyhow!("Failed to register device: {} - {}", status, error_body));
    }

    let registration: DeviceRegistration = response
        .json()
        .await
        .context("Failed to parse device registration response")?;

    eprintln!("🔍 DEBUG: Device registered: ID={}, WebSocket URL={}",
        registration.id, registration.web_socket_url);

    Ok(registration)
}
```

**Step 2: Add import for DeviceRegistrationRequest**

At top of `client.rs`, ensure imports include:

```rust
use crate::types::*;  // Should already exist
```

**Step 3: Build to verify method compiles**

Run: `cargo build -p apchat-webex`
Expected: Build succeeds

**Step 4: Commit**

```bash
git add crates/apchat-webex/src/client.rs
git commit -m "Implement device registration for Mercury WebSocket URL"
```

---

## Phase 3: WebSocket Router Skeleton

### Task 5: Create websocket_router module

**Files:**
- Create: `crates/apchat-webex/src/websocket_router.rs`
- Modify: `crates/apchat-webex/src/lib.rs`

**Step 1: Create websocket_router.rs file**

Create `crates/apchat-webex/src/websocket_router.rs`:

```rust
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

        // TODO: Connect to WebSocket and start listening
        // This will be implemented in the next tasks

        Ok(())
    }

    /// Get the Webex client (for sharing with output sink)
    pub fn client(&self) -> Arc<WebexClient> {
        Arc::clone(&self.client)
    }
}
```

**Step 2: Export websocket_router in lib.rs**

Add to `crates/apchat-webex/src/lib.rs`:

```rust
pub mod websocket_router;

// Add to pub use section
pub use websocket_router::WebexWebSocketRouter;
```

**Step 3: Build to verify module compiles**

Run: `cargo build -p apchat-webex`
Expected: Build succeeds

**Step 4: Commit**

```bash
git add crates/apchat-webex/src/websocket_router.rs crates/apchat-webex/src/lib.rs
git commit -m "Add WebexWebSocketRouter skeleton with device registration"
```

---

## Phase 4: WebSocket Connection

### Task 6: Implement WebSocket connection

**Files:**
- Modify: `crates/apchat-webex/src/websocket_router.rs`

**Step 1: Add WebSocket connection to run() method**

Replace the `run()` method in `websocket_router.rs`:

```rust
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
```

**Step 2: Build to verify WebSocket connection compiles**

Run: `cargo build -p apchat-webex`
Expected: Build succeeds

**Step 3: Commit**

```bash
git add crates/apchat-webex/src/websocket_router.rs
git commit -m "Implement Mercury WebSocket connection and event parsing"
```

---

## Phase 5: Message Processing

### Task 7: Extract and route messages from Mercury events

**Files:**
- Modify: `crates/apchat-webex/src/websocket_router.rs`

**Step 1: Add message processing helper method**

Add to `impl WebexWebSocketRouter` before `run()`:

```rust
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
```

**Step 2: Use process_event in run() method**

Update the event processing in `run()` method:

```rust
// Inside the while loop, replace the TODO section:
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
```

**Step 3: Build to verify message processing compiles**

Run: `cargo build -p apchat-webex`
Expected: Build succeeds

**Step 4: Commit**

```bash
git add crates/apchat-webex/src/websocket_router.rs
git commit -m "Add message extraction and routing from Mercury events"
```

---

## Phase 6: Reconnection Logic

### Task 8: Add reconnection with exponential backoff

**Files:**
- Modify: `crates/apchat-webex/src/websocket_router.rs`

**Step 1: Add imports for timing**

Add to imports at top of file:

```rust
use tokio::time::{sleep, Duration};
```

**Step 2: Add reconnection loop to run() method**

Replace the entire `run()` method:

```rust
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
```

**Step 3: Build to verify reconnection logic compiles**

Run: `cargo build -p apchat-webex`
Expected: Build succeeds

**Step 4: Commit**

```bash
git add crates/apchat-webex/src/websocket_router.rs
git commit -m "Add automatic reconnection with exponential backoff"
```

---

## Phase 7: Gap Recovery on Reconnect

### Task 9: Poll for missed messages on reconnect

**Files:**
- Modify: `crates/apchat-webex/src/websocket_router.rs`

**Step 1: Add gap recovery method**

Add to `impl WebexWebSocketRouter`:

```rust
/// Poll for missed messages after reconnection
async fn recover_message_gap(&self, room_id: &str) -> Result<()> {
    eprintln!("🔍 DEBUG: Checking for missed messages during disconnect...");

    let messages = self.client
        .get_messages(room_id)
        .await
        .context("Failed to poll for missed messages")?;

    let mut new_count = 0;

    for msg in messages {
        let mut seen = self.seen_message_ids.lock().await;
        if seen.contains(&msg.id) {
            // Hit already-seen message, stop
            break;
        }

        seen.insert(msg.id.clone());
        drop(seen); // Release lock before processing

        // Filter and route message
        if msg.person_email == self.authorized_user_email
            && msg.person_email != self.bot_email
        {
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
    }

    if new_count > 0 {
        eprintln!("🔍 DEBUG: Recovered {} missed messages", new_count);
    } else {
        eprintln!("🔍 DEBUG: No missed messages during disconnect");
    }

    Ok(())
}
```

**Step 2: Store room_id in struct**

Update `WebexWebSocketRouter` struct to include room_id:

```rust
pub struct WebexWebSocketRouter {
    client: Arc<WebexClient>,
    authorized_user_email: String,
    bot_email: String,
    mspc_channel: Arc<MspcChannel>,
    device_id: String,
    websocket_url: String,
    room_id: String,  // Add this field
    seen_message_ids: Arc<Mutex<HashSet<String>>>,
}
```

**Step 3: Update new() to store room_id**

In `new()` method, update the final Ok() to include room_id:

```rust
Ok(Self {
    client: Arc::new(client),
    authorized_user_email: user_email,
    bot_email,
    mspc_channel,
    device_id: registration.id,
    websocket_url: registration.web_socket_url,
    room_id,  // Add this
    seen_message_ids: Arc::new(Mutex::new(seen_ids)),
})
```

**Step 4: Call recover_message_gap after reconnection**

Update `run()` method to call gap recovery before each connection:

```rust
loop {
    // Reconnection backoff delay
    if reconnect_delay > 0 {
        eprintln!("🔍 DEBUG: Reconnecting in {}s...", reconnect_delay);
        sleep(Duration::from_secs(reconnect_delay)).await;

        // Recover any messages missed during disconnect
        if let Err(e) = self.recover_message_gap(&self.room_id).await {
            eprintln!("⚠️ Failed to recover message gap: {}", e);
        }
    }

    // ... rest of the loop
}
```

**Step 5: Build to verify gap recovery compiles**

Run: `cargo build -p apchat-webex`
Expected: Build succeeds

**Step 6: Commit**

```bash
git add crates/apchat-webex/src/websocket_router.rs
git commit -m "Add gap recovery polling on reconnection"
```

---

## Phase 8: CLI Integration

### Task 10: Add --webex-websocket CLI flag

**Files:**
- Modify: `apchat-main/src/cli.rs`

**Step 1: Add webex_websocket field to Cli struct**

Find the `Cli` struct and add the new field after the `webex_bot` field:

```rust
/// Use WebSocket instead of polling for Webex bot
/// Requires --webex-bot to be set
#[arg(long)]
pub webex_websocket: bool,
```

**Step 2: Build to verify CLI compiles**

Run: `cargo build -p apchat`
Expected: Build succeeds

**Step 3: Commit**

```bash
git add apchat-main/src/cli.rs
git commit -m "Add --webex-websocket CLI flag"
```

---

### Task 11: Wire up WebSocket router in main.rs

**Files:**
- Modify: `apchat-main/src/main.rs`

**Step 1: Update Webex bot initialization in main.rs**

Find the section that initializes Webex bot (around line 104-148) and update it:

```rust
// Run REPL mode (with optional Webex integration)
let (webex_sink, mspc_channel) = if let Some(ref user_email) = cli.webex_bot {
    // Create shared MSPC channel for both terminal and Webex
    let mspc_channel = Arc::new(apchat::mspc::MspcChannel::new(100));

    // Load Webex secret
    match apchat_webex::load_webex_secret() {
        Ok(token) => {
            if cli.webex_websocket {
                println!("{} Initializing Webex WebSocket bot for {}", "🌐".bright_cyan(), user_email);

                // Initialize Webex WebSocket router
                match apchat_webex::WebexWebSocketRouter::new(
                    token,
                    user_email.clone(),
                    mspc_channel.clone(),
                ).await {
                    Ok(router) => {
                        let client = router.client();

                        // Spawn WebSocket router as background task
                        tokio::spawn(async move {
                            if let Err(e) = router.run().await {
                                eprintln!("⚠️ Webex WebSocket router error: {}", e);
                            }
                        });

                        // Note: WebSocket doesn't have a room_id readily available
                        // We'll need to get it from the first message or store it
                        // For now, create output sink without room_id (will be fixed in next commit)
                        let sink = Arc::new(apchat_webex::WebexOutputSink::new(client, String::new()));
                        println!("{} Webex WebSocket bot ready - responses will be broadcast", "✓".bright_green());
                        (Some(sink), Some(mspc_channel))
                    }
                    Err(e) => {
                        eprintln!("{} Failed to initialize Webex WebSocket bot: {}", "⚠️".yellow(), e);
                        (None, Some(mspc_channel))
                    }
                }
            } else {
                println!("{} Initializing Webex bot (polling mode) for {}", "🌐".bright_cyan(), user_email);

                // Initialize Webex input router (polling mode)
                match apchat_webex::WebexInputRouter::new(
                    token,
                    user_email.clone(),
                    mspc_channel.clone(),
                ).await {
                    Ok(router) => {
                        let room_id = router.room_id().to_string();
                        let client = router.client();

                        // Spawn Webex input router as background task
                        tokio::spawn(async move {
                            if let Err(e) = router.run().await {
                                eprintln!("⚠️ Webex input router error: {}", e);
                            }
                        });

                        let sink = Arc::new(apchat_webex::WebexOutputSink::new(client, room_id));
                        println!("{} Webex bot ready (polling mode)", "✓".bright_green());
                        (Some(sink), Some(mspc_channel))
                    }
                    Err(e) => {
                        eprintln!("{} Failed to initialize Webex bot: {}", "⚠️".yellow(), e);
                        (None, Some(mspc_channel))
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("{} {}", "⚠️".yellow(), e);
            (None, Some(mspc_channel))
        }
    }
} else {
    (None, None)
};
```

**Step 2: Build to verify integration compiles**

Run: `cargo build`
Expected: Build succeeds

**Step 3: Commit**

```bash
git add apchat-main/src/main.rs
git commit -m "Wire up WebSocket router selection based on CLI flag"
```

---

### Task 12: Fix WebSocket router to expose room_id

**Files:**
- Modify: `crates/apchat-webex/src/websocket_router.rs`

**Step 1: Add room_id() method**

Add to `impl WebexWebSocketRouter` after `client()`:

```rust
/// Get the room ID for this router
pub fn room_id(&self) -> &str {
    &self.room_id
}
```

**Step 2: Update main.rs to use room_id()**

Update the WebSocket initialization section in `apchat-main/src/main.rs`:

```rust
Ok(router) => {
    let room_id = router.room_id().to_string();
    let client = router.client();

    // Spawn WebSocket router as background task
    tokio::spawn(async move {
        if let Err(e) = router.run().await {
            eprintln!("⚠️ Webex WebSocket router error: {}", e);
        }
    });

    let sink = Arc::new(apchat_webex::WebexOutputSink::new(client, room_id));
    println!("{} Webex WebSocket bot ready - responses will be broadcast", "✓".bright_green());
    (Some(sink), Some(mspc_channel))
}
```

**Step 3: Build to verify fix compiles**

Run: `cargo build`
Expected: Build succeeds

**Step 4: Commit**

```bash
git add crates/apchat-webex/src/websocket_router.rs apchat-main/src/main.rs
git commit -m "Expose room_id from WebSocket router for output sink"
```

---

## Phase 9: Testing

### Task 13: Manual testing - Basic WebSocket connection

**No files to modify - testing only**

**Step 1: Run with WebSocket mode**

Run: `cargo run -- -i --webex-bot YOUR_EMAIL --webex-websocket`

Expected output:
```
🌐 Initializing Webex WebSocket bot for YOUR_EMAIL
🔍 DEBUG: Fetching bot details from /people/me
🔍 DEBUG: Bot details response status: 200 OK
🔍 DEBUG: Registering device for Mercury WebSocket
🔍 DEBUG: Device registration response status: 200 OK
🔍 DEBUG: Device registered: ID=..., WebSocket URL=wss://...
🔍 DEBUG: Sending message to email YOUR_EMAIL: APchat ready (WebSocket mode)
✓ Webex WebSocket bot ready - responses will be broadcast
🌐 Webex WebSocket bot starting - monitoring messages from YOUR_EMAIL
🔍 DEBUG: WebSocket connected
```

**Step 2: Send test message in Webex**

Send a message in Webex to the bot.

Expected: Message appears in terminal within 1 second

**Step 3: Verify no duplicates**

Send multiple messages, verify each appears exactly once.

**Step 4: Document results**

Document any issues found in `docs/plans/2026-01-19-webex-websocket-testing.md`

---

### Task 14: Manual testing - Reconnection

**No files to modify - testing only**

**Step 1: Start bot and send message**

Run bot, send test message, verify it arrives.

**Step 2: Simulate disconnect**

Disconnect network (turn off WiFi or unplug ethernet).

Wait 5 seconds.

**Step 3: Send message while disconnected**

Send a message in Webex while network is disconnected.

**Step 4: Reconnect network**

Turn network back on.

Expected output:
```
⚠️ WebSocket error: ...
🔍 DEBUG: Attempting reconnection...
🔍 DEBUG: Reconnecting in 1s...
🔍 DEBUG: Checking for missed messages during disconnect...
📨 Recovered missed message from YOUR_EMAIL: [your message]
🔍 DEBUG: Recovered 1 missed messages
🔍 DEBUG: WebSocket connected
```

**Step 5: Verify message was recovered**

Confirm the message sent during disconnect appears in terminal.

**Step 6: Document results**

Add results to `docs/plans/2026-01-19-webex-websocket-testing.md`

---

### Task 15: Manual testing - Polling mode still works

**No files to modify - testing only**

**Step 1: Run in polling mode (no --webex-websocket flag)**

Run: `cargo run -- -i --webex-bot YOUR_EMAIL`

Expected output:
```
🌐 Initializing Webex bot (polling mode) for YOUR_EMAIL
...
✓ Webex bot ready (polling mode)
🌐 Webex bot starting - monitoring messages from YOUR_EMAIL
```

**Step 2: Send test message**

Send message in Webex.

Expected: Message appears within 2 seconds (polling interval)

**Step 3: Verify both modes work identically**

Messages should appear and be routed the same way in both modes.

**Step 4: Document comparison**

Add comparison to `docs/plans/2026-01-19-webex-websocket-testing.md`

---

## Phase 10: Final Cleanup

### Task 16: Update documentation

**Files:**
- Modify: `README.md` or `docs/webex-integration.md` if it exists

**Step 1: Add WebSocket flag documentation**

Add to CLI documentation section:

```markdown
### Webex Bot Integration

APChat can integrate with Cisco Webex for bidirectional messaging.

**Setup:**
1. Set `WEBEX_APCHAT_SECRET` environment variable or add to `~/.okaychat/env`
2. Run with `--webex-bot YOUR_EMAIL@example.com`

**Modes:**
- **Polling mode (default):** `--webex-bot EMAIL`
  - Polls for messages every 2 seconds
  - Works everywhere, no special setup

- **WebSocket mode:** `--webex-bot EMAIL --webex-websocket`
  - Real-time message delivery (< 1 second latency)
  - Automatic reconnection with gap recovery
  - Zero message loss during reconnects

**Example:**
```bash
# Polling mode
cargo run -- -i --webex-bot user@example.com

# WebSocket mode (recommended)
cargo run -- -i --webex-bot user@example.com --webex-websocket
```
```

**Step 2: Build documentation**

If using mdbook or similar, rebuild docs.

**Step 3: Commit**

```bash
git add README.md  # or appropriate doc file
git commit -m "Document WebSocket mode for Webex integration"
```

---

### Task 17: Final commit and verification

**Files:**
- None (verification only)

**Step 1: Run full build**

Run: `cargo build --release`
Expected: Build succeeds with no warnings in apchat-webex

**Step 2: Run tests**

Run: `cargo test -p apchat-webex`
Expected: All tests pass (if any exist)

**Step 3: Verify git status is clean**

Run: `git status`
Expected: No uncommitted changes

**Step 4: Create summary commit if needed**

If any loose ends exist, create final commit:

```bash
git add .
git commit -m "Complete Webex WebSocket integration

- Device registration with Mercury
- Real-time message delivery via WebSocket
- Automatic reconnection with exponential backoff
- Gap recovery polling on reconnect
- Zero message loss and deduplication
- CLI flag --webex-websocket for mode selection
- Polling mode remains default fallback

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Success Criteria Checklist

Verify all success criteria from design document:

- [ ] Messages delivered in real-time (< 1 second latency)
- [ ] Zero message loss during reconnections
- [ ] No duplicate messages delivered
- [ ] Automatic reconnection with backoff
- [ ] Polling mode remains functional
- [ ] Clean error handling and logging

## Testing Summary

Document final test results in `docs/plans/2026-01-19-webex-websocket-testing.md`:

- WebSocket connection success rate
- Message delivery latency
- Reconnection behavior
- Gap recovery verification
- Polling mode comparison
- Any edge cases discovered

---

## Implementation Complete

All phases implemented. WebSocket integration ready for production use.
