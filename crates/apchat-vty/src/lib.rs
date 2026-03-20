// VTY Output with Heart Emojis
//!
//! Provides centralized terminal output functions that prepend heart emojis
//! to every line of output for consistent formatting across APChat.

pub mod readline;
pub mod history;
pub mod instance;

pub use readline::{Readline, ReadlineResult, IdleConfig};
pub use history::{ReadlineEntry, ReadlineHistory, load_history, save_history, load_and_add_to_editor, save_to_file};
pub use instance::ReadlineInstance;
use std::io::BufWriter;
use std::fs::OpenOptions;

use apchat_types::InferenceOutcome;

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
    }

    /// Decrement the request counter
    fn decrement() {
        ACTIVE_REQUESTS.fetch_sub(1, Ordering::Relaxed);
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

pub mod token_counter {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Global atomic counter for active HTTP requests
    static TOKEN_COUNT: AtomicUsize = AtomicUsize::new(0);

    /// Get the current count of active requests
    ///
    /// # Returns
    /// * `usize` - The number of currently active HTTP requests
    pub fn get_count() -> usize {
        TOKEN_COUNT.load(Ordering::Relaxed)
    }

    /// Increment the request counter
    pub fn increment(x: usize) {
        TOKEN_COUNT.fetch_add(x, Ordering::Relaxed);
    }
}

pub use request_counter::{RequestGuard, get_count};

/// Atomic flag for tracking active smart context compaction
/// This module provides thread-safe tracking of ongoing intelligent compaction operations
pub mod compaction_counter {
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Flag indicating if compaction is currently active
    static IS_COMPACTION_ACTIVE: AtomicBool = AtomicBool::new(false);

    /// Check if compaction is currently active
    pub fn is_active() -> bool {
        IS_COMPACTION_ACTIVE.load(Ordering::Relaxed)
    }

    /// Set compaction as active
    pub fn set_active() {
        IS_COMPACTION_ACTIVE.store(true, Ordering::Relaxed);
    }

    /// Clear compaction active flag
    pub fn clear() {
        IS_COMPACTION_ACTIVE.store(false, Ordering::Relaxed);
    }
}

pub use compaction_counter::{is_active as is_compaction_active, set_active as set_compaction_active, clear as clear_compaction};

/// Atomic counter for tracking active tool executions
/// This module provides thread-safe tracking of ongoing tool operations
pub mod tool_counter {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::sync::Mutex;

    /// Global atomic counter for active tool executions
    static ACTIVE_TOOLS: AtomicUsize = AtomicUsize::new(0);

    /// Flag indicating if a tool is currently active
    static IS_TOOL_ACTIVE: AtomicBool = AtomicBool::new(false);

    /// Mutex-protected current tool name (used for thread-safe access)
    static CURRENT_TOOL_NAME: Mutex<Option<String>> = Mutex::new(None);

    /// Get the current count of active tools
    ///
    /// # Returns
    /// * `usize` - The number of currently active tool executions
    pub fn get_count() -> usize {
        ACTIVE_TOOLS.load(Ordering::Relaxed)
    }

    /// Check if a tool is currently active
    pub fn is_tool_active() -> bool {
        IS_TOOL_ACTIVE.load(Ordering::Relaxed)
    }

    /// Get the current tool name if a tool is active
    pub fn get_current_tool_name() -> Option<String> {
        CURRENT_TOOL_NAME.lock().unwrap().clone()
    }

    /// Increment the tool counter
    fn increment() {
        ACTIVE_TOOLS.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement the tool counter
    fn decrement() {
        ACTIVE_TOOLS.fetch_sub(1, Ordering::Relaxed);
    }

    /// Set the current tool name
    pub fn set_tool_name(name: &str) {
        IS_TOOL_ACTIVE.store(true, Ordering::Relaxed);
        *CURRENT_TOOL_NAME.lock().unwrap() = Some(name.to_string());
    }

    /// Clear the current tool name
    pub fn clear_tool_name() {
        IS_TOOL_ACTIVE.store(false, Ordering::Relaxed);
        *CURRENT_TOOL_NAME.lock().unwrap() = None;
    }

    /// RAII guard that automatically increments the counter on creation
    /// and decrements it on drop
    #[derive(Debug)]
    pub struct ToolGuard {
        _marker: (),
    }

    impl ToolGuard {
        /// Create a new ToolGuard, incrementing the active tool counter
        pub fn new() -> Self {
            increment();
            ToolGuard { _marker: () }
        }

        /// Create a new ToolGuard with a tool name, incrementing the active tool counter
        pub fn new_with_tool_name(tool_name: &str) -> Self {
            increment();
            set_tool_name(tool_name);
            ToolGuard { _marker: () }
        }
    }

    impl Drop for ToolGuard {
        fn drop(&mut self) {
            clear_tool_name();
            decrement();
        }
    }
}

pub use tool_counter::{ToolGuard, get_count as get_tool_count, is_tool_active, get_current_tool_name};

/// Status information module
/// Provides atomic singletons for status-related values that appear in the title bar
pub mod status_info {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    use tokio::task::JoinHandle;

    /// Marquee display width in characters
    const MARQUEE_WIDTH: usize = 30;

    /// Global atomic counter for queued items
    static QUEUED: AtomicUsize = AtomicUsize::new(0);

    /// Global atomic counter for history items
    static HISTORY: AtomicUsize = AtomicUsize::new(0);

    /// Global atomic counter for context bytes
    static CONTEXT_BYTES: AtomicUsize = AtomicUsize::new(0);

    /// Global atomic counter for urgent messages
    static URGENT: AtomicUsize = AtomicUsize::new(0);

    /// Process ID (set once at initialization)
    static PID: AtomicUsize = AtomicUsize::new(0);

    /// Marquee text storage
    static MARQUEE_TEXT: Mutex<Option<String>> = Mutex::new(None);
    
    /// Marquee scroll position (character index)
    static MARQUEE_INDEX: AtomicUsize = AtomicUsize::new(0);

    /// Set the queued count
    pub fn set_queued(count: usize) {
        QUEUED.store(count, Ordering::Relaxed);
    }

    /// Get the queued count
    pub fn get_queued() -> usize {
        QUEUED.load(Ordering::Relaxed)
    }

    /// Set the history count
    pub fn set_history(count: usize) {
        HISTORY.store(count, Ordering::Relaxed);
    }

    /// Get the history count
    pub fn get_history() -> usize {
        HISTORY.load(Ordering::Relaxed)
    }

    /// Set the context bytes count
    pub fn set_context_bytes(count: usize) {
        CONTEXT_BYTES.store(count, Ordering::Relaxed);
    }

    /// Get the context bytes count
    pub fn get_context_bytes() -> usize {
        CONTEXT_BYTES.load(Ordering::Relaxed)
    }

    /// Set the urgent message count
    pub fn set_urgent(count: usize) {
        URGENT.store(count, Ordering::Relaxed);
    }

    /// Get the urgent message count
    pub fn get_urgent() -> usize {
        URGENT.load(Ordering::Relaxed)
    }

    /// Set the process ID (should only be called once)
    pub fn set_pid(pid: usize) {
        PID.store(pid, Ordering::Relaxed);
    }

    /// Get the process ID
    pub fn get_pid() -> usize {
        PID.load(Ordering::Relaxed)
    }

    /// Set the marquee text (no longer uses background thread - increments on-demand)
    /// Note: Resets scroll index to 0 if the new text is shorter than the current index
    /// 
    /// Sanitization: Replaces newlines with " ⋯ " to prevent display corruption
    /// and ensures the text stays within safe bounds.
    pub fn set_marquee(text: &str) {
        // Sanitize text: replace newlines and carriage returns with safe separators
        let sanitized = text
            .replace('\n', " ⋯ ")
            .replace('\r', " ⋯ ");
        
        let new_len = sanitized.chars().count();
        *MARQUEE_TEXT.lock().unwrap() = Some(sanitized);
        
        // Check if current index is beyond the new text length
        // If so, reset to 0 to avoid out-of-bounds scrolling
        let current = MARQUEE_INDEX.load(Ordering::Relaxed);
        if current >= new_len {
            MARQUEE_INDEX.store(0, Ordering::Relaxed);
        }
    }

    /// Clear the marquee text
    pub fn clear_marquee() {
        *MARQUEE_TEXT.lock().unwrap() = None;
        MARQUEE_INDEX.store(0, Ordering::Relaxed);
    }

    /// Get the marquee display text (scrolled to current position)
    pub fn get_marquee_display() -> String {
        let binding = MARQUEE_TEXT.lock().unwrap();
        let text = match binding.as_ref() {
            Some(t) => t,
            None => return " ".repeat(MARQUEE_WIDTH), // Always return 30 spaces
        };

        if text.is_empty() {
            return " ".repeat(MARQUEE_WIDTH); // Always return 30 spaces
        }

        // Increment scroll index on each call for on-demand scrolling
        let current = MARQUEE_INDEX.load(Ordering::Relaxed);
        MARQUEE_INDEX.store(current + 1, Ordering::Relaxed);
        
        let text_len = text.chars().count();

        if text_len <= MARQUEE_WIDTH {
            // Text fits in marquee width, pad it to exactly MARQUEE_WIDTH
            let mut result = text.clone();
            while result.chars().count() < MARQUEE_WIDTH {
                result.push(' ');
            }
            // Take exactly MARQUEE_WIDTH characters
            result.chars().take(MARQUEE_WIDTH).collect()
        } else {
            // For continuous seamless scrolling, create a quadrupled string
            // This creates the illusion of infinite scrolling right-to-left
            // Moving through 4x the string for smoother scrolling
            let quadrupled = format!("{}{}{}{}", text, text, text, text);
            let quadrupled_len = quadrupled.chars().count();
            
            // Scroll through the quadrupled string
            let start = current % quadrupled_len;
            
            if start >= quadrupled_len {
                " ".repeat(MARQUEE_WIDTH)
            } else {
                // Take exactly MARQUEE_WIDTH characters from the quadrupled string
                // This ensures seamless transition when we reach the end
                quadrupled.chars().skip(start).take(MARQUEE_WIDTH).collect()
            }
        }
    }
}

pub use status_info::{set_queued, get_queued, set_history, get_history, set_context_bytes, get_context_bytes, set_urgent, get_urgent, set_pid, get_pid, set_marquee, clear_marquee, get_marquee_display};

use std::io::{self, Write};
use crossterm::terminal::size as terminal_size;

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
pub fn scroll_insert_up(lines_up: u16, text: &str, scroll_up: bool) {
    use crossterm::{cursor, QueueableCommand, execute};

    // Save cursor position
    // let _ = execute!(io::stdout(), cursor::SavePosition);
    print!("\x1b[s");

    print!("\x1b[{}A", lines_up);
    let _ = io::stdout().flush();
    println!("\r{}", text);

    if scroll_up {
      // Move to the bottom of the screen (scroll if needed)
      // let _ = execute!(io::stdout(), cursor::MoveTo(999, 1));
      print!("\x1b[999;1H");
      println!("");

      // Restore cursor position
      // let _ = execute!(io::stdout(), cursor::RestorePosition);

      // Save cursor position again
      // let _ = execute!(io::stdout(), cursor::SavePosition);
      print!("\x1b[u");
      print!("\x1b[s");
    }

    // Move up the specified number of lines
    // let _ = execute!(io::stdout(), cursor::MoveUp(lines_up));
    print!("\x1b[{}A", lines_up);
    let _ = io::stdout().flush();

    // Insert a line (push content down) - using direct ANSI escape since
    // crossterm doesn't provide an InsertLines command
    if scroll_up {
      print!("\x1b[L");
    }
    let _ = io::stdout().flush();

    // Print the text on the newly inserted line
    // println!("{}", text);

    // Flush output to ensure immediate display
    let _ = io::stdout().flush();

    // Restore original cursor position
    // let _ = execute!(io::stdout(), cursor::RestorePosition);
    print!("\x1b[u");
}

/// Strips ANSI escape codes from a string to get the visible character count.
///
/// ANSI escape codes are sequences like `\x1b[31m` (red) or `\x1b[1m` (bold).
/// This function removes them so we can calculate the actual display width.
///
/// # Arguments
///
/// * `s` - The string that may contain ANSI codes
///
/// # Returns
///
/// * `String` - The string with ANSI codes removed
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // ANSI escape sequence starts
            if let Some(&'[') = chars.peek() {
                chars.next(); // consume '['
                // Skip until we find the end character (a letter, usually 'm')
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Calculates the display width of a string, accounting for ANSI codes and unicode.
///
/// This function strips ANSI escape codes and then calculates the actual display
/// width of the remaining characters. Most characters are 1 column wide, but some
/// unicode characters (like emojis) can be 2 columns wide.
///
/// # Arguments
///
/// * `s` - The string to measure
///
/// # Returns
///
/// * `usize` - The display width in columns
fn display_width(s: &str) -> usize {
    use unicode_width::UnicodeWidthStr;
    let stripped = strip_ansi_codes(s);
    UnicodeWidthStr::width(stripped.as_str())
}

/// Wraps text to fit within a specified width.
///
/// Respects word boundaries and handles ANSI escape codes properly.
///
/// # Arguments
///
/// * `text` - The text to wrap
/// * `max_width` - Maximum width in columns
///
/// # Returns
///
/// * `String` - The wrapped text with newlines
fn wrap_text_at_width(text: &str, max_width: usize) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut wrapped_lines = Vec::new();

    for line in lines {
        if line.is_empty() {
            wrapped_lines.push(String::new());
            continue;
        }

        let current_width = display_width(line);
        if current_width <= max_width {
            // Line fits as-is
            wrapped_lines.push(line.to_string());
            continue;
        }

        // Need to wrap the line
        let mut remaining = line;
        while !remaining.is_empty() {
            let mut current_pos = 0;
            let mut current_width = 0;
            let mut break_point = None;
            let mut last_space_pos = 0;
            let mut chars = remaining.char_indices().peekable();

            while let Some((pos, ch)) = chars.next() {
                let char_width = if ch as u32 > 0x1F300 { 2 } else { 1 };
                
                if current_width + char_width > max_width {
                    break;
                }

                current_width += char_width;
                current_pos = pos + ch.len_utf8();

                if ch == ' ' {
                    last_space_pos = pos;
                }

                if chars.peek().is_none() {
                    break_point = Some(current_pos);
                }
            }

            if let Some(pos) = break_point {
                let segment = &remaining[..pos];
                wrapped_lines.push(segment.to_string());
                remaining = &remaining[pos..];
                
                // Trim leading whitespace from next segment
                remaining = remaining.trim_start();
            } else if last_space_pos > 0 {
                // Break at last space
                let segment = &remaining[..=last_space_pos];
                wrapped_lines.push(segment.to_string());
                remaining = &remaining[last_space_pos + 1..];
            } else {
                // No break point found, force break at max_width
                let mut chars = remaining.char_indices().peekable();
                let mut cut_pos = 0;
                let mut width = 0;
                
                while let Some((pos, ch)) = chars.next() {
                    let char_width = if ch as u32 > 0x1F300 { 2 } else { 1 };
                    if width + char_width > max_width {
                        break;
                    }
                    width += char_width;
                    cut_pos = pos + ch.len_utf8();
                }
                
                wrapped_lines.push(remaining[..cut_pos].to_string());
                remaining = &remaining[cut_pos..];
            }
        }
    }

    wrapped_lines.join("\n")
}

/// Prints text with a red heart emoji (❤️) prepended to each line.
///
/// Text is automatically wrapped to fit the terminal width, accounting for the emoji prefix.
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
    let width = terminal_size().map(|(cols, _rows)| cols as usize).unwrap_or(80);
    // Emoji is 2 columns wide, so wrap text at width - 2
    let wrap_width = width.saturating_sub(2);
    let wrapped = wrap_text_at_width(text, wrap_width.max(1));
    print_with_emoji("❤️", &wrapped, newline, io::stdout());
}

/// Prints text with a yellow heart emoji (💛) prepended to each line.
///
/// Text is automatically wrapped to fit the terminal width.
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
    let width = terminal_size().map(|(cols, _rows)| cols as usize).unwrap_or(80);
    let wrapped = wrap_text_at_width(text, width);
    print_with_emoji("💛", &wrapped, newline, io::stderr());
}

