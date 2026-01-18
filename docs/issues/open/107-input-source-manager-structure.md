# Issue 107: Create InputSourceManager Structure

## Summary
Create the InputSourceManager struct and basic implementation for managing multiple input sources.

## Location
- File: `src/input_router/manager.rs` (new file)

## Current Behavior
No central InputSourceManager exists to coordinate multiple input readers.

## Expected Behavior
- InputSourceManager struct defined
- Fields for tracking reader tasks
- Basic new() and cleanup() methods

## Impact
Without this, we cannot manage multiple input sources in a coordinated way.

## Suggested Implementation

### Step 1: Create new file
```bash
mkdir -p src/input_router
touch src/input_router/manager.rs
```

### Step 2: Add basic structure
```rust
// src/input_router/manager.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::sync::mpsc;

use crate::mspc::{MSPCMessage, MSPCChannel};

pub struct InputSourceManager {
    terminal_reader: Option<JoinHandle<()>>,
    webex_reader: Option<JoinHandle<()>>,
    websocket_handlers: HashMap<String, JoinHandle<()>>,
}

impl InputSourceManager {
    pub fn new() -> Self {
        Self {
            terminal_reader: None,
            webex_reader: None,
            websocket_handlers: HashMap::new(),
        }
    }

    pub async fn cleanup(&mut self) {
        if let Some(handle) = self.terminal_reader.take() {
            handle.abort();
            let _ = handle.await;
        }

        if let Some(handle) = self.webex_reader.take() {
            handle.abort();
            let _ = handle.await;
        }

        for (_, handle) in self.websocket_handlers.drain() {
            handle.abort();
            let _ = handle.await;
        }
    }
}
```

### Step 3: Add to module tree
```rust
// src/input_router/mod.rs or lib.rs

pub mod manager;
pub use manager::InputSourceManager;
```

## Resolution

This will provide the foundation for managing multiple input sources.

**Files Created:**
- `src/input_router/manager.rs`
- `src/input_router/mod.rs` (if needed)

**Testing:**
- Create instance and verify structure
- Test cleanup without readers

---
*Created: 2026-01-18*
