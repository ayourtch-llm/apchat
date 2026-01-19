# Webex WebSocket Integration Design

**Date:** 2026-01-19
**Status:** Approved

## Overview

Replace polling-based Webex message delivery with WebSocket-based real-time delivery using Mercury protocol. Keep polling as a configuration option for backwards compatibility and environments where WebSocket may not work.

## Goals

- Real-time message delivery with zero latency
- Eliminate continuous API polling overhead
- Maintain reliability with automatic reconnection
- Zero message loss during reconnections
- Keep polling mode available as fallback

## Architecture

### Component Structure

```
WebexClient (shared HTTP client)
├── get_messages() - REST API polling (used by both routers)
├── send_message() - Message sending
├── register_device() - NEW: Get Mercury WebSocket URL
└── get_device() - NEW: Refresh WebSocket URL

WebexInputRouter (existing)
└── Polling-based (every 2 seconds)

WebexWebSocketRouter (new)
├── Persistent WebSocket connection
├── Auto-reconnect with backoff
└── Poll on reconnect for gap recovery

Main.rs
└── Router selection based on --webex-websocket flag
```

### Router Selection

```rust
if cli.webex_websocket {
    WebexWebSocketRouter::new(token, email, mspc_channel).await
} else {
    WebexInputRouter::new(token, email, mspc_channel).await
}
```

Both routers share the same interface: route messages from authorized user to MSPC channel.

## WebSocket Connection Flow

### 1. Device Registration (Startup)

```
POST /devices
Request: {
  "deviceName": "apchat-bot",
  "deviceType": "DESKTOP",
  ...
}

Response: {
  "url": "https://wdm-a.wbx2.com/wdm/api/v1/devices/...",
  "webSocketUrl": "wss://mercury-connection-a.wbx2.com/v1/...",
  "id": "device-uuid"
}
```

### 2. WebSocket Connection

- Connect to `webSocketUrl` from registration
- Send authorization message with access token
- Mercury acknowledges connection
- Start listening for events

### 3. Initial Message Sync

- Send "APchat ready" startup message via REST API
- Poll `get_messages()` to get all existing messages
- Mark all message IDs as seen in HashSet
- Ready to receive new messages via WebSocket

### 4. Mercury Event Format

```json
{
  "id": "event-uuid",
  "data": {
    "eventType": "conversation.activity",
    "activity": {
      "verb": "post",
      "object": {
        "id": "msg-id",
        "roomId": "room-id",
        "personEmail": "user@example.com",
        "text": "message text"
      }
    }
  }
}
```

Parse events, extract message data, filter by authorized user, route to MSPC channel.

## Reconnection & Gap Recovery

### Disconnect Detection

WebSocket library signals connection closed (clean or error).

### Exponential Backoff

- First retry: immediate
- Subsequent retries: 1s, 2s, 4s, 8s, max 30s
- Continue until successful

### Gap Recovery Process

```rust
// On reconnect:
1. Re-register device (get fresh WebSocket URL)
2. Connect WebSocket
3. Poll get_messages() to catch missed messages
4. Filter: only process messages NOT in seen_ids HashSet
5. Resume WebSocket listening
```

### Message Deduplication

Maintain single `HashSet<String>` of seen message IDs:
- Populated from initial sync
- Updated from WebSocket events
- Updated from reconnect polls
- Checked before processing any message

Ensures zero duplicates across disconnects and reconnects.

## Data Structures

### New Types (types.rs)

```rust
// Device registration
#[derive(Serialize)]
pub struct DeviceRegistrationRequest {
    pub device_name: String,
    pub device_type: String,
    pub localized_model: String,
    pub model: String,
    pub system_name: String,
    pub system_version: String,
}

#[derive(Deserialize)]
pub struct DeviceRegistration {
    pub url: String,
    pub web_socket_url: String,
    pub id: String,
}

// Mercury WebSocket messages
#[derive(Deserialize)]
pub struct MercuryEvent {
    pub id: String,
    pub data: MercuryEventData,
}

#[derive(Deserialize)]
pub struct MercuryEventData {
    #[serde(rename = "eventType")]
    pub event_type: String,
    pub activity: Option<Activity>,
}

#[derive(Deserialize)]
pub struct Activity {
    pub verb: String,
    pub object: serde_json::Value,
}
```

### WebexWebSocketRouter Structure

```rust
pub struct WebexWebSocketRouter {
    client: Arc<WebexClient>,
    authorized_user_email: String,
    bot_email: String,
    mspc_channel: Arc<MspcChannel>,
    device_id: String,
    websocket_url: String,
    seen_message_ids: Arc<Mutex<HashSet<String>>>,
}
```

## Error Handling

- **WebSocket errors:** Trigger reconnection with backoff
- **Device registration errors:** Retry with backoff, log errors
- **Message parsing errors:** Log and skip individual message, continue listening
- **MSPC send errors:** Log but continue (consistent with polling mode)

## Dependencies

Add to `crates/apchat-webex/Cargo.toml`:
```toml
tokio-tungstenite = "0.21"
```

(futures-util already present)

## Implementation Phases

### Phase 1: Device Registration
- Add `register_device()` to `client.rs`
- Add new types to `types.rs`
- Test device registration returns WebSocket URL

### Phase 2: Basic WebSocket Connection
- Create `websocket_router.rs` skeleton
- Implement WebSocket connection to Mercury
- Parse and log incoming events

### Phase 3: Message Routing
- Extract message data from Mercury events
- Route to MSPC channel with user filtering
- Add message deduplication with HashSet

### Phase 4: Reconnection & Gap Recovery
- Implement disconnect detection
- Add exponential backoff reconnection
- Poll on reconnect to catch missed messages

### Phase 5: Integration
- Add `--webex-websocket` CLI flag to `cli.rs`
- Wire up router selection in `main.rs`
- End-to-end testing with real Webex account

## Testing Strategy

### Unit Testing
- Mock WebSocket responses for message parsing
- Test reconnection backoff logic
- Test message deduplication with HashSet

### Manual Testing
- Test with real Webex account using `--webex-websocket` flag
- Verify messages arrive in real-time
- Simulate disconnect (kill network) and verify reconnect + gap recovery
- Compare side-by-side with polling mode

### Fallback Verification
- Ensure polling mode still works when flag not present
- Both modes should produce identical message delivery

## Success Criteria

- ✓ Messages delivered in real-time (< 1 second latency)
- ✓ Zero message loss during reconnections
- ✓ No duplicate messages delivered
- ✓ Automatic reconnection with backoff
- ✓ Polling mode remains functional
- ✓ Clean error handling and logging

## Non-Goals

- Supporting multiple concurrent users (single authorized user only)
- Group space support (direct messages only)
- Sending attachments (text only)
- Message editing/deletion events
