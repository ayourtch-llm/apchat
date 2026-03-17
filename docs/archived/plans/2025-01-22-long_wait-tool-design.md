# Long Wait Tool - Design Document

**Date:** 2025-01-22
**Status:** Design Approved
**Author:** AI Assistant (via brainstorming skill)

## Overview

The `long_wait` tool allows AI agents to pause execution during long reasoning chains when waiting for external events such as builds, deployments, or file transfers. The tool integrates with APChat's MSPC framework to broadcast progress updates to all subscribers and supports user interruption via `InterruptSignal`.

## Purpose

- Enable AI agents to wait for external events without manual intervention
- Provide visibility into wait progress across all output destinations
- Allow users to cancel long waits gracefully
- Prevent AI from making assumptions about operation completion

## Key Features

1. **Configurable Duration**: Up to 600 seconds (10 minutes)
2. **Descriptive Messaging**: Optional message explains why the wait is occurring
3. **Exponential Backoff Updates**: Progress updates at 1s, 2s, 4s, 8s, 16s... showing remaining time
4. **MSPC Integration**: All output destinations (terminal, web, logs) receive progress updates
5. **User-Cancellable**: Responds to `InterruptSignal` for graceful cancellation
6. **Clean Result Reporting**: Returns status on completion or cancellation (not treated as error)

## Tool Specification

### Signature

```rust
pub struct LongWaitTool;

impl Tool for LongWaitTool {
    fn name(&self) -> &str {
        "long_wait"
    }

    fn description(&self) -> &str {
        "Pause execution for a specified duration with progress updates. Use when waiting for external events like builds or deployments."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!(
                "duration_seconds",
                "number",
                "Duration to wait in seconds (max: 600)",
                required
            ),
            param!(
                "message",
                "string",
                "Optional description of why waiting is occurring (e.g., 'Waiting for build to complete')",
                optional
            ),
        ])
    }
}
```

### Example Usage

```xml
<long_wait>
  <duration_seconds>120</duration_seconds>
  <message>Waiting for Docker build to complete</message>
</long_wait>
```

## Architecture

### Execution Flow

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Parse & validate parameters                              │
│    - duration_seconds must be <= 600                        │
│    - message is optional                                    │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 2. Initialize timing state                                  │
│    - Record start time (Instant::now())                     │
│    - Set first update interval = 1 second                   │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 3. Main loop (100ms poll interval)                         │
│    ├─ Check if elapsed >= duration → return success        │
│    ├─ Check for InterruptSignal → return cancellation      │
│    ├─ If update due → send progress via MSPC               │
│    │   └─ Double interval (1→2→4→8→16→32→64)               │
│    └─ Sleep 100ms                                          │
└─────────────────────────────────────────────────────────────┘
```

### MSPC Integration

**Sending Progress Updates:**
```rust
MspcMessage::SystemMessage("⏳ Waiting for build - 117s remaining")
```

**Receiving Interrupts:**
```rust
// Non-blocking check
match context.mspc_receiver.try_recv() {
    Ok(MspcMessage::InterruptSignal(reason, sender)) => {
        // Handle cancellation
    }
    _ => continue,
}
```

### Update Schedule (Exponential Backoff)

| Elapsed | Update Interval | Next Update At | Message |
|---------|-----------------|----------------|---------|
| 0s | 1s | 1s | "⏳ Waiting... 120s remaining" |
| 1s | 2s | 3s | "⏳ Waiting... 119s remaining" |
| 3s | 4s | 7s | "⏳ Waiting... 117s remaining" |
| 7s | 8s | 15s | "⏳ Waiting... 113s remaining" |
| 15s | 16s | 31s | "⏳ Waiting... 105s remaining" |
| 31s | 32s | 63s | "⏳ Waiting... 89s remaining" |
| 63s | 64s | 127s | "⏳ Waiting... 57s remaining" |
| 127s | 64s | 191s | "⏳ Waiting... - (capped)" |

*Maximum update interval is capped at 64 seconds*

## Implementation Structure

### Files

```
crates/apchat-tools/src/
├── long_wait.rs          # NEW - LongWaitTool implementation
└── lib.rs               # UPDATED - Export the tool

