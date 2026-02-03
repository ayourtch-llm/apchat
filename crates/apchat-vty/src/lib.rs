// VTY Output with Heart Emojis
//!
//! Provides centralized terminal output functions that prepend heart emojis
//! to every line of output for consistent formatting across APChat.

pub mod readline;
pub mod history;
pub mod instance;

pub use readline::{Readline, ReadlineResult};
pub use history::{ReadlineEntry, ReadlineHistory, load_history, save_history, load_and_add_to_editor, save_to_file};
pub use instance::ReadlineInstance;
use std::io::BufWriter;
use std::fs::OpenOptions;

/// Global broadcast channel for TextOutput messages
/// Allows non-blocking sends from synchronous code
/// This is set by apchat-main to connect print_with_emoji to the OutputRouter
static TEXT_OUTPUT_TX: once_cell::sync::OnceCell<tokio::sync::broadcast::Sender<apchat_mspc::output::TextOutput>> = once_cell::sync::OnceCell::new();

/// Set the global TEXT_OUTPUT_TX channel
/// This should be called by apchat-main during initialization
pub fn set_text_output_tx(tx: tokio::sync::broadcast::Sender<apchat_mspc::output::TextOutput>) {
    TEXT_OUTPUT_TX.set(tx).expect("TEXT_OUTPUT_TX already set");
}

/// Get the global TEXT_OUTPUT_TX channel
pub fn get_text_output_tx() -> Option<tokio::sync::broadcast::Sender<apchat_mspc::output::TextOutput>> {
    TEXT_OUTPUT_TX.get().cloned()
}

/// Atomic counter for tracking active HTTP requests
/// This module provides thread-safe tracking of ongoing LLM API requests
pub mod request_counter {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Global atomic counter for active HTTP requests
    static ACTIVE_REQUESTS: AtomicUsize = AtomicUsize::new(0);

    /// Get the current count of active requests
    ///
    /// # Returns
    /// * `usize` - The number of currently active HTTP requests
    pub fn get_count() -> usize {
        ACTIVE_REQUESTS.load(Ordering::Relaxed)
    }

    /// Increment the request counter
    fn increment() {
        ACTIVE_REQUESTS.fetch_add(1, Ordering::Relaxed);
println!("REQ: {:?}", ACTIVE_REQUESTS);
    }

    /// Decrement the request counter
    fn decrement() {
        ACTIVE_REQUESTS.fetch_sub(1, Ordering::Relaxed);
println!("DEQ: {:?}", ACTIVE_REQUESTS);
    }

    /// RAII guard that automatically increments the counter on creation
    /// and decrements it on drop
    #[derive(Debug)]
    pub struct RequestGuard {
        _marker: (),
    }

    impl RequestGuard {
        /// Create a new RequestGuard, incrementing the active request counter
        pub fn new() -> Self {
            increment();
            RequestGuard { _marker: () }
        }
    }

    impl Drop for RequestGuard {
        fn drop(&mut self) {
            decrement();
        }
    }
}

pub use request_counter::{RequestGuard, get_count};


use std::io::{self, Write};

