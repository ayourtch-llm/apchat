//! PTY allocation helper for tests that require a TTY on stdin.
//!
//! When running under `cargo test`, stdin is not a terminal, so `tcgetattr(STDIN_FILENO)`
//! fails and `Readline::new()` cannot enable raw mode. This module allocates a PTY pair
//! and redirects stdin to the slave side, making terminal operations succeed.
//!
//! Usage: call `ensure_pty_stdin()` at the top of any test (or test module) that needs
//! a real TTY. The setup is idempotent — it only runs once per process.

use std::sync::Once;

static PTY_INIT: Once = Once::new();
/// Saved original stdin fd (so we could restore if needed)
static mut ORIGINAL_STDIN: i32 = -1;
/// Master side of PTY (kept alive so slave stays valid)
static mut PTY_MASTER: i32 = -1;

/// Ensures stdin is backed by a PTY slave, so `tcgetattr(STDIN_FILENO)` succeeds.
///
/// This is safe to call multiple times — only the first call has any effect.
/// The PTY master fd is kept alive for the lifetime of the process.
pub fn ensure_pty_stdin() {
    PTY_INIT.call_once(|| {
        unsafe {
            let mut master: libc::c_int = 0;
            let mut slave: libc::c_int = 0;

            let ret = libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            if ret != 0 {
                panic!("openpty() failed: {}", std::io::Error::last_os_error());
            }

            // Save original stdin so we don't leak it
            ORIGINAL_STDIN = libc::dup(libc::STDIN_FILENO);

            // Redirect stdin to the PTY slave
            if libc::dup2(slave, libc::STDIN_FILENO) == -1 {
                panic!("dup2(slave, STDIN_FILENO) failed: {}", std::io::Error::last_os_error());
            }

            // Close the slave fd (stdin now holds a reference to it)
            libc::close(slave);

            // Keep master alive — store it globally
            PTY_MASTER = master;
        }
    });
}
