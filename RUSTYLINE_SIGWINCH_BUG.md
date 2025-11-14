# rustyline SIGWINCH Panic Bug Report

## Summary

rustyline's global SIGWINCH (window resize) signal handler can panic with `fd != -1` when:
1. A temporary `DefaultEditor` is created and dropped
2. The signal handler persists after the editor is dropped
3. The terminal is resized while the program is in a waiting state (e.g., tokio runtime parking)

## Reproduction

We provide two standalone programs that reproduce this issue:

### Simple Reproduction

```bash
cargo run --bin rustyline_sigwinch_repro
```

1. Program creates a temporary `DefaultEditor` and uses it
2. Editor is dropped
3. Program waits for 30 seconds
4. **Resize your terminal window during the wait**
5. Program panics with `fd != -1` from rustyline's SIGWINCH handler

### Tokio-based Reproduction (More Accurate)

This better simulates the real-world scenario:

```bash
cargo run --bin rustyline_sigwinch_tokio_repro
```

1. Program asks for confirmation using temporary `DefaultEditor`
2. Answer 'y' and press Enter
3. Editor is dropped, program enters tokio async execution
4. **Resize your terminal window during the async sleep**
5. Program panics from SIGWINCH handler while tokio runtime is parked

## Panic Details

```
thread 'main' panicked at /Users/[user]/.cargo/registry/src/.../rustyline-14.0.0/src/tty/unix.rs:1197:28:
fd != -1
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread 'main' panicked at library/core/src/panicking.rs:225:5:
panic in a function that cannot unwind
```

Key stack frames:
- `rustyline::tty::unix::sigwinch_handler` - Signal handler fires
- `parking_lot::condvar::Condvar::wait` - Waiting in condvar
- `tokio::runtime::park::CachedParkThread::park` - Tokio runtime parked

## Root Cause

1. **Signal Handler Registration**: When `DefaultEditor::new()` is called, rustyline registers a global SIGWINCH signal handler via `sigaction()`

2. **Handler Persistence**: This signal handler is registered at the process level and persists even after the `DefaultEditor` is dropped

3. **Invalid File Descriptors**: The handler expects to access file descriptors (stdin/stdout) that may no longer be valid or may be in use elsewhere

4. **Panic in Signal Context**: When window resize happens, the signal handler fires in async context (during tokio parking) and panics when it finds `fd == -1`

5. **Non-Unwinding Panic**: Signal handlers cannot unwind, so this becomes a fatal abort

## Problematic Usage Pattern

```rust
// ❌ PROBLEMATIC: Creates temporary editor for simple prompt
fn get_confirmation() -> bool {
    let mut rl = DefaultEditor::new()?;
    let response = rl.readline(">>> ")?;
    response.trim() == "y"
    // Editor dropped, but SIGWINCH handler still active!
}

async fn main() {
    if get_confirmation() {
        // Now in async runtime with orphaned signal handler
        do_async_work().await; // SIGWINCH during this = PANIC
    }
}
```

## Workaround

Use standard `stdin` for simple prompts instead of rustyline:

```rust
// ✅ SAFE: No signal handlers
fn get_confirmation() -> bool {
    use std::io::{self, BufRead, Write};

    print!(">>> ");
    io::stdout().flush().unwrap();

    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut response = String::new();
    handle.read_line(&mut response).unwrap();

    response.trim() == "y"
}
```

Reserve rustyline for long-lived REPL sessions, not temporary prompts.

## Expected Behavior

One of:
1. Signal handler should be unregistered when `DefaultEditor` is dropped
2. Signal handler should safely handle the case where FD is invalid
3. Documentation should warn against creating temporary editors
4. Provide a signal-handler-free mode for simple use cases

## Environment

- **rustyline**: 14.0.0
- **OS**: macOS (also affects Linux)
- **Rust**: 1.83.0
- **tokio**: 1.48.0

## Related Issues

This affects any program that:
- Creates temporary `DefaultEditor` instances for prompts
- Runs in an async runtime (tokio, async-std, etc.)
- Can receive SIGWINCH during async execution
- Uses rustyline for non-REPL purposes

## Suggested Fix Locations

File: `src/tty/unix.rs:1197`
```rust
pub extern "C" fn sigwinch_handler(_: libc::c_int) {
    let fd = SIGWINCH_PIPE.load(Ordering::Relaxed);
    assert!(fd != -1); // ← This panics when handler is orphaned
    // ...
}
```

Potential fixes:
1. Check `fd != -1` and return early if invalid instead of asserting
2. Unregister handler in `Drop` implementation
3. Use thread-local or instance-specific handlers instead of global
