//! Interprocess communication types for agent-to-agent messaging.
//!
//! Each apchat process creates a Unix datagram socket at
//! `$APCHAT_MSG_DIR/apchat_pid_<pid>.sock`. Messages are sent as
//! self-contained JSON datagrams.

use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

/// A message received from another apchat process.
#[derive(Debug, Clone)]
pub struct IpcMessage {
    pub sender_pid: u32,
    pub content: String,
}

/// Shared mailbox for interprocess messages.
pub struct InterprocessMailbox {
    pub messages: Vec<IpcMessage>,
    pub notify: Arc<Notify>,
}

impl InterprocessMailbox {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Push a message and notify any waiters (e.g., long_wait tool).
    pub fn push(&mut self, msg: IpcMessage) {
        self.messages.push(msg);
        self.notify.notify_waiters();
    }

    /// Drain all pending messages.
    pub fn drain(&mut self) -> Vec<IpcMessage> {
        self.messages.drain(..).collect()
    }
}

/// Get the IPC message directory path.
pub fn get_ipc_msg_dir() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("APCHAT_MSG_DIR") {
        std::path::PathBuf::from(dir)
    } else {
        std::path::PathBuf::from("/tmp/apchat-msg")
    }
}

/// Get the socket path for a given PID.
pub fn get_socket_path(pid: u32) -> std::path::PathBuf {
    get_ipc_msg_dir().join(format!("apchat_pid_{}.sock", pid))
}

/// Type alias for the shared mailbox.
pub type SharedMailbox = Arc<Mutex<InterprocessMailbox>>;

/// Create a new shared mailbox.
pub fn new_shared_mailbox() -> SharedMailbox {
    Arc::new(Mutex::new(InterprocessMailbox::new()))
}
