# Webex Bot Integration Design

**Date:** 2026-01-19
**Status:** Approved - Ready for Implementation

## Overview

Add Webex bot integration to APChat, allowing a single authorized user to interact with APChat through Webex direct messages while maintaining parallel terminal access. All AI responses are broadcast to both terminal and Webex.

## Requirements

- Single authorized user (specified via CLI)
- Direct messages only (1:1 with bot)
- Polling-based message monitoring (works behind firewalls)
- Bi-directional communication (receive messages, send responses)
- Broadcast responses to both terminal and Webex
- Secret loaded from `WEBEX_APCHAT_SECRET` environment variable or `~/.okaychat/env`
- Startup notification: "APchat ready" sent to Webex on connection

## Architecture

### Component Structure

The Webex bot integrates into APChat's MSPC (multi-source, multi-sink) architecture:

1. **WebexInputRouter** - Monitors Webex event stream for messages from authorized user, converts to `MspcMessage::UserInput`, sends to MSPC channel

2. **WebexOutputSink** - Receives AI responses and broadcasts them to Webex DM space

3. **Webex Client** - Uses the existing `webex` crate (https://crates.io/crates/webex) for API interaction

### Data Flow

```
Webex DM → WebexInputRouter → MSPC Channel → APChat Core → Response
                                                                ↓
Terminal ← Terminal Output ←────────────────────────────────────┤
Webex DM ← WebexOutputSink ←────────────────────────────────────┘
```

## CLI Integration

### New Flag

```rust
/// Enable Webex bot and specify authorized user email
#[arg(long, value_name = "USER_EMAIL")]
pub webex_bot: Option<String>,
```

**Usage:**
```bash
cargo run -- -i --webex-bot alice@example.com
```

### Secret Loading

Create `load_webex_secret()` utility function that:

1. Checks `WEBEX_APCHAT_SECRET` environment variable
2. If not found, reads `~/.okaychat/env` file:
   - Parse lines matching `WEBEX_APCHAT_SECRET=value`
   - Strip quotes if present
   - Ignore comment lines starting with `#`
3. Returns error with helpful message if not found

**Location:** `crates/apchat-webex/src/config.rs`

## Implementation Details

### WebexInputRouter

**File:** `crates/apchat-webex/src/input_router.rs`

```rust
pub struct WebexInputRouter {
    webex_client: Arc<Webex>,
    room_id: String,
    authorized_user_id: String,
    mspc_channel: Arc<MspcChannel>,
}

impl WebexInputRouter {
    pub async fn new(
        access_token: String,
        user_email: String,
        mspc_channel: Arc<MspcChannel>,
    ) -> Result<Self> {
        let webex_client = Webex::new(&access_token)?;

        // Get person ID from email
        let person_id = webex_client.get_person_by_email(&user_email).await?;

        // Get or create direct room
        let room_id = webex_client.get_or_create_direct_room(&person_id).await?;

        // Send startup message
        webex_client.send_message(&room_id, "APchat ready").await?;

        Ok(Self {
            webex_client: Arc::new(webex_client),
            room_id,
            authorized_user_id: person_id,
            mspc_channel,
        })
    }

    pub async fn run(&self) -> Result<()> {
        let mut event_stream = self.webex_client.event_stream().await?;

        while let Some(event) = event_stream.next().await {
            if let Event::Message(msg) = event {
                // Filter: only from authorized user, not from bot itself
                if msg.person_id == self.authorized_user_id
                    && msg.room_id == self.room_id {
                    let message = MspcMessage::UserInput(
                        msg.text,
                        Some(format!("webex:{}", self.authorized_user_id))
                    );
                    self.mspc_channel.send(message).await?;
                }
            }
        }

        Ok(())
    }
}
```

### WebexOutputSink

**File:** `crates/apchat-webex/src/output_sink.rs`

```rust
pub struct WebexOutputSink {
    webex_client: Arc<Webex>,
    room_id: String,
}

impl WebexOutputSink {
    pub fn new(webex_client: Arc<Webex>, room_id: String) -> Self {
        Self { webex_client, room_id }
    }

    pub async fn send_response(&self, text: &str) -> Result<()> {
        self.webex_client.send_message(&self.room_id, text).await?;
        Ok(())
    }
}
```

### REPL Integration

**File:** `apchat-main/src/app/repl.rs`

Add optional Webex sink parameter:

```rust
pub async fn run_repl_mode(
    chat: &mut APChat,
    mspc_channel: Arc<MspcChannel>,
    terminal_router: Arc<TerminalInputRouter>,
    webex_sink: Option<WebexOutputSink>,  // NEW
) -> Result<()>
```

When AI generates response, broadcast to both outputs:

```rust
// Print to terminal (existing)
println!("{}", response_text);

// Also send to Webex if enabled
if let Some(webex) = &webex_sink {
    if let Err(e) = webex.send_response(&response_text).await {
        eprintln!("⚠️ Failed to send to Webex: {}", e);
    }
}
```

### Main Application Flow

**File:** `apchat-main/src/main.rs`

When `--webex-bot <email>` is provided:

1. Load Webex secret via `load_webex_secret()`
2. Create `WebexInputRouter` and `WebexOutputSink`
3. Spawn WebexInputRouter as background task
4. Pass WebexOutputSink to REPL

```rust
// Spawn Webex input router as background task
if let Some(webex_router) = webex_router {
    tokio::spawn(async move {
        if let Err(e) = webex_router.run().await {
            eprintln!("Webex input router error: {}", e);
        }
    });
}
```

## Dependencies

**New crate:** `crates/apchat-webex/Cargo.toml`

```toml
[dependencies]
webex = "0.4"  # Check latest version on crates.io
tokio = { version = "1", features = ["full"] }
anyhow = "1.0"
apchat-mspc = { path = "../apchat-mspc" }
```

## Error Handling

- **Missing secret** → Clear error: "WEBEX_APCHAT_SECRET not found. Set via environment variable or add to ~/.okaychat/env"
- **Invalid email format** → Error: "Invalid Webex user email format"
- **Network errors** → Log warning, continue (don't crash REPL)
- **Webex send failures** → Log warning to terminal, don't block AI response

## Testing Strategy

1. **Unit tests** - Config loading, message filtering
2. **Integration test** - Mock Webex client, verify MSPC messages
3. **Manual test** - Real Webex account, verify DM interaction

## Implementation Phases

1. Create `crates/apchat-webex` with config loading
2. Implement `WebexInputRouter` with event stream monitoring
3. Implement `WebexOutputSink` for response broadcasting
4. Integrate into CLI and main application
5. Add startup message "APchat ready"
6. Test with real Webex account

## References

- [webex crate on crates.io](https://crates.io/crates/webex)
- [webex-rust GitHub repository](https://github.com/wr-org/webex-rust)
- [webex crate documentation](https://docs.rs/webex/latest/webex/)
