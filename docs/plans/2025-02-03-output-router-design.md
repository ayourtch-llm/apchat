# OutputRouter Design Document

## Overview

The OutputRouter system provides a flexible mechanism to route emoji-prefixed text output from `print_with_emoji` to multiple destinations (readline, terminal, file logger, websocket, etc.) via a broadcasting architecture.

## Problem Statement

Currently, `print_with_emoji`, `print_heart_red`, and `print_heart_yellow` write directly to stdout/stderr. This makes it impossible to:
- Send output to the readline display integrated with the input line
- Log output to files
- Send output to web UI via websocket

## Solution

Create a message-based routing system that:
1. Defines a `TextOutput` message containing emoji, content, and newline flag
2. Extends `MspcMessage::EmojiText` for readline integration
3. Uses an `OutputRouter` to broadcast messages to registered destinations
4. Maintains backward compatibility by keeping direct writes to provided writers

## Architecture

### Component: TextOutput (Message)

```rust
pub struct TextOutput {
    pub emoji: String,
    pub content: String,
    pub newline: bool,
}
```

Simple, serializable message capturing what `print_with_emoji` prints.

### Component: OutputDestination Trait

```rust
#[async_trait]
pub trait OutputDestination: Send + Sync {
    async fn send_output(&self, msg: &OutputMessage) -> Result<(), Box<dyn StdError + Send>>;
    fn dest_id(&self) -> String;
    fn is_active(&self) -> bool;
}
```

All output destinations implement this trait.

### Component: OutputRouter

```rust
pub struct OutputRouter {
    destinations: Arc<RwLock<Vec<Arc<dyn OutputDestination>>>>,
}

impl OutputRouter {
    pub fn new() -> Self;
    pub fn start_monitoring(&self); // Spawns broadcast receiver task
    pub async fn register(&self, dest: Arc<dyn OutputDestination>);
    pub async fn unregister(&self, id: String) -> bool;
    pub async fn active_count(&self) -> usize;
}
```

Central coordinator managing all output destinations.

### Component: Global Broadcast Channel

```rust
use tokio::sync::broadcast::Sender;

pub static TEXT_OUTPUT_TX: Lazy<Sender<TextOutput>> =
    Lazy::new(|| broadcast::channel(100).0);
```

Synchronous channel allows `print_with_emoji` to send messages without becoming async.

## Message Flows

### TextOutput Message Flow

1. Code calls `print_with_emoji("❤️", "Hello world", true)`
2. `print_with_emoji` creates `TextOutput` and sends to `TEXT_OUTPUT_TX`
3. Background monitoring task receives from broadcast channel
4. Monitoring task iterates through registered destinations
5. Each destination's `send_output()` is called with `OutputMessage::EmojiText`
6. Destination formats and delivers output according to its mechanism

### Internal flow in print_with_emoji

```rust
fn print_with_emoji(emoji: &str, text: &str, newline: bool, mut writer: impl io::Write) {
    let output = TextOutput {
        emoji: emoji.to_string(),
        content: text.to_string(),
        newline,
    };

    // Send to router (non-blocking)
    let _ = TEXT_OUTPUT_TX.send(output);

    // Also write directly to provided writer (backward compatibility)
    // ... existing code ...
}
```

## Destination Implementations

### ReadlineDestination

Routes to the existing mpsc channel used by the readline instance.

```rust
pub struct ReadlineDestination {
    sender: tokio_mpsc::Sender<MspcMessage>,
    active: Arc<AtomicBool>,
}
```

Sends `MspcMessage::EmojiText { emoji, content, newline }`.

**Readline Integration:**