crates/apchat-toolcore/src/
└── tool_context.rs       # UPDATED - Add MSPC channel fields

apchat-main/src/config/
└── mod.rs                # UPDATED - Register the tool
```

### ToolContext Changes

```rust
pub struct ToolContext {
    pub work_dir: PathBuf,
    pub mspc_sender: tokio::sync::mpsc::Sender<MspcMessage>,  // NEW
    pub mspc_receiver: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<MspcMessage>>>, // NEW
    // ... existing fields
}
```

### Registration

In `apchat-main/src/config/mod.rs`:
```rust
pub fn initialize_tool_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    // ... existing registrations

    // Register system tools
    registry.register_with_categories(RunCommandTool, vec!["system".to_string()]);
    registry.register_with_categories(LongWaitTool, vec!["system".to_string()); // NEW

    registry
}
```

## Error Handling

| Scenario | Handling | Return Value |
|----------|----------|--------------|
| Duration > 600s | Validate upfront | `ToolResult::error("Duration exceeds maximum of 600 seconds")` |
| Duration = 0 | Return immediately | `ToolResult::success("Wait completed: 0 seconds")` |
| Invalid duration (parse error) | Parameter parsing fails | `ToolResult::error("Failed to parse duration_seconds: ...")` |
| MSPC send failure | Log error, continue wait | Don't fail the wait |
| Interrupt signal | Graceful exit | `ToolResult::success("Wait cancelled after X seconds")` |

## Return Values

### Success Completion
```json
{
  "status": "success",
  "result": "Wait completed: 120 seconds"
}
```

### User Cancellation
```json
{
  "status": "success",
  "result": "Wait cancelled after 45 seconds: User interrupted"
}
```

### Error
```json
{
  "status": "error",
  "error": "Duration exceeds maximum of 600 seconds"
}
```

## Testing Strategy

### Unit Tests

1. **Duration Validation**
   - Test duration = 0 (immediate completion)
   - Test normal duration (e.g., 5 seconds)
   - Test duration > MAX (should error)

2. **Update Interval Progression**
   - Verify intervals: 1 → 2 → 4 → 8 → 16 → 32 → 64 (capped)
   - Verify messages sent at correct times

3. **Cancellation Handling**
   - Send `InterruptSignal` during wait
   - Verify cancellation message returned
   - Verify elapsed time reported correctly

4. **Message Formatting**
   - Test with custom message
   - Test without custom message
   - Verify emoji and remaining time included

5. **Edge Cases**
   - Negative duration (parse error)
   - Non-numeric duration (parse error)
   - Very long duration (600s)

### Integration Tests

1. **MSPC Broadcasting**
   - Verify progress updates reach all subscribers
   - Test with terminal + web UI active

2. **ToolContext Integration**
   - Verify MSPC sender/receiver accessible from tool
   - Test with concurrent tools running

## Constants

```rust
const MAX_DURATION: u64 = 600;          // 10 minutes
const MIN_POLL_INTERVAL_MS: u64 = 100;  // Responsive interrupt checking
const MAX_UPDATE_INTERVAL: u64 = 64;    // Cap exponential backoff
```

## Future Enhancements (Out of Scope)

- Configurable maximum duration (via settings)
- Progress callback for custom UI rendering
- Wait-with-condition (poll until file exists/command succeeds)
- Pause/resume capability
- Multiple concurrent waits with progress bars

## Rationale

### Why Exponential Backoff?
- Reduces message spam for long waits
- Frequent updates early (when things are changing fast)
- Less frequent updates later (when status is stable)
- Natural fit for human attention patterns

### Why MSPC Integration?
- Consistent with existing architecture
- All subscribers get updates (terminal, web, logs)
- Interrupts already use `InterruptSignal`
- No new channels or mechanisms needed

### Why Success on Cancellation?
- Cancellation is a user choice, not a failure
- AI can decide what to do next (retry, check status, continue)
- Prevents AI from treating user interrupts as errors to retry
