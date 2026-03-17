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

/// Get the metadata file path for a given PID.
pub fn get_meta_path(pid: u32) -> std::path::PathBuf {
    get_ipc_msg_dir().join(format!("apchat_pid_{}.meta", pid))
}

/// Agent metadata stored alongside the socket.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct AgentMeta {
    pub pid: u32,
    #[serde(default)]
    pub parent_pid: u32,
    #[serde(default)]
    pub work_dir: String,
    #[serde(default)]
    pub title: String,
}

/// Write agent metadata to the meta file.
pub fn write_agent_meta(meta: &AgentMeta) {
    let path = get_meta_path(meta.pid);
    if let Ok(json) = serde_json::to_string(meta) {
        let _ = std::fs::write(path, json);
    }
}

/// Read agent metadata for a given PID. Returns None if file doesn't exist or can't be parsed.
pub fn read_agent_meta(pid: u32) -> Option<AgentMeta> {
    let path = get_meta_path(pid);
    std::fs::read_to_string(path).ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

/// Update just the title in the agent's metadata file.
pub fn set_agent_title(pid: u32, title: &str) {
    let mut meta = read_agent_meta(pid).unwrap_or(AgentMeta {
        pid,
        ..Default::default()
    });
    meta.title = title.to_string();
    write_agent_meta(&meta);
}

/// Type alias for the shared mailbox.
pub type SharedMailbox = Arc<Mutex<InterprocessMailbox>>;

/// Create a new shared mailbox.
pub fn new_shared_mailbox() -> SharedMailbox {
    Arc::new(Mutex::new(InterprocessMailbox::new()))
}