The readline poll loop handles `EmojiText` specially:
1. Saves cursor position (user's input line)
2. Clears current line
3. Prints emoji text below the input
4. Restores cursor to input position
5. Continues event loop (doesn't return)

This creates the "scroll up" effect where emoji text appears without disrupting user input.

### TerminalDestination

Direct write to stdout/stderr using existing `print_heart_red`/`print_heart_yellow`.

```rust
pub struct TerminalDestination {
    active: Arc<AtomicBool>,
}
```

### FileDestination

Appends formatted output to a log file with timestamps.

```rust
pub struct FileDestination {
    file_path: PathBuf,
    active: Arc<AtomicBool>,
}
```

### WsDestination (Future)

Sends JSON-formatted output to WebSocket clients for web UI.

## Extended MspcMessage Enum

```rust
pub enum MspcMessage {
    // ... existing variants ...
    UserInput(String, Option<String>),
    SystemPrompt(String, Option<String>),
    ConfirmationRequest(String, Option<String>),
    ConfirmationResponse(bool, Option<String>),
    ToolConfirmationRequest { content: String, confirmation_id: String },
    ToolConfirmationResponse { approved: bool, reason: Option<String>, confirmation_id: String },
    InterruptSignal(String, Option<String>),
    Command(String, Option<String>),
    ToolResult(String, Option<String>),
    Error(String, Option<String>),

    // NEW: Emoji-prefixed text for display in readline
    EmojiText {
        emoji: String,
        content: String,
        newline: bool,
    },
}
```

## Data Types

### OutputMessage Enum

Extended to include `EmojiText` variant:

```rust
pub enum OutputMessage {
    UserMessage { sender: String, text: String },
    AssistantResponse(String),
    ToolCall { name: String, args: Value },
    ToolResult(String),
    SystemMessage(String),
    Error(String),
    TextOutput { emoji: String, content: String, newline: bool },
}
```

## Initialization

The OutputRouter is initialized early in the application:

```rust
pub async fn initialize_output_router(
    mspc_sender: tokio_mpsc::Sender<MspcMessage>,
) -> Arc<OutputRouter> {
    let router = Arc::new(OutputRouter::new());

    // Register destinations
    let readline_dest = Arc::new(ReadlineDestination::new(mspc_sender));
    router.register(readline_dest).await;

    let terminal_dest: Arc<dyn OutputDestination> = Arc::new(TerminalDestination::new());
    router.register(terminal_dest).await;

    // Start background monitoring
    router.start_monitoring();

    router
}
```

## Backward Compatibility

- `print_with_emoji` continues writing directly to the provided `writer`
- Existing callers continue to work without modification
- Routing is additive, not replacement

## Error Handling

- Channel full: Non-blocking `.send()` drops message silently (acceptable for non-critical output)
- Destination inactive: `send_output()` returns error, logged by monitoring task
- Destination failure: Logged, doesn't stop other destinations from receiving

## Future Extensions

1. **Filtering**: Add message filtering based on emoji or content patterns
2. **Formatting**: Per-destination formatting options
3. **Queueing**: Optional buffering for destinations that may be temporarily unavailable
4. **Metrics**: Track message counts, errors per destination

## Files to Modify

1. `crates/apchat-mspc/src/channel.rs` - Add `EmojiText` variant to `MspcMessage`
2. `apchat-main/src/mspc/output.rs` - Add `TextOutput` struct and extend `OutputMessage`
3. `apchat-main/src/mspc/mod.rs` - Add `TEXT_OUTPUT_TX` static
4. `apchat-main/src/mspc/router.rs` (new) - Implement `OutputRouter`
5. `apchat-main/src/mspc/destinations.rs` (new) - Implement destination types
6. `crates/apchat-vty/src/readline.rs` - Handle `EmojiText` in poll loop
7. `crates/apchat-vty/src/lib.rs` - Update `print_with_emoji` to send to router
8. `apchat-main/src/main.rs` - Initialize OutputRouter on startup

## Testing Strategy

1. Unit tests for each destination implementation
2. Integration test verifying messages reach all destinations
3. Readline integration test verifying cursor save/restore
4. Backward compatibility test verifying direct writes still work
5. Error handling test for closed channels/inactive destinations
