# Issue 102: WebSocket Connection to MSPC Channel

## Summary
The WebSocket router exists but is not connected to the MSPC channel system. WebSocket messages should be routed through the MSPC channel.

## Location
- File: `src/web/routes.rs`
- Function: `handle_message`

## Current Behavior
The WebSocket handler receives messages and processes them directly without going through the MSPC channel system. This bypasses the input decoupling architecture.

## Expected Behavior
WebSocket messages should:
1. Be sent to the MSPC channel
2. Go through the MSPC chat loop
3. Be processed alongside terminal inputs
4. Maintain consistent message history

## Impact
Without this connection, the input decoupling architecture is incomplete. WebSocket inputs won't benefit from the MSPC infrastructure (interrupt handling, command parsing, message history).

## Suggested Implementation

### Step 1: Update WebSocket handler to use MSPC

In `src/web/routes.rs`, modify the message handler:

```rust
// Add to struct fields
pub struct WebServer {
    // ... existing fields ...
    mspc_tx: Option<mpsc::Sender<MSPCMessage>>, // Add this
}

// Update new function
impl WebServer {
    pub fn new(config: WebServerConfig, mspc_tx: mpsc::Sender<MSPCMessage>) -> Self {
        Self {
            // ... existing fields ...
            mspc_tx: Some(mspc_tx),
        }
    }

    async fn handle_message(&self, message: String, session_id: String) -> Result<(), String> {
        // Parse message
        let parsed = match parse_webex_message(&message) {
            Ok(m) => m,
            Err(e) => return Err(format!("Parse error: {}", e)),
        };

        // Send to MSPC channel instead of direct processing
        if let Some(tx) = &self.mspc_tx {
            let mspc_msg = MSPCMessage::WebexInput {
                content: parsed.content,
                user_id: parsed.user_id,
                session_id: parsed.session_id,
                timestamp: Utc::now(),
            };
            
            if tx.send(mspc_msg).await.is_err() {
                return Err("Failed to send message to MSPC channel".to_string());
            }
        }

        Ok(())
    }
}
```

### Step 2: Update WebServer initialization

In `src/app/setup.rs` or wherever the web server is created:

```rust
// Create MSPC channel
let (mspc_tx, mspc_rx) = mspc_channel::create_channel();

// Initialize web server with MSPC transmitter
let web_server = WebServer::new(
    web_config,
    mspc_tx.clone(),
);

// Spawn web server
tokio::spawn(async move {
    web_server.run().await
});

// Spawn MSPC chat loop with receiver
let mspc_loop = MSPCChatLoop::new(
    mspc_rx,
    mspc_tx.clone(),
    session,
    config,
    logger,
);
tokio::spawn(async move {
    mspc_loop.run().await
});
```

### Step 3: Update Webex Input Router

The existing Webex input router should be removed or repurposed since WebSocket messages will now go directly to MSPC.

```rust
// In src/input_router/webex.rs

// Remove or update the router to be a no-op or for future use
pub struct WebexInputRouter;

impl WebexInputRouter {
    pub fn new(_tx: mpsc::Sender<MSPCMessage>) -> Self {
        Self
    }

    pub async fn run(&self) {
        // Future: Connect to Webex API if needed
        // Currently WebSocket handles direct connections
    }
}
```

## Resolution

This issue will be resolved by:

1. Updating WebSocket routes to send messages to MSPC channel
2. Updating WebServer initialization to receive MSPC transmitter
3. Removing/updating the Webex input router
4. Ensuring message consistency between WebSocket and terminal inputs

**Files Modified:**
- `src/web/routes.rs`
- `src/app/setup.rs`
- `src/input_router/webex.rs`

**Testing:**
- Test WebSocket message routing
- Verify messages appear in MSPC channel
- Test message history consistency
- Test interrupt handling via WebSocket
- Test command parsing via WebSocket

---
*Created: 2026-01-18*
