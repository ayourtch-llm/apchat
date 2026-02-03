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
/// # Arguments
/// * `emoji` - The emoji to prepend (e.g., "❤️" or "💛")
/// * `text` - The text to print (can contain embedded newlines)
/// * `newline` - Whether to add a trailing newline
/// * `writer` - The output destination (stdout or stderr)
fn print_with_emoji(emoji: &str, text: &str, newline: bool, mut writer: impl io::Write) {
    let lines: Vec<&str> = text.split('\n').collect();

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
