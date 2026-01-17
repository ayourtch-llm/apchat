# REPL Input Channel Refactoring Plan

## Goal
Refactor `run_repl_mode` function to use an input channel with a separate terminal listener task, allowing non-blocking input checking.

## Architecture
- Create an `InputChannel<Result<String, ReadlineError>>` for receiving terminal input
- Spawn a separate async task that continuously listens for terminal input
- Modify the main loop to check for pending input using `has_pending_messages()` and `try_recv()`
- Maintain backward compatibility with existing readline behavior

## Implementation Steps

### Task 1: Create Terminal Listener Task

Create a new async function that:
1. Takes a sender (mpsc::Sender) and the rustyline editor
2. Continuously reads input in a loop
3. Sends each result to the channel
4. Handles errors and closure gracefully

```rust
async fn terminal_listener(
    mut rl: DefaultEditor,
    sender: mpsc::Sender<Result<String, ReadlineError>>,
    prompt: String,
) -> Result<()> {
    loop {
        let line = rl.readline(&prompt);
        if let Err(ReadlineError::Eof) = line {
            break;  // Exit on Ctrl+D
        }
        // Send result to channel
        sender.send(line).await.map_err(|_| anyhow::anyhow!("Channel closed"))?;
    }
    Ok(())
}
```

### Task 2: Initialize InputChannel in run_repl_mode

Add channel initialization after editor creation:

```rust
// Initialize input channel
let input_config = InputChannelConfig::default();
let mut input_channel = InputChannel::new(input_config);
let sender = input_channel.sender().clone();  // Need to add sender() method or get sender

// Spawn terminal listener
let prompt = format!("{} {} ", 
    format!("[{}] ({})", chat.current_model.display_name(), model_name).bright_magenta(),
    "You:".bright_green().bold()
);
tokio::spawn(async move {
    terminal_listener(rl, sender, prompt).await
});
```

### Task 3: Refactor Main Loop

Replace the current readline call with channel checking:

```rust
loop {
    let model_name = get_model_name_for_prompt(&chat.current_model, &chat.client_config);
    
    // Check for pending input first
    let line_result = if input_channel.has_pending_messages().await {
        input_channel.try_recv().await
    } else {
        // No pending input, continue with other operations
        // But we still need the prompt for the listener
        tokio::task::yield_now().await;
        None
    };
    
    match line_result {
        Some(Ok(line)) => {
            let line = line.trim();
            // Process input...
        }
        Some(Err(e)) => {
            // Handle readline errors
        }
        None => {
            // No input available, check for timeout or other conditions
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }
    }
}
```

### Task 4: Handle Edge Cases

1. **Channel closure**: Handle when terminal listener exits
2. **Timeout support**: Integrate with existing idle_timeout logic
3. **Prompt updates**: Ensure prompt stays current with model changes
4. **History management**: Maintain readline history functionality

### Task 5: Testing

Create tests to verify:
1. Input is received through channel
2. Multiple inputs are handled correctly
3. Channel closure is detected
4. Backward compatibility with existing commands

## Files to Modify

- `src/app/repl.rs`: Main refactoring work
- `src/chat/input_channel.rs`: Add sender accessor if needed

## Verification Steps

1. Run existing tests to ensure no regression
2. Manual testing of REPL mode
3. Verify all existing commands still work
4. Check that input responsiveness is maintained