/// Pretty-prints a debug outcome with a styled layout.
///
/// Displays the outcome title in green text on gray background,
/// followed by the outcome value (if any) pretty-printed in light-gray text on gray background.
///
/// # Arguments
/// * `title` - The title to display (e.g., "process_llm_response outcome")
/// * `outcome` - The InferenceOutcome value to display
///
/// # Example
/// ```ignore
/// print_outcome_box("process_llm_response outcome", &outcome);
/// ```
pub fn print_outcome_box(title: &str, outcome: &InferenceOutcome) {
    use std::fmt::Write;
    
    // ANSI codes:
    // Green text: \x1b[38;2;0;255;0m
    // Gray background: \x1b[48;2;60;60;60m
    // Light-gray text: \x1b[38;2;200;200;200m
    // Reset: \x1b[0m
    
    // Format the title in green on gray background
    let topbar = format!(
        "\x1b[48;2;60;60;60m\x1b[38;2;0;255;0m{}:\x1b[0m",
        title
    );
    
    // Format the outcome value
    let content = match outcome {
        InferenceOutcome::Response(ref text) => {
            // Pretty-print the response string in light-gray on gray background
            format!(
                "\x1b[48;2;60;60;60m\x1b[38;2;200;200;200m{}\x1b[0m",
                text
            )
        }
        InferenceOutcome::Interrupted => {
            format!(
                "\x1b[48;2;60;60;60m\x1b[38;2;200;200;200m{}\x1b[0m",
                "Interrupted by user"
            )
        }
        InferenceOutcome::Error => {
            format!(
                "\x1b[48;2;60;60;60m\x1b[38;2;200;200;200m{}\x1b[0m",
                "Error occurred"
            )
        }
        InferenceOutcome::ToolsContinue => {
            format!(
                "\x1b[48;2;60;60;60m\x1b[38;2;200;200;200m{}\x1b[0m",
                "Continuing with tool calls"
            )
        }
    };
    
    print_heart_red(&format!("{} {}", topbar, content), true);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_heart_red_does_not_panic() {
        // Single line
        print_heart_red("Hello", true);
        // Multi-line
        print_heart_red("Hello\nWorld", true);
        // Without trailing newline
        print_heart_red("No newline", false);
        // Empty string
        print_heart_red("", true);
    }

    #[test]
    fn test_print_heart_yellow_does_not_panic() {
        // Single line
        print_heart_yellow("Warning!", true);
        // Without trailing newline
        print_heart_yellow("Warning!", false);
        // Empty string
        print_heart_yellow("", true);
        // Multi-line
        print_heart_yellow("Line1\nLine2\nLine3", true);
    }

    #[test]
    fn test_print_outcome_box_does_not_panic() {
        use apchat_types::InferenceOutcome;

        // Response variant
        print_outcome_box("test title", &InferenceOutcome::Response("some text".to_string()));
        // Interrupted variant
        print_outcome_box("test title", &InferenceOutcome::Interrupted);
        // Error variant
        print_outcome_box("test title", &InferenceOutcome::Error);
        // ToolsContinue variant
        print_outcome_box("test title", &InferenceOutcome::ToolsContinue);
    }

    #[test]
    fn test_scroll_insert_up_does_not_panic() {
        // Basic call without scroll
        scroll_insert_up(1, "test content", false);
        // With scroll
        scroll_insert_up(2, "scrolled content", true);
        // Empty text
        scroll_insert_up(1, "", false);
    }

    #[test]
    fn test_print_with_emoji_single_line() {
        let mut buf = Vec::new();
        print_with_emoji("*", "hello", true, &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("hello"));
    }

    #[test]
    fn test_print_with_emoji_multiline() {
        let mut buf = Vec::new();
        print_with_emoji("*", "line1\nline2", true, &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("line1"));
        assert!(output.contains("line2"));
    }

    #[test]
    fn test_print_with_emoji_no_newline() {
        let mut buf = Vec::new();
        print_with_emoji("*", "hello", false, &mut buf);
        let output = String::from_utf8(buf).unwrap();
        // Should not end with newline
        assert!(!output.ends_with('\n'));
    }

    #[test]
    fn test_strip_ansi_codes() {
        let input = "\x1b[31mRed\x1b[0m Normal";
        let output = strip_ansi_codes(input);
        assert_eq!(output, "Red Normal");
    }

    #[test]
    fn test_display_width_plain() {
        assert_eq!(display_width("hello"), 5);
    }

    #[test]
    fn test_display_width_with_ansi() {
        assert_eq!(display_width("\x1b[31mhello\x1b[0m"), 5);
    }

    #[test]
    fn test_request_counter() {
        let initial = request_counter::get_count();
        {
            let _guard = RequestGuard::new();
            assert_eq!(request_counter::get_count(), initial + 1);
        }
        assert_eq!(request_counter::get_count(), initial);
    }

    #[test]
    fn test_tool_counter() {
        let initial = tool_counter::get_count();
        {
            let _guard = ToolGuard::new_with_tool_name("test_tool");
            assert_eq!(tool_counter::get_count(), initial + 1);
            assert!(tool_counter::is_tool_active());
            assert_eq!(tool_counter::get_current_tool_name(), Some("test_tool".to_string()));
        }
        assert_eq!(tool_counter::get_count(), initial);
    }

    #[test]
    fn test_compaction_counter() {
        // Save and restore state to avoid affecting other tests
        let was_active = compaction_counter::is_active();
        compaction_counter::set_active();
        assert!(compaction_counter::is_active());
        compaction_counter::clear();
        assert!(!compaction_counter::is_active());
        if was_active {
            compaction_counter::set_active();
        }
    }

    #[test]
    fn test_token_counter() {
        let initial = token_counter::get_count();
        token_counter::increment(42);
        assert_eq!(token_counter::get_count(), initial + 42);
    }

    #[test]
    fn test_status_info_queued() {
        status_info::set_queued(5);
        assert_eq!(status_info::get_queued(), 5);
        status_info::set_queued(0);
    }

    #[test]
    fn test_status_info_marquee() {
        status_info::set_marquee("test marquee text");
        let display = status_info::get_marquee_display();
        // Display should be exactly 30 chars (MARQUEE_WIDTH)
        assert_eq!(display.chars().count(), 30);
        status_info::clear_marquee();
    }

    #[test]
    fn test_status_info_history() {
        status_info::set_history(42);
        assert_eq!(status_info::get_history(), 42);
        status_info::set_history(0);
        assert_eq!(status_info::get_history(), 0);
    }

    #[test]
    fn test_status_info_context_bytes() {
        status_info::set_context_bytes(1024);
        assert_eq!(status_info::get_context_bytes(), 1024);
        status_info::set_context_bytes(0);
        assert_eq!(status_info::get_context_bytes(), 0);
    }

    #[test]
    fn test_status_info_urgent() {
        status_info::set_urgent(3);
        assert_eq!(status_info::get_urgent(), 3);
        status_info::set_urgent(0);
        assert_eq!(status_info::get_urgent(), 0);
    }

    #[test]
    fn test_status_info_pid() {
        status_info::set_pid(12345);
        assert_eq!(status_info::get_pid(), 12345);
    }

    #[test]
    fn test_tool_guard_without_name() {
        let initial = tool_counter::get_count();
        {
            let _guard = ToolGuard::new();
            assert_eq!(tool_counter::get_count(), initial + 1);
            // Without a tool name, is_tool_active may or may not be set
            // depending on prior state, but counter should increment
        }
        assert_eq!(tool_counter::get_count(), initial);
    }

    #[test]
    fn test_marquee_clear() {
        status_info::set_marquee("some text");
        status_info::clear_marquee();
        let display = status_info::get_marquee_display();
        // After clearing, display should be 30 spaces
        assert_eq!(display, " ".repeat(30));
    }

    #[test]
    fn test_marquee_empty_text() {
        status_info::set_marquee("");
        let display = status_info::get_marquee_display();
        assert_eq!(display, " ".repeat(30));
        status_info::clear_marquee();
    }

    #[test]
    fn test_marquee_with_newlines_sanitized() {
        status_info::set_marquee("line1\nline2\rline3");
        let display = status_info::get_marquee_display();
        // Should not contain raw newlines
        assert!(!display.contains('\n'));
        assert!(!display.contains('\r'));
        assert_eq!(display.chars().count(), 30);
        status_info::clear_marquee();
    }

    #[test]
    fn test_wrap_text_at_width_short_line() {
        let result = wrap_text_at_width("hello", 80);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_wrap_text_at_width_long_line() {
        let result = wrap_text_at_width("hello world foo bar baz", 12);
        // Should wrap into multiple lines
        assert!(result.contains('\n'));
        // Each line should be at most 12 display chars
        for line in result.split('\n') {
            assert!(display_width(line) <= 12, "Line '{}' exceeds width 12", line);
        }
    }

    #[test]
    fn test_wrap_text_at_width_empty() {
        let result = wrap_text_at_width("", 80);
        assert_eq!(result, "");
    }

    #[test]
    fn test_wrap_text_at_width_preserves_newlines() {
        let result = wrap_text_at_width("line1\nline2\nline3", 80);
        assert_eq!(result, "line1\nline2\nline3");
    }

    #[test]
    fn test_strip_ansi_codes_nested() {
        let input = "\x1b[1m\x1b[31mBold Red\x1b[0m";
        let output = strip_ansi_codes(input);
        assert_eq!(output, "Bold Red");
    }

    #[test]
    fn test_display_width_empty() {
        assert_eq!(display_width(""), 0);
    }

    #[test]
    fn test_print_heart_to_file_does_not_panic() {
        // This writes to /tmp/apchat-debug.txt; just verify no panic
        let _ = print_heart_to_file("test output", true);
        let _ = print_heart_to_file("no newline", false);
    }
}
