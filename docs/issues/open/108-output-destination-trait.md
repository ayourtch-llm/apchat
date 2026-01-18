# Issue 108: Implement OutputDestination Trait

## Summary
Implement the OutputDestination trait that will serve as the foundation for all output destinations in the MSPC architecture.

## Location
- File: `src/mspc/output.rs` (new file)

## Current Behavior
No OutputDestination trait exists, making it impossible to implement the output abstraction layer needed for multi-source input.

## Expected Behavior
The OutputDestination trait and supporting types should be defined:
1. `OutputDestination` trait with required methods
2. `OutputMessage` enum for message types
3. Basic implementation infrastructure

## Impact
Without this trait, we cannot implement TerminalOutputDestination, WebSocketOutputDestination, or TuiOutputDestination. This is a critical missing piece for Phase 1.

## Suggested Implementation

### Step 1: Create new file
```bash
touch src/mspc/output.rs
```

### Step 2: Implement OutputDestination trait
```rust
use async_trait::async_trait;
use serde_json::Value;

/// Enum representing different types of output messages
#[derive(Debug, Clone)]
pub enum OutputMessage {
    UserMessage { sender: String, text: String },
    AssistantResponse(String),
    ToolCall { name: String, args: Value },
    ToolResult(String),
    SystemMessage(String),
    Error(String),
}

/// Trait for output destinations
#[async_trait]
pub trait OutputDestination: Send + Sync {
    /// Send an output message to this destination
    async fn send_output(&self, message: &OutputMessage) -> Result<(), Box<dyn std::error::Error + Send>>;

    /// Unique identifier for this destination
    fn dest_id(&self) -> String;

    /// Check if the destination is active/connected
    fn is_active(&self) -> bool;
}

/// Broadcast a message to all destinations
pub async fn broadcast_to_all(
    destinations: &[Box<dyn OutputDestination>],
    message: OutputMessage,
) {
    for dest in destinations {
        if dest.is_active() {
            if let Err(e) = dest.send_output(&message).await {
                eprintln!("Failed to send to {}: {}", dest.dest_id(), e);
            }
        }
    }
}
```

### Step 3: Add module exports
```rust
// In src/mspc/mod.rs or lib.rs

pub mod output;
pub use output::{OutputDestination, OutputMessage, broadcast_to_all};
```

## Resolution

This will provide the foundation for all output destinations in the system.

**Files Created:**
- `src/mspc/output.rs`

**Files Modified:**
- `src/mspc/mod.rs` or appropriate module file

**Testing:**
- Verify trait can be implemented
- Test broadcast_to_all with mock destinations
- Test OutputMessage serialization

---
*Created: 2026-01-18*
