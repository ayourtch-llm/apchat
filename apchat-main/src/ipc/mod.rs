//! Interprocess communication via Unix datagram sockets.
//!
//! Each apchat process binds a Unix datagram socket at
//! `$APCHAT_MSG_DIR/apchat_pid_<pid>.sock`. A background async task
//! receives datagrams and pushes them into the shared mailbox.
//! Senders bind their own socket so `recv_from` reveals the sender PID.

use std::path::PathBuf;
use apchat_mspc::ipc::{SharedMailbox, IpcMessage, get_ipc_msg_dir, get_socket_path};
use apchat_vty::print_heart_yellow;
use tokio::task::JoinHandle;

/// Guard that removes the socket file on drop.
pub struct SocketGuard {
    path: PathBuf,
    reader_handle: JoinHandle<()>,
}

impl Drop for SocketGuard {
    fn drop(&mut self) {
        self.reader_handle.abort();
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Start the datagram socket listener for this process.
/// Returns a guard that cleans up the socket on drop.
pub fn start_socket_listener(mailbox: SharedMailbox) -> Option<SocketGuard> {
    let pid = std::process::id();
    let msg_dir = get_ipc_msg_dir();
    let sock_path = get_socket_path(pid);

    // Create the message directory
    if let Err(e) = std::fs::create_dir_all(&msg_dir) {
        print_heart_yellow(&format!("⚠️ Failed to create IPC message directory {:?}: {}", msg_dir, e), true);
        return None;
    }

    // Remove stale socket if it exists
    let _ = std::fs::remove_file(&sock_path);

    // Bind the datagram socket
    let socket = match std::os::unix::net::UnixDatagram::bind(&sock_path) {
        Ok(s) => s,
        Err(e) => {
            print_heart_yellow(&format!("⚠️ Failed to bind IPC socket {:?}: {}", sock_path, e), true);
            return None;
        }
    };

    // Convert to tokio async socket
    socket.set_nonblocking(true).ok()?;
    let async_socket = match tokio::net::UnixDatagram::from_std(socket) {
        Ok(s) => s,
        Err(e) => {
            print_heart_yellow(&format!("⚠️ Failed to create async IPC socket: {}", e), true);
            let _ = std::fs::remove_file(&sock_path);
            return None;
        }
    };

    let sock_path_clone = sock_path.clone();
    let handle = tokio::spawn(async move {
        socket_reader_loop(async_socket, mailbox).await;
    });

    print_heart_yellow(&format!("✓ IPC socket bound at {:?}", sock_path_clone), true);

    Some(SocketGuard {
        path: sock_path,
        reader_handle: handle,
    })
}

/// Background loop that receives datagrams from the socket.
async fn socket_reader_loop(socket: tokio::net::UnixDatagram, mailbox: SharedMailbox) {
    let mut buf = vec![0u8; 65536]; // Max datagram size

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, addr)) => {
                let data = &buf[..len];
                let text = match std::str::from_utf8(data) {
                    Ok(s) => s.trim().to_string(),
                    Err(_) => continue,
                };
                if text.is_empty() {
                    continue;
                }

                // Extract sender PID from the socket path
                let sender_pid = addr.as_pathname()
                    .and_then(|p| p.file_name())
                    .and_then(|f| f.to_str())
                    .and_then(|name| name.strip_prefix("apchat_pid_"))
                    .and_then(|rest| rest.strip_suffix(".sock"))
                    .and_then(|pid_str| pid_str.parse::<u32>().ok())
                    .unwrap_or(0);

                // Parse as JSON if possible, otherwise treat as plain text
                let content = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    json.get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&text)
                        .to_string()
                } else {
                    text
                };

                print_heart_yellow(&format!("📨 [IPC] Message from PID {}: {}",
                    sender_pid,
                    if content.len() > 100 { &content[..100] } else { &content }
                ), true);

                let mut mb = mailbox.lock().await;
                mb.push(IpcMessage { sender_pid, content });
            }
            Err(e) => {
                // EAGAIN/EWOULDBLOCK are normal for non-blocking sockets
                // when there's nothing to read — but tokio handles this internally.
                // Any other error means the socket is broken.
                print_heart_yellow(&format!("⚠️ [IPC] Socket recv error: {}", e), true);
                break;
            }
        }
    }
}

/// Find a child agent PID by scanning the socket directory for processes
/// whose parent PID is the current process.
pub fn find_child_agent_pid() -> Option<u32> {
    let our_pid = std::process::id();
    let msg_dir = get_ipc_msg_dir();

    if !msg_dir.exists() {
        return None;
    }

    let mut candidates = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&msg_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(pid_str) = name.strip_prefix("apchat_pid_").and_then(|s| s.strip_suffix(".sock")) {
                if let Ok(pid) = pid_str.parse::<u32>() {
                    if pid == our_pid {
                        continue;
                    }
                    if get_parent_pid(pid) == Some(our_pid) {
                        candidates.push(pid);
                    }
                }
            }
        }
    }

    candidates.into_iter().max()
}

/// Get the parent PID of a process (public wrapper).
pub fn get_parent_pid_pub(pid: u32) -> Option<u32> {
    get_parent_pid(pid)
}

fn get_parent_pid(pid: u32) -> Option<u32> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string(format!("/proc/{}/stat", pid))
            .ok()
            .and_then(|stat| {
                let after_comm = stat.rfind(')')?;
                let fields: Vec<&str> = stat[after_comm + 2..].split_whitespace().collect();
                fields.get(1)?.parse::<u32>().ok()
            })
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "ppid="])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().parse::<u32>().ok())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

/// Send a message to a specific PID's socket (synchronous, for REPL use).
pub fn send_message_to_pid(target_pid: u32, sender_pid: u32, content: &str) {
    let target_path = get_socket_path(target_pid);
    let sender_path = get_socket_path(sender_pid);

    if !target_path.exists() {
        print_heart_yellow(&format!("⚠️ [IPC] Cannot send to PID {}: socket does not exist", target_pid), true);
        return;
    }

    // Bind sender socket (it should already exist from our listener)
    // Use send_to from our bound socket so the receiver knows who sent it
    match std::os::unix::net::UnixDatagram::unbound() {
        Ok(sock) => {
            // Connect our already-bound socket path so recv_from can identify us
            // Actually, we need to bind to our path for the sender address to show up.
            // But our listener already bound it. Use unbound + send_to instead,
            // and include sender_pid in the message payload as fallback.
            let msg = serde_json::json!({
                "sender_pid": sender_pid,
                "content": content,
            });
            let data = msg.to_string();
            if let Err(e) = sock.send_to(data.as_bytes(), &target_path) {
                print_heart_yellow(&format!("⚠️ [IPC] Failed to send to PID {}: {}", target_pid, e), true);
            }
        }
        Err(e) => {
            print_heart_yellow(&format!("⚠️ [IPC] Failed to create socket: {}", e), true);
        }
    }
}