/// Scrolls content upward by inserting a blank line at the specified position.
///
/// This function saves the current cursor position, moves up the specified number
/// of lines, inserts a blank line (pushing existing content down), prints the
/// provided text on that new line, and then restores the original cursor position.
///
/// This function uses the `crossterm` crate for cross-platform terminal control
/// for most operations, but uses a direct ANSI escape sequence for line insertion
/// since crossterm does not provide an InsertLines command.
///
/// # Arguments
/// * `lines_up` - How many lines up from the current cursor position to insert the new line
/// * `text` - The text to print on the newly inserted line
///
/// # Platform
/// This function is only available on Unix-like systems (Linux, macOS, BSD).
///
/// # Example
/// ```ignore,no_run
/// use apchat_vty::scroll_insert_up;
///
/// // This will move up 5 lines from current position, insert a blank line,
/// // print "New content 1" there, and restore cursor position
/// scroll_insert_up(5, "New content 1");
///
/// // You can use this in a loop for a scrolling effect:
/// for i in 1..=25 {
///     scroll_insert_up(5, &format!("I: {}", i));
///     std::thread::sleep(std::time::Duration::from_millis(500));
/// }
/// ```
#[cfg(unix)]
pub fn scroll_insert_up(lines_up: u16, text: &str) {
    use crossterm::{cursor, QueueableCommand, execute};

    // Save cursor position
    let _ = execute!(io::stdout(), cursor::SavePosition);

    // Move to the bottom of the screen (scroll if needed)
    let _ = execute!(io::stdout(), cursor::MoveTo(999, 1));
    println!("");

    // Restore cursor position
    let _ = execute!(io::stdout(), cursor::RestorePosition);

    // Save cursor position again
    let _ = execute!(io::stdout(), cursor::SavePosition);

    // Move up the specified number of lines
    let _ = execute!(io::stdout(), cursor::MoveUp(lines_up));

    // Insert a line (push content down) - using direct ANSI escape since
    // crossterm doesn't provide an InsertLines command
    print!("\x1b[L");

    // Print the text on the newly inserted line
    println!("{}", text);

    // Flush output to ensure immediate display
    let _ = io::stdout().flush();

    // Restore original cursor position
    let _ = execute!(io::stdout(), cursor::RestorePosition);
}

/// Prints text with a red heart emoji (❤️) prepended to each line.
///
/// # Arguments
/// * `text` - The text to print (can contain embedded newlines)
/// * `newline` - Whether to add a trailing newline
///
/// # Example
/// ```ignore
/// print_heart_red("Hello\nWorld", true);
/// // Outputs:
/// // ❤️ Hello
/// // ❤️ World
/// // (with trailing newline)
/// ```
pub fn print_heart_red(text: &str, newline: bool) {
    print_with_emoji("❤️", text, newline, io::stdout());
}

/// Prints text with a yellow heart emoji (💛) prepended to each line.
///
/// # Arguments
/// * `text` - The text to print (can contain embedded newlines)
/// * `newline` - Whether to add a trailing newline
///
/// # Example
/// ```ignore
/// print_heart_yellow("Warning!", false);
/// // Outputs:
/// // 💛 Warning! (no trailing newline)
/// ```
pub fn print_heart_yellow(text: &str, newline: bool) {
    print_with_emoji("💛", text, newline, io::stderr());
}

pub fn print_heart_to_file(text: &str, newline: bool) -> Result<(), std::io::Error> {
    let file = OpenOptions::new()
        .append(true) // Open file in append mode
        .create(true) // Create the file if it doesn't exist
        .open("/tmp/apchat-debug.txt")?; // Actually open file

    let mut writer = BufWriter::new(file);

    write!(writer, "{}", text)?;
    if newline {
        writeln!(writer, "")?;
    }
    writer.flush()?;
    Ok(())
}

/// Internal helper that prints with an emoji prepended to each line.
///
/// # Issue 138: Sends TextOutput messages to TEXT_OUTPUT_TX for routing
/// while maintaining backward compatibility with direct writes.
///
/// # Arguments
/// * `emoji` - The emoji to prepend (e.g., "❤️" or "💛")
/// * `text` - The text to print (can contain embedded newlines)
/// * `newline` - Whether to add a trailing newline
/// * `writer` - The output destination (stdout or stderr)
fn print_with_emoji(emoji: &str, text: &str, newline: bool, mut writer: impl io::Write) {
    let lines: Vec<&str> = text.split('\n').collect();

    // Issue 138: Send to TEXT_OUTPUT_TX if available (for OutputRouter integration)
    if let Some(ref tx) = get_text_output_tx() {
        let _ = tx.send(apchat_mspc::output::TextOutput::new(emoji, text, newline));
        return;
    } 

    for (i, line) in lines.iter().enumerate() {
        if i < lines.len() - 1 {
            let _ = writeln!(writer, "{} {}", emoji, line);
        } else {
            if lines.len() == 1 && !newline {
               let _ = write!(writer, "{}", line);
            } else {
               let _ = write!(writer, "{} {}", emoji, line);
            }
        }
    }

    // Add trailing newline if requested
    if newline {
        let _ = writeln!(writer);
    }

    // Flush to ensure output is written immediately
    let _ = writer.flush();

}
