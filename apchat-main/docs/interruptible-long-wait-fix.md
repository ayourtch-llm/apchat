# Fix for Interruptible Long Wait Tool

## Problem

The `long_wait` tool was not working correctly because it was trying to receive interrupt signals via the `mspc_receiver` channel, but that channel was being consumed by `mspc_session.rs`. This created a race condition where tools could never receive interrupt signals.

## Solution

Created a separate **interrupt channel** that allows tools to receive interrupt signals directly without competing with the main MSPC consumer.

### Changes Made

#### 1. ToolContext (`crates/apchat-toolcore/src/tool_context.rs`)

- Changed `signal_receiver` field type from `Option<Receiver<MspcMessage>>` to `Option<Arc<Mutex<Receiver<MspcMessage>>>>`
  - This matches the pattern used by `mspc_receiver` and allows the receiver to be shared via `Arc`
- Updated `with_signal_receiver()` to accept `Arc<Mutex<Receiver<MspcMessage>>>`

#### 2. LongWait Tool (`crates/apchat-tools/src/long_wait.rs`)

- Changed `check_for_interrupts()` to use `signal_receiver` instead of `mspc_receiver`
- Updated docstring to reflect the correct channel

#### 3. APChat (`apchat-main/src/apchat.rs`)

- Added `signal_receiver` field: `Option<Arc<Mutex<Receiver<MspcMessage>>>>`
- Added `with_signal_receiver()` builder method
- Updated tool context building to pass `signal_receiver` to tools

#### 4. REPL (`apchat-main/src/app/repl.rs`)

- Created a new interrupt channel: `(interrupt_sender, interrupt_receiver)`
- Wrapped `interrupt_receiver` in `Arc<Mutex<>>` before passing to `APChat`
- Forwarded interrupt signals from the main MSPC channel to the interrupt channel
  - When an `InterruptSignal` is received on the MSPC channel, it's now also sent to the interrupt channel
  - This ensures tools receive interrupt signals even when they're executing

### Architecture

```
User presses Ctrl+C or sends !cancel
        |
        v
TerminalInputRouter sends InterruptSignal to MSPC channel
        |
        v
Main REPL receives InterruptSignal from MSPC channel
        |
        +--> Forwards to interrupt_sender channel
        |
        v
Tool receives interrupt via signal_receiver (from interrupt channel)
        |
        v
Tool detects interrupt and exits gracefully
```

### Why This Works

1. **Separation of Concerns**: The main MSPC channel is used for message routing and session management, while the interrupt channel is specifically for tool interrupts.

2. **No Race Conditions**: Tools have their own dedicated channel for receiving interrupts, so they don't compete with `mspc_session.rs` for messages.

3. **Simple Forwarding**: The main loop simply forwards any interrupt signals it receives to the tool interrupt channel, maintaining a clean separation.

### Usage Example

When a tool like `long_wait` is running:

1. Tool calls `check_for_interrupts()` periodically
2. If the user sends an interrupt (Ctrl+C or `!cancel`), the signal is forwarded to the tool's interrupt channel
3. The tool detects the interrupt and returns an error: `"Wait interrupted after X seconds (Y% complete)"`

### Testing

To test interruptible tools:

1. Start the apchat REPL
2. Ask the AI to run `long_wait` for a long duration (e.g., 60 seconds)
3. Press Ctrl+C or type `!cancel` in a separate terminal
4. The tool should detect the interrupt and exit with a message

## Files Modified

1. `crates/apchat-toolcore/src/tool_context.rs`
2. `crates/apchat-tools/src/long_wait.rs`
3. `apchat-main/src/apchat.rs`
4. `apchat-main/src/app/repl.rs`
