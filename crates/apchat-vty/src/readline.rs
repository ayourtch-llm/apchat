//! Readline implementation with terminal mode management.
//!
//! This module provides a `Readline` struct that manages terminal I/O
//! using "semi-raw" mode: raw input with normal output.

use crossterm::cursor::{MoveDown, MoveTo, MoveToColumn, MoveUp};
use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{Clear, size as terminal_size};
use crossterm::QueueableCommand;
use std::io::{self, Write};
use std::time::Duration;
use chrono::prelude::*;

use apchat_mspc::MspcMessage;
use apchat_mspc::output::TextOutput;
use crate::scroll_insert_up;
use crate::request_counter;
use crate::tool_counter;
use crate::tool_counter::ToolGuard;
use crate::token_counter;
use crate::status_info;
use crate::print_heart_yellow;
use crate::compaction_counter;

// Termios imports for raw mode on stdin only
use libc::{tcsetattr, termios, ECHO, ICANON, ISIG, STDIN_FILENO, TCSANOW};

/// Idle timeout configuration
#[derive(Clone, Debug)]
pub struct IdleConfig {
    pub timeout_secs: u32,
    pub input_text: String,
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
/// * `usize` - The number of visible characters (excluding ANSI codes)
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

/// Get the display width of a single character in terminal columns.
fn char_display_width(c: char) -> usize {
    use unicode_width::UnicodeWidthChar;
    UnicodeWidthChar::width(c).unwrap_or(0)
}

/// Result type for the readline operation.
///
/// # Variants
///
/// * `Input(String)` - User entered a line of text
/// * `Eof` - End of file (Ctrl-D)
/// * `Interrupt` - Interrupted (Ctrl-C)
/// * `Signal(MspcMessage)` - MPSC signal received
#[derive(Debug)]
pub enum ReadlineResult {
    /// User entered a line of text
    Input(String),
    /// End of file (Ctrl-D)
    Eof,
    /// Interrupted (Ctrl-C)
    Interrupt,
    /// MPSC signal received
    Signal(MspcMessage),
}

/// Internal result type for key event handling.
///
/// # Variants
///
/// * `Continue` - Continue reading input
/// * `Redraw` - Redraw the screen and continue
/// * `Return(ReadlineResult)` - Return the specified result
enum KeyResult {
    /// Continue reading input
    Continue,
    /// Redraw the screen and continue
    Redraw,
    /// Return the specified result
    Return(ReadlineResult),
}

/// Edit mode for the readline interface.
#[derive(Clone, Copy, PartialEq, Debug)]
enum EditMode {
    /// Normal editing mode
    Normal,
    /// Reverse search mode (Ctrl-R)
    Search,
    /// Confirmation mode (y/n prompt)
    Confirmation,
}

/// Enables raw mode on stdin only (not stdout).
///
/// This saves the original terminal settings and modifies them to:
/// - Disable line buffering (ICANON)
/// - Disable echo (ECHO)
/// - Disable signal generation (ISIG)
///
/// # Safety
///
/// This function uses raw libc calls to manipulate terminal settings.
fn enable_raw_mode_on_stdin() -> io::Result<termios> {
    unsafe {
        let mut term: termios = std::mem::zeroed();
        
        // Get current terminal settings
        if libc::tcgetattr(STDIN_FILENO, &mut term) != 0 {
            return Err(io::Error::last_os_error());
        }
        
        let original = term;

        // Set raw mode flags (clear ICANON, ECHO)
        // We keep ISIG set so Ctrl-C and other signal characters work as expected
        term.c_lflag &= !(ICANON | ECHO);

        // Set minimum characters to read and timeout
        term.c_cc[libc::VMIN] = 1;   // Minimum number of characters for non-canonical read
        term.c_cc[libc::VTIME] = 0;  // Timeout in deciseconds (0 = blocking)
        
        // Apply new settings
        if tcsetattr(STDIN_FILENO, TCSANOW, &term) != 0 {
            return Err(io::Error::last_os_error());
        }

        // Enable bracketed paste mode
        // This allows us to distinguish paste events from regular typing
        print!("\x1b[?2004h");
        io::stdout().flush()?;

        Ok(original)
    }
}

/// Restores terminal settings to the original state.
///
/// # Safety
///
/// This function uses raw libc calls to manipulate terminal settings.
fn restore_terminal_settings(original: &termios) -> io::Result<()> {
    unsafe {
        if tcsetattr(STDIN_FILENO, TCSANOW, original) != 0 {
            return Err(io::Error::last_os_error());
        }

        // Disable bracketed paste mode
        print!("\x1b[?2004l");
        io::stdout().flush()?;

        Ok(())
    }
}

/// Readline struct that manages terminal mode, input state, and command history.
///
/// This struct enables raw mode on construction (for character-by-character input)
/// and automatically disables raw mode when dropped (for cleanup).
///
/// # Terminal Mode
///
/// The "semi-raw" mode means:
/// - Raw input: Character-by-character input without line buffering
/// - Normal output: Output is processed normally (not raw)
/// - Bracketed paste mode: Enabled to support intelligent paste handling
///
/// # Paste Behavior
///
/// Bracketed paste mode is automatically enabled. When you paste multiline text,
/// the newlines are converted to spaces to keep the input on a single line.
///
/// Example:
/// ```ignore
/// Pasting:    Becomes:
/// line1       line1 line2 line3
/// line2
/// line3
/// ```
///
/// # Keybindings
///
/// Movement:
/// - `Left`, `Right`, `Ctrl-Left`, `Ctrl-Right` - Move by character/word
/// - `Home`, `End`, `Ctrl-A`, `Ctrl-E` - Move to start/end
/// - `Up`, `Down` - Navigate history
///
/// Editing:
/// - `Backspace`, `Delete` - Delete characters
/// - `Ctrl-K`, `Ctrl-U`, `Ctrl-W` - Kill text
/// - `Ctrl-Y` - Yank (paste) last killed text
///
/// Special:
/// - `Enter` - Submit the current line
/// - `Ctrl-C` - Interrupt (sends interrupt signal)
/// - `Ctrl-D` - Exit if line is empty, otherwise delete character
/// - `Ctrl-R` - Reverse search in history
///
/// # History Management
///
/// The readline maintains a command history that can be navigated with Up/Down arrows.
///
/// # Example
///
/// ```no_run
/// use apchat_vty::Readline;
///
/// let readline = Readline::new().unwrap();
/// // Terminal is now in raw mode
/// // ... use readline for input ...
/// drop(readline);  // Terminal mode is restored automatically
/// ```
pub struct Readline {
    /// The current input line buffer (now multiline)
    lines: Vec<String>,
    /// Current cursor line (0-based)
    cursor_line: usize,
    /// Cursor offset from bottom (the actual cursor)
    cursor_offset_from_bottom: usize,
    /// Current cursor column within the current line (0-based)
    cursor_col: usize,
    /// Original terminal settings before enabling raw mode
    original_termios: Option<termios>,
    /// Command history (previous commands)
    history: Vec<String>,
    /// Current position in history navigation (None = editing current line)
    history_index: Option<usize>,
    /// Saved lines when entering history navigation
    saved_lines: Vec<String>,
    /// Current edit mode (Normal or Search)
    mode: EditMode,
    /// Search pattern in search mode
    search_pattern: String,
    /// Indices of matching history entries
    search_matches: Vec<usize>,
    /// Current position in search matches
    search_match_index: usize,
    /// Original lines before entering search mode
    original_lines: Vec<String>,
    /// Original cursor line before entering search mode
    original_cursor_line: usize,
    /// Original cursor column before entering search mode
    original_cursor_col: usize,
    /// Kill ring (circular buffer for killed text)
    kill_ring: Vec<String>,
    /// Current position in kill ring
    kill_ring_index: usize,
    /// Maximum size of kill ring (Emacs default is 16)
    max_kill_ring_size: usize,
    /// Maximum number of lines to display
    max_lines: usize,
    /// Scroll offset for displaying lines
    scroll_offset: usize,
    /// Current editor height in lines (never decreases until Enter is pressed)
    editor_height: usize,
    /// Compatibility field: current line (deprecated, use lines[cursor_line])
    line: String,
    /// Compatibility field: cursor position (deprecated, use cursor_col)
    cursor: usize,
    /// Confirmation prompt/message (set when in Confirmation mode)
    confirmation_prompt: Option<String>,
    /// Confirmation ID for tool confirmation requests (for routing the response back to the tool)
    confirmation_id: Option<String>,
    /// Visible width of the prompt (for auto-wrapping calculations)
    prompt_width: usize,
    /// Cached terminal screen width (cleared on startup and on SIGWINCH)
    cached_screen_width: Option<u16>,
    /// Idle command to inject when timeout is reached
    idle_command: Option<String>,
    /// Seconds of inactivity before injecting idle_command (None = disabled)
    idle_period_secs: Option<u32>,
    /// Next time to inject idle command (Some = enabled)
    idle_command_time: Option<chrono::DateTime<chrono::Local>>,
    /// Optional filename to load after initialization (set via --load flag)
    pub load_filename: Option<String>,
}

impl Readline {
    /// Gets the terminal screen width, using cached value if available.
    ///
    /// This method caches the screen width to avoid calling `crossterm::terminal::size()`
    /// on every character, which adds significant latency. The cache is cleared
    /// on startup and when a window resize event (SIGWINCH) is detected.
    ///
    /// # Returns
    ///
    /// * `u16` - The terminal screen width in columns
    fn get_screen_width(&mut self) -> u16 {
        // Return cached value if available
        if let Some(width) = self.cached_screen_width {
            return width;
        }

        // Query terminal size and cache the result
        let width = terminal_size().map(|(cols, _rows)| cols).unwrap_or(80);
        self.cached_screen_width = Some(width);
        width
    }

    /// Clears the cached screen width.
    ///
    /// This should be called when the terminal window is resized to ensure
    /// the width cache is invalidated and will be refreshed on the next call
    /// to `get_screen_width()`.
    fn clear_screen_width_cache(&mut self) {
        self.cached_screen_width = None;
    }

    /// Creates a new `Readline` instance and enables raw mode.
    ///
    /// # Errors
    ///
    /// Returns an error if raw mode cannot be enabled.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use apchat_vty::Readline;
    ///
    /// let readline = Readline::new().expect("Failed to initialize readline");
    /// ```
    pub fn new() -> io::Result<Self> {
        // Enable raw mode on stdin only (not stdout)
        let original_termios = Some(enable_raw_mode_on_stdin()?);

        Ok(Readline {
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_offset_from_bottom: 0,
            cursor_col: 0,
            original_termios,
            history: Vec::new(),
            history_index: None,
            saved_lines: Vec::new(),
            mode: EditMode::Normal,
            search_pattern: String::new(),
            search_matches: Vec::new(),
            search_match_index: 0,
            original_lines: Vec::new(),
            original_cursor_line: 0,
            original_cursor_col: 0,
            kill_ring: Vec::new(),
            kill_ring_index: 0,
            max_kill_ring_size: 16,
            max_lines: 10,
            scroll_offset: 0,
            editor_height: 1,
            line: String::new(),
            cursor: 0,
            confirmation_prompt: None,
            confirmation_id: None,
            prompt_width: 0,
            cached_screen_width: None,
            idle_command: None,
            idle_period_secs: None,
            idle_command_time: None,
            load_filename: None,
        })
    }

    /// Get the number of seconds until the next idle command injection
    /// Returns None if idle timeout is not configured
    fn get_idle_time_remaining(&self) -> Option<i64> {
        match (&self.idle_command_time, &self.idle_period_secs) {
            (Some(time), Some(_period)) => {
                let now = chrono::Local::now();
                let diff = *time - now;
                Some(diff.num_seconds())
            }
            _ => None,
        }
    }

    /// Returns the current input line.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use apchat_vty::Readline;
    ///
    /// let readline = Readline::new().unwrap();
    /// assert_eq!(readline.line(), "");
    /// ```
    pub fn line(&self) -> &str {
        &self.lines[self.cursor_line]
    }

    /// Returns the current cursor position.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use apchat_vty::Readline;
    ///
    /// let readline = Readline::new().unwrap();
    /// assert_eq!(readline.cursor(), 0);
    /// ```
    pub fn cursor(&self) -> usize {
        self.cursor_col
    }

    /// Sets the output line.
    /// Useful for unit tests that need to modify the line.
    #[allow(dead_code)] // Only used in tests
    #[cfg(test)]
    pub fn set_line(&mut self, line: &str) {
        if self.cursor_line < self.lines.len() {
            self.lines[self.cursor_line] = line.to_string();
            // Sync deprecated field
            self.line = self.lines[self.cursor_line].clone();
        }
    }

    /// Sets the cursor position.
    /// Useful for unit tests that need to position the cursor.
    #[allow(dead_code)] // Only used in tests
    #[cfg(test)]
    pub fn set_cursor(&mut self, cursor: usize, line: Option<usize>) {
        if let Some(l) = line {
            self.cursor_line = l;
        }
        self.cursor_col = cursor;
        // Sync deprecated field
        if self.cursor_line < self.lines.len() {
            self.cursor = self.cursor_col;
        }
        // Ensure cursor_line is within bounds
        if self.cursor_line >= self.lines.len() && !self.lines.is_empty() {
            self.cursor_line = self.lines.len() - 1;
        }
        if self.cursor_line < self.lines.len() {
            // Clamp cursor_col to the new line length
            let max_col = self.lines[self.cursor_line].chars().count();
            self.cursor_col = self.cursor_col.min(max_col);
            self.cursor = self.cursor_col;
        }
    }

    /// Checks if raw mode is currently enabled.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use apchat_vty::Readline;
    ///
    /// let readline = Readline::new().unwrap();
    /// assert!(readline.is_raw_mode_enabled());
    /// ```
    pub fn is_raw_mode_enabled(&self) -> bool {
        // Raw mode is enabled if we have saved terminal settings
        self.original_termios.is_some()
    }

    /// Adds an entry to the command history.
    ///
    /// This method adds a command to the history buffer, excluding empty lines
    /// and consecutive duplicates.
    ///
    /// # Arguments
    ///
    /// * `entry` - The command string to add to history
    ///
    /// # Example
    ///
    /// ```ignore
    /// use apchat_vty::Readline;
    ///
    /// let mut readline = Readline::new().unwrap();
    /// readline.add_history_entry("hello world");
    /// assert_eq!(readline.get_history_entries().len(), 1);
    /// ```
    pub fn add_history_entry(&mut self, entry: &str) {
        // Don't add empty lines to history
        if entry.trim().is_empty() {
            return;
        }

        // Don't add consecutive duplicates
        if let Some(last) = self.history.last() {
            if last == entry {
                return;
            }
        }

        self.history.push(entry.to_string());
    }

    pub fn clear_history_for_tests_only(&mut self) {
        self.history.clear();
    }

    /// Navigates to the previous entry in history (Up arrow).
    ///
    /// Saves the current line buffer on first navigation, then replaces
    /// the line buffer with the previous history entry.
    ///
    /// # Returns
    ///
    /// * `true` - Successfully navigated to a previous entry
    /// * `false` - Already at the oldest entry (no change)
    ///
    /// # Example
    ///
    /// ```ignore
    /// use apchat_vty::Readline;
    ///
    /// let mut readline = Readline::new().unwrap();
    /// readline.add_history_entry("first command");
    /// readline.add_history_entry("second command");
    ///
    /// // Start typing something
    /// readline.line = "new input".to_string();
    /// readline.cursor = 9;
    ///
    /// // Navigate up - should save current line and go to "second command"
    /// assert!(readline.history_up());
    /// assert_eq!(readline.line(), "second command");
    /// assert_eq!(readline.saved_line, "new input");
    ///
    /// // Navigate up again - should go to "first command"
    /// assert!(readline.history_up());
    /// assert_eq!(readline.line(), "first command");
    ///
    /// // Navigate up again - should stay at "first command" (oldest)
    /// assert!(!readline.history_up());
    /// assert_eq!(readline.line(), "first command");
    /// ```
    pub fn history_up(&mut self) -> bool {
        if self.history.is_empty() {
            return false;
        }

        // If we're not currently navigating history, save the current multiline state
        if self.history_index.is_none() {
            self.saved_lines = self.lines.clone();
            self.history_index = Some(self.history.len().saturating_sub(1));
            let entry = &self.history[self.history_index.unwrap()];
            self.lines = entry.split('\n').map(String::from).collect();
            // Position cursor at end of last line
            self.cursor_line = self.lines.len().saturating_sub(1);
            self.cursor_col = self.lines.last().map(|l| l.chars().count()).unwrap_or(0);
            // Sync deprecated fields
            self.cursor = self.cursor_col;
            if !self.lines.is_empty() {
                self.line = self.lines[self.cursor_line].clone();
            }
            return true;
        }

        // Check if we can go up (to older entries)
        if let Some(idx) = self.history_index {
            if idx > 0 {
                self.history_index = Some(idx - 1);
                let entry = &self.history[idx - 1];
                self.lines = entry.split('\n').map(String::from).collect();
                // Position cursor at end of last line
                self.cursor_line = self.lines.len().saturating_sub(1);
                self.cursor_col = self.lines.last().map(|l| l.chars().count()).unwrap_or(0);
                // Sync deprecated fields
                self.cursor = self.cursor_col;
                if !self.lines.is_empty() {
                    self.line = self.lines[self.cursor_line].clone();
                }
                return true;
            }
        }

        false
    }

    /// Navigates to the next entry in history (Down arrow).
    ///
    /// Restores newer history entries, or restores the original line buffer
    /// when reaching the end of history.
    ///
    /// # Returns
    ///
    /// * `true` - Successfully navigated to a newer entry
    /// * `false` - Already at the newest entry (no change)
    ///
    /// # Example
    ///
    /// ```ignore
    /// use apchat_vty::Readline;
    ///
    /// let mut readline = Readline::new().unwrap();
    /// readline.add_history_entry("first command");
    /// readline.add_history_entry("second command");
    ///
    /// // Navigate to oldest
    /// readline.history_up();
    /// readline.history_up();
    /// assert_eq!(readline.line(), "first command");
    ///
    /// // Navigate down - should go to "second command"
    /// assert!(readline.history_down());
    /// assert_eq!(readline.line(), "second command");
    ///
    /// // Navigate down again - should return to empty line (current input)
    /// assert!(readline.history_down());
    /// assert_eq!(readline.line(), "");
    ///
    /// // Navigate down again - should stay at current input
    /// assert!(!readline.history_down());
    /// ```
    pub fn history_down(&mut self) -> bool {
        if self.history.is_empty() || self.history_index.is_none() {
            return false;
        }

        if let Some(idx) = self.history_index {
            if idx < self.history.len() - 1 {
                // Move to newer entry
                self.history_index = Some(idx + 1);
                let entry = &self.history[idx + 1];
                self.lines = entry.split('\n').map(String::from).collect();
                // Position cursor at end of last line
                self.cursor_line = self.lines.len().saturating_sub(1);
                self.cursor_col = self.lines.last().map(|l| l.chars().count()).unwrap_or(0);
                return true;
            } else {
                // Exit history navigation, restore saved multiline state
                self.history_index = None;
                self.lines = self.saved_lines.clone();
                // Restore cursor to end of last line
                self.cursor_line = self.lines.len().saturating_sub(1);
                self.cursor_col = self.lines.last().map(|l| l.chars().count()).unwrap_or(0);
                self.saved_lines.clear();
                return true;
            }
        }

        false
    }

    /// Returns all entries in the command history.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use apchat_vty::Readline;
    ///
    /// let mut readline = Readline::new().unwrap();
    /// readline.add_history_entry("command 1");
    /// readline.add_history_entry("command 2");
    ///
    /// let entries = readline.get_history_entries();
    /// assert_eq!(entries.len(), 2);
    /// assert_eq!(entries[0], "command 1");
    /// assert_eq!(entries[1], "command 2");
    /// ```
    pub fn get_history_entries(&self) -> &[String] {
        &self.history
    }

    /// Exits history navigation mode and restores the current line.
    ///
    /// This is called when the user modifies the line while navigating history,
    /// to switch back to editing the current line.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use apchat_vty::Readline;
    ///
    /// let mut readline = Readline::new().unwrap();
    /// readline.add_history_entry("old command");
    ///
    /// // Navigate to history
    /// readline.history_up();
    /// assert_eq!(readline.line(), "old command");
    ///
    /// // Modify the line (this would normally be done by key handlers)
    /// readline.exit_history_navigation();
    ///
    /// // Now we're editing the current line (empty in this case)
    /// assert_eq!(readline.line(), "");
    /// assert!(readline.history_index.is_none());
    /// ```
    pub fn exit_history_navigation(&mut self) {
        // Only clear the history index - don't restore the line
        // The current line content should remain as-is (user continues editing)
        self.history_index = None;
        self.saved_lines.clear();
    }

    /// Exits history navigation and clears the line buffer.
    ///
    /// This is used when the user starts typing a new character while viewing
    /// history - they want to start a new line, not edit the history entry.
    fn exit_history_navigation_with_clear(&mut self) {
        self.history_index = None;
        self.saved_lines.clear();
        // Clear the lines and reset cursor to start a fresh line
        self.lines = vec![String::new()];
        self.cursor_line = 0;
        self.cursor_col = 0;
        // Sync deprecated fields
        self.line = String::new();
        self.cursor = 0;
    }

    /// Enters reverse search mode (Ctrl-R).
    ///
    /// Saves the current line and cursor position, then switches to search mode.
    fn enter_search_mode(&mut self) {
        self.original_lines = self.lines.clone();
        self.original_cursor_line = self.cursor_line;
        self.original_cursor_col = self.cursor_col;
        // Also save to compatibility fields
        self.line = self.lines.get(0).cloned().unwrap_or_default();
        self.cursor = self.cursor_col;
        self.search_pattern.clear();
        self.search_matches.clear();
        self.search_match_index = 0;
        self.mode = EditMode::Search;

        // If there's history, show the most recent entry when pattern is empty
        if !self.history.is_empty() {
            self.update_search();
        }
    }

    /// Exits reverse search mode.
    ///
    /// Restores the original line and cursor position, then switches back to normal mode.
    fn exit_search_mode(&mut self) {
        self.mode = EditMode::Normal;
        self.lines = self.original_lines.clone();
        self.cursor_line = self.original_cursor_line;
        self.cursor_col = self.original_cursor_col;
        // Update compatibility fields
        self.line = self.lines.get(0).cloned().unwrap_or_default();
        self.cursor = self.cursor_col;
        self.search_pattern.clear();
        self.search_matches.clear();
    }

    /// Updates the search pattern and finds matching history entries.
    ///
    /// Searches for history entries containing the current pattern (case-sensitive).
    /// Matches are ordered from newest to oldest.
    fn update_search(&mut self) {
        self.search_matches.clear();
        self.search_match_index = 0;

        // Search through history from newest to oldest
        for (idx, entry) in self.history.iter().enumerate().rev() {
            if entry.contains(&self.search_pattern) {
                self.search_matches.push(idx);
            }
        }

        // Display the first match if any
        if !self.search_matches.is_empty() {
            self.search_match_index = 0;
            let match_idx = self.search_matches[0];
            // Split on newlines in case the history entry has multiple lines
            self.lines = self.history[match_idx].split('\n').map(String::from).collect();
            // Position cursor at end of last line
            self.cursor_line = self.lines.len().saturating_sub(1);
            self.cursor_col = self.lines.last().map(|l| l.chars().count()).unwrap_or(0);
            // Sync deprecated fields
            self.cursor = self.cursor_col;
            if !self.lines.is_empty() {
                self.line = self.lines[self.cursor_line].clone();
            }
        } else {
            // No matches, clear the line
            self.lines[self.cursor_line].clear();
            self.cursor_col = 0;
        }
    }

    /// Cycles to the next search match.
    ///
    /// In reverse search mode, Ctrl-R cycles through matches from newest to oldest.
    /// When reaching the oldest match, wraps around to the newest.
    fn cycle_search_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }

        // Move to next match (with wraparound)
        self.search_match_index = (self.search_match_index + 1) % self.search_matches.len();
        let match_idx = self.search_matches[self.search_match_index];
        // Split on newlines in case the history entry has multiple lines
        self.lines = self.history[match_idx].split('\n').map(String::from).collect();
        // Position cursor at end of last line
        self.cursor_line = self.lines.len().saturating_sub(1);
        self.cursor_col = self.lines.last().map(|l| l.chars().count()).unwrap_or(0);
        // Sync deprecated fields
        self.cursor = self.cursor_col;
        if !self.lines.is_empty() {
            self.line = self.lines[self.cursor_line].clone();
        }
    }

    /// Enters confirmation mode with a prompt.
    ///
    /// Stores the original state and switches to confirmation mode.
    /// The prompt is stored for display during confirmation.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The confirmation prompt to display
    pub fn enter_confirmation_mode(&mut self, prompt: String, confirmation_id: Option<String>) {
        self.original_lines = self.lines.clone();
        self.original_cursor_line = self.cursor_line;
        self.original_cursor_col = self.cursor_col;
        self.confirmation_prompt = Some(prompt);
        self.confirmation_id = confirmation_id;
        self.mode = EditMode::Confirmation;
        // Clear the current line for user response
        self.lines = vec![String::new()];
        self.cursor_line = 0;
        self.cursor_col = 0;
    }

    /// Exits confirmation mode.
    ///
    /// Restores the original line and cursor position, then switches back to normal mode.
    fn exit_confirmation_mode(&mut self) {
        self.mode = EditMode::Normal;
        self.lines = self.original_lines.clone();
        self.cursor_line = self.original_cursor_line;
        self.cursor_col = self.original_cursor_col;
        self.confirmation_prompt = None;
        self.confirmation_id = None;
    }

    /// Gets the current confirmation prompt, if any.
    pub fn confirmation_prompt(&self) -> Option<&str> {
        self.confirmation_prompt.as_deref()
    }

    /// Handles a character input event.
    ///
    /// Inserts the character at the current cursor position.
    ///
    /// # Arguments
    ///
    /// * `c` - The character to insert
    ///
    /// # Returns
    ///
    /// * `true` - The line was modified, a redraw is needed
    ///
    /// # Example
    ///
    /// ```ignore
    /// use apchat_vty::Readline;
    ///
    /// let mut readline = Readline::new().unwrap();
    ///
    /// // Insert characters
    /// assert!(readline.handle_char('h'));
    /// assert!(readline.handle_char('i'));
    /// assert_eq!(readline.line(), "hi");
    /// assert_eq!(readline.cursor(), 2);
    ///
    /// // Insert in middle
    /// readline.cursor = 1;
    /// assert!(readline.handle_char('e'));
    /// assert_eq!(readline.line(), "hei");
    /// assert_eq!(readline.cursor(), 2);
    /// ```
    pub fn handle_char(&mut self, c: char) -> bool {
        // Exit history navigation if we were in it
        // Preserves the current line content for editing (bug fix)
        if self.history_index.is_some() {
            self.exit_history_navigation();
        }

        // Insert character at cursor position in the current line
        let line = &mut self.lines[self.cursor_line];

        // Convert character position to byte position for UTF-8
        let byte_pos = line.chars().take(self.cursor_col).map(|c| c.len_utf8()).sum();
        line.insert(byte_pos, c);

        self.cursor_col += 1;

        // Auto-split line if it exceeds terminal width
        self.split_line_if_needed();

        true
    }

    /// Handles the Backspace key.
    ///
    /// Deletes the character before the cursor.
    ///
    /// # Returns
    ///
    /// * `true` - The line was modified, a redraw is needed
    /// * `false` - Nothing to delete (cursor at start)
    ///
    /// # Example
    ///
    /// ```ignore
    /// use apchat_vty::Readline;
    ///
    /// let mut readline = Readline::new().unwrap();
    /// readline.line = "hello".to_string();
    /// readline.cursor = 5;
    ///
    /// // Delete last character
    /// assert!(readline.handle_backspace());
    /// assert_eq!(readline.line(), "hell");
    /// assert_eq!(readline.cursor(), 4);
    ///
    /// // Try to delete at start - should do nothing
    /// readline.cursor = 0;
    /// assert!(!readline.handle_backspace());
    /// assert_eq!(readline.line(), "hell");
    /// ```
    pub fn handle_backspace(&mut self) -> bool {
        // Exit history navigation if we were in it
        if self.history_index.is_some() {
            self.exit_history_navigation();
        }

        // Get current line and validate cursor position
        let current_line_len = self.lines[self.cursor_line].chars().count();
        
        // Ensure cursor_col is within bounds
        if self.cursor_col > current_line_len {
            self.cursor_col = current_line_len;
        }

        if self.cursor_col > 0 {
            // Delete character within current line
            self.cursor_col -= 1;
            let line_chars: Vec<char> = self.lines[self.cursor_line].chars().collect();
            let new_line: String = line_chars[..self.cursor_col]
                .iter()
                .chain(line_chars[self.cursor_col + 1..].iter())
                .collect();
            self.lines[self.cursor_line] = new_line;
            true
        } else if self.cursor_col == 0 && self.cursor_line > 0 {
            // Join current line with end of previous line
            let current_line = self.lines[self.cursor_line].clone();
            let prev_line_len = self.lines[self.cursor_line - 1].chars().count();
            self.lines[self.cursor_line - 1].push_str(&current_line);
            self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_col = prev_line_len;

            // Update scroll_offset if needed
            if self.cursor_line < self.scroll_offset {
                self.scroll_offset = self.cursor_line;
            }

            true
        } else {
            false
        }
    }

    /// Handles the Delete key.
    ///
    /// Deletes the character at the cursor position.
    ///
    /// # Returns
    ///
    /// * `true` - The line was modified, a redraw is needed
    /// * `false` - Nothing to delete (cursor at end)
    ///
    /// # Example
    ///
    /// ```ignore
    /// use apchat_vty::Readline;
    ///
    /// let mut readline = Readline::new().unwrap();
    /// readline.line = "hello".to_string();
    /// readline.cursor = 1;
    ///
    /// // Delete character at cursor
    /// assert!(readline.handle_delete());
    /// assert_eq!(readline.line(), "hllo");
    /// assert_eq!(readline.cursor(), 1);
    ///
    /// // Try to delete at end - should do nothing
    /// readline.cursor = 4;
    /// assert!(!readline.handle_delete());
    /// assert_eq!(readline.line(), "hllo");
    /// ```
    pub fn handle_delete(&mut self) -> bool {
        // Exit history navigation if we were in it
        if self.history_index.is_some() {
            self.exit_history_navigation();
        }

        let current_line_len = self.lines[self.cursor_line].chars().count();

        // Ensure cursor_col is within bounds
        if self.cursor_col > current_line_len {
            self.cursor_col = current_line_len;
        }

        if self.cursor_col < current_line_len {
            // Delete character within current line
            let line_chars: Vec<char> = self.lines[self.cursor_line].chars().collect();
            let new_line: String = line_chars[..self.cursor_col]
                .iter()
                .chain(line_chars[self.cursor_col + 1..].iter())
                .collect();
            self.lines[self.cursor_line] = new_line;
            true
        } else if self.cursor_col == current_line_len && self.cursor_line < self.lines.len() - 1 {
            // Join next line with current line
            let next_line = self.lines[self.cursor_line + 1].clone();
            self.lines[self.cursor_line].push_str(&next_line);
            self.lines.remove(self.cursor_line + 1);

            // Update scroll_offset if needed (shouldn't need adjustment in this case
            // since we're removing a line after the cursor)

            true
        } else {
            false
        }
    }

    /// Handles the Left arrow key.
    ///
    /// Moves the cursor one position to the left.
    ///
    /// # Returns
    ///
    /// * `true` - Cursor moved, a redraw is needed
    /// * `false` - Cursor already at start
    ///
    /// # Example
    ///
    /// ```ignore
    /// use apchat_vty::Readline;
    ///
    /// let mut readline = Readline::new().unwrap();
    /// readline.line = "hi".to_string();
    /// readline.set_cursor(2, None);
    ///
    /// // Move left
    /// assert!(readline.handle_left());
    /// assert_eq!(readline.cursor(), 1);
    ///
    /// // Move left again
    /// assert!(readline.handle_left());
    /// assert_eq!(readline.cursor(), 0);
    ///
    /// // Try to move past start - should do nothing
    /// assert!(!readline.handle_left());
    /// assert_eq!(readline.cursor(), 0);
    /// ```
    pub fn handle_left(&mut self) -> bool {
        // If cursor_col > 0: move cursor left within current line
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            self.update_scroll_offset();
            return true;
        }

        // If cursor_col == 0 and cursor_line > 0: move to end of previous line
        if self.cursor_col == 0 && self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.cursor_col = self.lines[self.cursor_line].chars().count();
            self.update_scroll_offset();
            return true;
        }

        false
    }

    /// Handles the Right arrow key.
    ///
    /// Moves the cursor one position to the right.
    ///
    /// # Returns
    ///
    /// * `true` - Cursor moved, a redraw is needed
    /// * `false` - Cursor already at end
    ///
    /// # Example
    ///
    /// ```ignore
    /// use apchat_vty::Readline;
    ///
    /// let mut readline = Readline::new().unwrap();
    /// readline.line = "hi".to_string();
    /// readline.cursor = 0;
    ///
    /// // Move right
    /// assert!(readline.handle_right());
    /// assert_eq!(readline.cursor(), 1);
    ///
    /// // Move right again
    /// assert!(readline.handle_right());
    /// assert_eq!(readline.cursor(), 2);
    ///
    /// // Try to move past end - should do nothing
    /// assert!(!readline.handle_right());
    /// assert_eq!(readline.cursor(), 2);
    /// ```
    pub fn handle_right(&mut self) -> bool {
        let current_line_len = self.lines[self.cursor_line].chars().count();

        // If cursor_col < current line length: move cursor right within current line
        if self.cursor_col < current_line_len {
            self.cursor_col += 1;
            self.update_scroll_offset();
            return true;
        }

        // If cursor_col == current line length and cursor_line < lines.len() - 1: move to start of next line
        if self.cursor_col == current_line_len && self.cursor_line < self.lines.len() - 1 {
            self.cursor_line += 1;
            self.cursor_col = 0;
            self.update_scroll_offset();
            return true;
        }

        false
    }

    /// Handles the Home key.
    ///
    /// Moves the cursor to the start of the line.
    ///
    /// # Returns
    ///
    /// * `true` - Cursor moved, a redraw is needed
    /// * `false` - Cursor already at start
    ///
    /// # Example
    ///
    /// ```ignore
    /// use apchat_vty::Readline;
    ///
    /// let mut readline = Readline::new().unwrap();
    /// readline.line = "hello".to_string();
    /// readline.cursor = 5;
    ///
    /// // Move to start
    /// assert!(readline.handle_home());
    /// assert_eq!(readline.cursor(), 0);
    ///
    /// // Already at start - should return false
    /// assert!(!readline.handle_home());
    /// assert_eq!(readline.cursor(), 0);
    /// ```
    pub fn handle_home(&mut self) -> bool {
        if self.cursor_col == 0 {
            return false;
        }

        self.cursor_col = 0;
        true
    }

    /// Handles the End key.
    ///
    /// Moves the cursor to the end of the line.
    ///
    /// # Returns
    ///
    /// * `true` - Cursor moved, a redraw is needed
    /// * `false` - Cursor already at end
    ///
    /// # Example
    ///
    /// ```ignore
    /// use apchat_vty::Readline;
    ///
    /// let mut readline = Readline::new().unwrap();
    /// readline.line = "hello".to_string();
    /// readline.cursor = 0;
    ///
    /// // Move to end
    /// assert!(readline.handle_end());
    /// assert_eq!(readline.cursor(), 5);
    ///
    /// // Already at end - should return false
    /// assert!(!readline.handle_end());
    /// assert_eq!(readline.cursor(), 5);
    /// ```
    pub fn handle_end(&mut self) -> bool {
        let line_len = self.lines[self.cursor_line].chars().count();
        
        // Ensure cursor_col is within bounds
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
        
        if self.cursor_col >= line_len {
            return false;
        }

        self.cursor_col = line_len;
        true
    }

    /// Kills (cuts) text from cursor to end of line (Ctrl-K).
    ///
    /// The killed text is added to the kill ring for later yanking.
    ///
    /// # Returns
    ///
    /// * `true` - Text was killed, a redraw is needed
    /// * `false` - No text to kill
    pub fn kill_to_end(&mut self) -> bool {
        let line_len = self.lines[self.cursor_line].chars().count();
        
        // Ensure cursor_col is within bounds
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
        
        if self.cursor_col > line_len {
            return false;
        }
        // if the line is empty - kill it
        if line_len == 0 {
            if self.cursor_line + 1 < self.lines.len() {
               self.lines.remove(self.cursor_line);
               return true;
            } else {
               return false;
            }
        }

        // Get text from cursor to end
        let byte_pos = self.lines[self.cursor_line].chars().take(self.cursor_col).map(|c| c.len_utf8()).sum();
        let killed = self.lines[self.cursor_line][byte_pos..].to_string();

        // Remove text from cursor to end
        self.lines[self.cursor_line] = self.lines[self.cursor_line].clone();
        self.lines[self.cursor_line].truncate(byte_pos);

        // Add to kill ring
        self.add_to_kill_ring(killed);
        true
    }

    /// Kills (cuts) text from start of line to cursor (Ctrl-U).
    ///
    /// The killed text is added to the kill ring for later yanking.
    ///
    /// # Returns
    ///
    /// * `true` - Text was killed, a redraw is needed
    /// * `false` - No text to kill
    pub fn kill_to_start(&mut self) -> bool {
        let line_len = self.lines[self.cursor_line].chars().count();
        
        // Ensure cursor_col is within bounds
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
        
        if self.cursor_col == 0 {
            return false;
        }

        // Get text from start to cursor
        let byte_pos = self.lines[self.cursor_line].chars().take(self.cursor_col).map(|c| c.len_utf8()).sum();
        let killed = self.lines[self.cursor_line][..byte_pos].to_string();

        // Remove text from start to cursor
        self.lines[self.cursor_line] = self.lines[self.cursor_line][byte_pos..].to_string();
        self.cursor_col = 0;

        // Add to kill ring
        self.add_to_kill_ring(killed);
        true
    }

    /// Kills (cuts) word to the right of cursor (Alt-D).
    ///
    /// Words are sequences of alphanumeric characters.
    ///
    /// # Returns
    ///
    /// * `true` - Text was killed, a redraw is needed
    /// * `false` - No text to kill
    pub fn kill_word_right(&mut self) -> bool {
        let line_len = self.lines[self.cursor_line].chars().count();
        
        // Ensure cursor_col is within bounds
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
        
        if self.cursor_col >= line_len {
            return false;
        }

        // Find end of current word
        let chars: Vec<char> = self.lines[self.cursor_line].chars().collect();
        let mut end = self.cursor_col;

        // Skip non-alphanumeric characters
        while end < line_len && !chars[end].is_alphanumeric() {
            end += 1;
        }

        // Skip alphanumeric characters (the word)
        while end < line_len && chars[end].is_alphanumeric() {
            end += 1;
        }

        // Clamp end to line length
        if end > line_len {
            end = line_len;
        }

        if end == self.cursor_col {
            return false;
        }

        // Get the word to kill
        let start_byte = self.lines[self.cursor_line].chars().take(self.cursor_col).map(|c| c.len_utf8()).sum();
        let end_byte = self.lines[self.cursor_line].chars().take(end).map(|c| c.len_utf8()).sum();
        let killed = self.lines[self.cursor_line][start_byte..end_byte].to_string();

        // Remove the word
        self.lines[self.cursor_line] = format!("{}{}", &self.lines[self.cursor_line][..start_byte], &self.lines[self.cursor_line][end_byte..]);

        // Add to kill ring
        self.add_to_kill_ring(killed);
        true
    }

    /// Kills (cuts) word to the left of cursor (Ctrl-W).
    ///
    /// Words are sequences of alphanumeric characters.
    ///
    /// # Returns
    ///
    /// * `true` - Text was killed, a redraw is needed
    /// * `false` - No text to kill
    pub fn kill_word_left(&mut self) -> bool {
        let line_len = self.lines[self.cursor_line].chars().count();
        
        // Ensure cursor_col is within bounds
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
        
        if self.cursor_col == 0 {
            return false;
        }

        // Find start of previous word
        let chars: Vec<char> = self.lines[self.cursor_line].chars().collect();
        let mut start = self.cursor_col;

        // Skip non-alphanumeric characters
        while start > 0 && !chars[start - 1].is_alphanumeric() {
            start -= 1;
        }

        // Skip alphanumeric characters (the word)
        while start > 0 && chars[start - 1].is_alphanumeric() {
            start -= 1;
        }

        if start == self.cursor_col {
            return false;
        }

        // Get the word to kill
        let start_byte = self.lines[self.cursor_line].chars().take(start).map(|c| c.len_utf8()).sum();
        let end_byte = self.lines[self.cursor_line].chars().take(self.cursor_col).map(|c| c.len_utf8()).sum();
        let killed = self.lines[self.cursor_line][start_byte..end_byte].to_string();

        // Remove the word and update cursor
        self.lines[self.cursor_line] = format!("{}{}", &self.lines[self.cursor_line][..start_byte], &self.lines[self.cursor_line][end_byte..]);
        self.cursor_col = start;

        // Add to kill ring
        self.add_to_kill_ring(killed);
        true
    }

    /// Yanks (pastes) the last killed text (Ctrl-Y).
    ///
    /// Inserts the most recently killed text at the cursor position.
    ///
    /// # Returns
    ///
    /// * `true` - Text was yanked, a redraw is needed
    /// * `false` - Kill ring is empty
    pub fn yank(&mut self) -> bool {
        if self.kill_ring.is_empty() {
            return false;
        }

        // Get the most recent kill (index adjusted to point to last entry)
        let index = if self.kill_ring_index == 0 {
            self.kill_ring.len() - 1
        } else {
            self.kill_ring_index - 1
        };

        let text = &self.kill_ring[index];

        // Insert at cursor position
        let byte_pos = self.lines[self.cursor_line].chars().take(self.cursor_col).map(|c| c.len_utf8()).sum();
        self.lines[self.cursor_line].insert_str(byte_pos, text);
        self.cursor_col += text.chars().count();
        true
    }

    /// Inserts a string at the cursor position.
    ///
    /// Helper method to insert multiple characters at once.
    /// This is used by paste handling to insert multiple characters efficiently.
    ///
    /// # Arguments
    ///
    /// * `s` - The string to insert
    fn insert_str(&mut self, s: &str) {
        for c in s.chars() {
            self.handle_char(c);
        }
    }

    /// Handles paste events from bracketed paste mode.
    ///
    /// When text is pasted, we preserve newlines to support multiline paste.
    /// This allows pasting multiple lines of text at once.
    ///
    /// # Arguments
    ///
    /// * `content` - The pasted content (may contain newlines)
    ///
    /// # Returns
    ///
    /// * `true` - The line was modified, a redraw is needed
    /// * `false` - Nothing to insert
    pub fn handle_paste(&mut self, content: String) -> bool {
        if content.is_empty() {
            return false;
        }

        // Exit history navigation if we were in it
        if self.history_index.is_some() {
            self.exit_history_navigation();
        }

        // Split the pasted content by newlines
        let pasted_lines: Vec<&str> = content.split('\n').collect();

        if pasted_lines.len() == 1 {
            // Single line paste: insert at cursor position in current line
            let current = self.lines[self.cursor_line].clone();
            let byte_pos = current.chars().take(self.cursor_col).map(|c| c.len_utf8()).sum();
            self.lines[self.cursor_line].insert_str(byte_pos, pasted_lines[0]);
            self.cursor_col += pasted_lines[0].chars().count();
        } else {
            // Multi-line paste: preserve newlines
            let current = self.lines[self.cursor_line].clone();
            let byte_pos = current.chars().take(self.cursor_col).map(|c| c.len_utf8()).sum();

            // Split current line at cursor position
            let before = &current[..byte_pos];
            let after = &current[byte_pos..];

            // Insert first line of pasted content at cursor position
            self.lines[self.cursor_line] = format!("{}{}", before, pasted_lines[0]);

            // Insert middle lines as new lines
            for (i, line) in pasted_lines.iter().skip(1).take(pasted_lines.len() - 2).enumerate() {
                self.lines.insert(self.cursor_line + 1 + i, line.to_string());
            }

            // Insert last line and append the rest of the original line
            let last_line = pasted_lines.last().unwrap();
            let final_line = format!("{}{}", last_line, after);
            self.lines.insert(self.cursor_line + pasted_lines.len() - 1, final_line);

            // Note: No need to remove the original line since we already replaced it in step 2
            // The old line at cursor_line was overwritten with "before + pasted_lines[0]"

            // Move cursor to end of last pasted line
            self.cursor_line += pasted_lines.len() - 1;
            self.cursor_col = last_line.chars().count();

            // Update scroll offset if needed
            self.update_scroll_offset();
        }

        true
    }

    /// Moves cursor left by one word (Ctrl-Left or Alt-B).
    ///
    /// Words are sequences of alphanumeric characters.
    ///
    /// # Returns
    ///
    /// * `true` - Cursor moved, a redraw is needed
    /// * `false` - Cursor already at start
    pub fn handle_word_left(&mut self) -> bool {
        if self.cursor_col == 0 {
            return false;
        }

        let chars: Vec<char> = self.lines[self.cursor_line].chars().collect();
        let mut new_pos = self.cursor_col;

        // Skip non-alphanumeric characters
        while new_pos > 0 && !chars[new_pos - 1].is_alphanumeric() {
            new_pos -= 1;
        }

        // Skip alphanumeric characters (the word)
        while new_pos > 0 && chars[new_pos - 1].is_alphanumeric() {
            new_pos -= 1;
        }

        self.cursor_col = new_pos;
        true
    }

    /// Moves cursor right by one word (Ctrl-Right or Alt-F).
    ///
    /// Words are sequences of alphanumeric characters.
    ///
    /// # Returns
    ///
    /// * `true` - Cursor moved, a redraw is needed
    /// * `false` - Cursor already at end
    pub fn handle_word_right(&mut self) -> bool {
        let line_len = self.lines[self.cursor_line].chars().count();
        if self.cursor_col >= line_len {
            return false;
        }

        let chars: Vec<char> = self.lines[self.cursor_line].chars().collect();
        let mut new_pos = self.cursor_col;

        // Skip alphanumeric characters (the current word)
        while new_pos < line_len && chars[new_pos].is_alphanumeric() {
            new_pos += 1;
        }

        // Skip non-alphanumeric characters
        while new_pos < line_len && !chars[new_pos].is_alphanumeric() {
            new_pos += 1;
        }

        self.cursor_col = new_pos;
        true
    }

    /// Adds text to the kill ring.
    ///
    /// The kill ring is a circular buffer with a maximum size.
    fn add_to_kill_ring(&mut self, text: String) {
        // Add to kill ring
        self.kill_ring.push(text);

        // Update index
        self.kill_ring_index = self.kill_ring.len();

        // Trim if exceeds max size
        if self.kill_ring.len() > self.max_kill_ring_size {
            self.kill_ring.remove(0);
            self.kill_ring_index = self.kill_ring.len();
        }
    }

    /// Inserts a new line at the current cursor position.
    ///
    /// This method splits the current line at the cursor position and creates
    /// a new line with the text after the cursor. The cursor is then moved to
    /// the start of the new line.
    ///
    /// # Returns
    ///
    /// * `true` - A redraw is needed
    pub fn handle_newline(&mut self) -> bool {
        // Get the current line content
        let current = self.lines[self.cursor_line].clone();
        
        // Find byte positions for slicing
        let byte_pos = current.chars().take(self.cursor_col).map(|c| c.len_utf8()).sum();
        let before = &current[..byte_pos];
        let after = &current[byte_pos..];
        
        // Update current line and insert new line
        self.lines[self.cursor_line] = before.to_string();
        self.lines.insert(self.cursor_line + 1, after.to_string());
        
        // Move cursor to start of new line
        self.cursor_line += 1;
        self.cursor_col = 0;
        
        // Update scroll offset to keep cursor visible
        self.update_scroll_offset();
        true
    }

    /// Updates the scroll offset to ensure the cursor line is visible.
    ///
    /// This method adjusts `scroll_offset` so that the current cursor line
    /// is always within the visible display range (max_lines).
    fn update_scroll_offset(&mut self) {
        if self.cursor_line < self.scroll_offset {
            // Cursor is above visible area, scroll up
            self.scroll_offset = self.cursor_line;
        } else if self.cursor_line >= self.scroll_offset + self.max_lines {
            // Cursor is below visible area, scroll down
            self.scroll_offset = self.cursor_line - self.max_lines + 1;
        }
    }

    /// Checks if the current line exceeds terminal width and splits it if needed.
    ///
    /// This function is called after character insertion to auto-wrap lines that
    /// are too long for the terminal. It splits at word boundaries when possible,
    /// falling back to forced breaks if no word boundary exists.
    ///
    /// # Algorithm
    ///
    /// 1. Calculate the display width of the current line
    /// 2. Get terminal width and calculate available space (accounting for prompt on first line)
    /// 3. If line exceeds available width, find split point:
    ///    - Search backward from overflow point for a space (word boundary)
    ///    - If no space found, split at the overflow point (forced break)
    /// 4. Split the line and update cursor position to maintain typing continuity
    fn split_line_if_needed(&mut self) {
        // Get terminal width from cache
        let terminal_width = self.get_screen_width() as usize;

        // Calculate available width for text
        // On first line (line 0), subtract the prompt width
        let available_width = if self.cursor_line == 0 {
            terminal_width.saturating_sub(self.prompt_width)
        } else {
            terminal_width
        };

        // Clone the current line to avoid borrow issues
        let line = self.lines[self.cursor_line].clone();

        // Don't split if line is empty or fits within available width
        if line.is_empty() || display_width(&line) <= available_width {
            return;
        }

        // Find the split point
        let line_width = display_width(&line);
        let mut current_width = 0;
        let mut split_idx = 0;

        // Iterate through characters to find where we exceed available width
        for (idx, ch) in line.char_indices() {
            let char_width = char_display_width(ch);
            if current_width + char_width > available_width {
                // We've exceeded the limit - this is our overflow point
                split_idx = idx;
                break;
            }
            current_width += char_width;
        }

        // Search backward from split point for a word boundary (space)
        let actual_split_idx = if split_idx > 0 {
            line[..split_idx]
                .rfind(' ')
                .map(|pos| pos)
                .unwrap_or(split_idx)
        } else {
            split_idx
        };

        // Skip trailing space if we split at word boundary
        let final_split_idx = if actual_split_idx > 0 && line.as_bytes()[actual_split_idx] == b' ' {
            actual_split_idx + 1
        } else {
            actual_split_idx
        };

        // Calculate the character position at the split point (not byte index)
        // This is needed because cursor_col tracks character positions, not byte indices
        let split_char_pos = line[..final_split_idx].chars().count();

        // Perform the split if we have something to split on
        if final_split_idx < line.len() {
            let before = &line[..final_split_idx];
            let after = &line[final_split_idx..];

            // Check if we can merge the overflow text with the next line
            let merged_with_next = if self.cursor_line + 1 < self.lines.len() {
                let next_line = &self.lines[self.cursor_line + 1];

                // Concatenate directly - 'after' already starts with the word (no leading space)
                // and next_line may have leading spaces from previous splits
                let merged = format!("{}{}", after, next_line);

                // Check if merged line fits within available width
                if display_width(&merged) <= available_width {
                    // Merge with next line instead of creating a new line
                    self.lines[self.cursor_line] = before.to_string();
                    self.lines[self.cursor_line + 1] = merged;

                    // Keep cursor on the same line
                    // cursor_col stays the same since we're still in the "before" part

                    // Update scroll offset to keep cursor visible
                    self.update_scroll_offset();

                    true
                } else {
                    false
                }
            } else {
                false
            };

            // If we couldn't merge with the next line, proceed with normal split
            if !merged_with_next {
                // Update current line and insert new line
                self.lines[self.cursor_line] = before.to_string();
                self.lines.insert(self.cursor_line + 1, after.to_string());

                // Move cursor to appropriate position in the new line
                self.cursor_line += 1;

                // Adjust cursor column: if cursor was after the split point,
                // position it relative to where it was in the original line
                if self.cursor_col >= split_char_pos {
                    self.cursor_col = self.cursor_col - split_char_pos;
                } else {
                    // Cursor was before the split point, keep it on the same line
                    self.cursor_line -= 1;
                    // cursor_col stays the same since we're still in the "before" part
                }

                // Update scroll offset to keep cursor visible
                self.update_scroll_offset();
            }
        }
    }

    /// Checks if the cursor is at the very end of all text.
    ///
    /// Returns true if the cursor is on the last line and at the end of that line.
    fn is_at_end(&self) -> bool {
        self.cursor_line == self.lines.len() - 1
            && self.cursor_col == self.lines[self.cursor_line].chars().count()
    }

    /// Returns the full text with newlines.
    ///
    /// Joins all lines with "\n" to create the complete text content.
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    /// Clears input and resets cursor to the beginning.
    ///
    /// Resets all lines to a single empty line, moves cursor to the start,
    /// and resets the scroll offset to zero.
    pub fn reset_input(&mut self) {
        let stdout = &mut std::io::stdout();
        self.lines = vec![String::new()];
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
        while self.cursor_line > 0 {
            stdout.queue(MoveUp(1)).ok();
            self.cursor_offset_from_bottom += 1;
        }
        /* 
        self.editor_height = 1;
        self.cursor_offset_from_bottom = 1;
        */
    }

    /// Redraws the current line to the terminal.
    ///
    /// This function clears the current line and redraws it with the prompt
    /// and current input, positioning the cursor correctly.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The prompt string to display (e.g., "> ")
    ///
    /// # Example
    ///
    /// ```ignore
    /// use apchat_vty::Readline;
    ///
    /// let mut readline = Readline::new().unwrap();
    /// readline.set_line("hello");
    /// readline.set_cursor(5, None);
    ///
    /// // Redraw with prompt
    /// readline.redraw("> ");
    /// ```
    pub fn redraw(&mut self, prompt: &str) {
        let stdout = &mut std::io::stdout();

        let screen_width: usize = self.get_screen_width() as usize;

        // In search mode, display the search interface with multiline support
        if self.mode == EditMode::Search {
            // Calculate visible range and expand editor height if needed (same as normal mode)
            let start = self.scroll_offset;
            let end = start + self.max_lines.min(self.lines.len());
            let display_count = end - start;

            // Expand editor height if needed
            let mut lines_to_add = 0;
            if display_count > self.editor_height {
                lines_to_add = display_count - self.editor_height;
                for _ in 0..lines_to_add {
                    println!();
                }
                self.editor_height = display_count;
            }

            // Recalculate with actual editor height
            let start = self.scroll_offset;
            let end = start + self.editor_height;
            let display_count = end - start;

            // Move to the top of the editor area
            let visual_line = self.cursor_line.saturating_sub(self.scroll_offset);
            let lines_to_move = self.editor_height.saturating_sub(self.cursor_offset_from_bottom);

            for _ in 0..lines_to_move {
                stdout.queue(MoveUp(1)).ok();
                self.cursor_offset_from_bottom += 1;
            }

            // Draw title bar with search info
            stdout.queue(MoveUp(1)).ok();
            self.cursor_offset_from_bottom += 1;
            stdout.queue(MoveToColumn(0)).ok();

            // Build search title
            let local_time = &Local::now().time().to_string()[0..8];
            let search_status = if self.search_matches.is_empty() {
                "(no match)".to_string()
            } else {
                format!("{}/{}", self.search_match_index + 1, self.search_matches.len())
            };

            // Build tool status display
            let tool_status = if tool_counter::is_tool_active() {
                if let Some(tool_name) = tool_counter::get_current_tool_name() {
                    format!("TOOL({})", tool_name)
                } else {
                    "TOOL(?)".to_string()
                }
            } else {
                "TOOL(0)".to_string()
            };

            let mut title = format!(
                "(reverse-i-search)`{}': {} | time: {} | req: {} | tok: {} | {} | queued: {} | history: {} | ctx: {} | urgent: {} | pid: {}",
                self.search_pattern,
                search_status,
                &local_time,
                request_counter::get_count(),
                token_counter::get_count(),
                tool_status,
                status_info::get_queued(),
                status_info::get_history(),
                status_info::get_context_bytes(),
                status_info::get_urgent(),
                status_info::get_pid()
            );

            // Ensure title never exceeds screen width
            let title_display_width = display_width(&title);
            if title_display_width > screen_width {
                let mut current_width = 0;
                let mut truncated = String::new();
                for ch in title.chars() {
                    let char_width = char_display_width(ch);
                    if current_width + char_width > screen_width {
                        break;
                    }
                    current_width += char_width;
                    truncated.push(ch);
                }
                title = truncated;
            } else {
                while display_width(&title) < screen_width {
                    title.push(' ');
                }
            }

            write!(stdout, "{}{}", crossterm::style::Attribute::Reverse, title);
            stdout.queue(Clear(crossterm::terminal::ClearType::UntilNewLine)).ok();
            write!(stdout, "{}", crossterm::style::Attribute::NoReverse);
            stdout.queue(MoveDown(1)).ok();
            self.cursor_offset_from_bottom -= 1;

            // Display the matched lines (truncated to screen width)
            for (idx, i) in (start..end).enumerate() {
                stdout.queue(MoveToColumn(0)).ok();

                let line = if i < self.lines.len() { &self.lines[i] } else { "" };

                // Truncate line to screen width to prevent overflow
                let line_display_width = display_width(line);
                let truncated_line = if line_display_width > screen_width {
                    let mut current_width = 0;
                    let mut truncated = String::new();
                    for ch in line.chars() {
                        let char_width = char_display_width(ch);
                        if current_width + char_width > screen_width {
                            break;
                        }
                        current_width += char_width;
                        truncated.push(ch);
                    }
                    truncated
                } else {
                    line.to_string()
                };

                write!(stdout, "{}", truncated_line).ok();
                stdout.queue(Clear(crossterm::terminal::ClearType::UntilNewLine)).ok();

                if idx < display_count - 1 {
                    stdout.queue(MoveDown(1)).ok();
                    self.cursor_offset_from_bottom -= 1;
                }
            }

            // Position cursor at the end of the current line
            let visual_line = self.cursor_line.saturating_sub(self.scroll_offset);
            let lines_from_bottom = display_count.saturating_sub(visual_line + 1);
            if lines_from_bottom > 0 {
                for _ in 0..lines_from_bottom {
                    stdout.queue(MoveUp(1)).ok();
                    self.cursor_offset_from_bottom += 1;
                }
            }

            // Position cursor at end of truncated line (or screen width if truncated)
            let current_line = self.lines.get(self.cursor_line).map(|s| s.as_str()).unwrap_or("");
            let line_display_width = display_width(current_line);
            let cursor_col = if line_display_width > screen_width {
                screen_width
            } else {
                line_display_width
            };
            stdout.queue(MoveToColumn(cursor_col as u16)).ok();

            stdout.flush().ok();
            return;
        }

        // In confirmation mode, display the confirmation prompt
        if self.mode == EditMode::Confirmation {
            // Move cursor to start of line (column 0)
            stdout.queue(MoveToColumn(0)).ok();

            // Clear the current line
            stdout.queue(Clear(crossterm::terminal::ClearType::CurrentLine)).ok();

            // Display the confirmation prompt with color
            use colored::Colorize;
            if let Some(ref prompt) = self.confirmation_prompt {
                let prompt_display = format!("{} [Y/n]: ", prompt);
                write!(stdout, "{}", prompt_display.bright_yellow()).ok();
            } else {
                write!(stdout, "{}", "Confirm [Y/n]: ".bright_yellow()).ok();
            }

            // Flush all queued commands
            stdout.flush().ok();
            return;
        }

        // Normal mode: display multiline input with scrolling
        {
	    // Calculate visible range based on scroll_offset and max_lines
            // check if we need to expand the editor
	    let start = self.scroll_offset;
	    let end = start + self.max_lines.min(self.lines.len());
	    let display_count = end - start;
	    // Expand editor height if needed (scroll terminal up to make room)
	    // We never decrease editor_height until Enter is pressed
	    let mut lines_to_add = 0;
	    if display_count > self.editor_height {
		lines_to_add = display_count - self.editor_height;
		for _ in 0..lines_to_add {
		    println!();  // Scroll terminal up by adding newlines
		}
		self.editor_height = display_count;
	    }
        }
        // Calculate visible range based on scroll_offset and max_lines, take 2
        // This time, we assume the whole editor height - so we take care
        // of things like line deletions correctly.
	let start = self.scroll_offset;
	let end = start + self.editor_height; // NOTE: without .min(self.lines.len());
	let display_count = end - start;
        let pr = format!("[{}]{}", std::process::id(), prompt);
        let prompt = &pr;


        // Get the prompt visible display width (excluding ANSI codes)
        let prompt_visible = strip_ansi_codes(prompt);
        let prompt_len = display_width(&prompt_visible);

        // Move to the top of the editor area
        // We need to move up from the current cursor position to the top
        // Calculate how many lines up we need to go
        let visual_line = self.cursor_line.saturating_sub(self.scroll_offset);
        let lines_to_move = self.editor_height.saturating_sub(self.cursor_offset_from_bottom); //  + lines_to_add;

        // Move up to the top line of the editor
        for _ in 0..lines_to_move {
            stdout.queue(MoveUp(1)).ok();
            self.cursor_offset_from_bottom += 1;
        }
        let draw_title_bar = true;
        if draw_title_bar {
	    stdout.queue(MoveUp(1)).ok();
	    self.cursor_offset_from_bottom += 1;
	    stdout.queue(MoveToColumn(0)).ok();
            let local_time = &Local::now().time().to_string()[0..8];
	    let idle_remaining = self.idle_command_time.map(|t| {
		let diff = t.signed_duration_since(chrono::Local::now());
		if diff.num_seconds() > 0 {
		    diff.num_seconds() as u32
		} else {
		    0
		}
	    }).unwrap_or(0);

            let req_count = request_counter::get_count();
            let tool_count = tool_counter::get_count();

            // Check if compaction is active
            let is_compaction_active = compaction_counter::is_active();

            // Create fixed-width status zone after clock
            let status_zone = if is_compaction_active {
                // Show compaction status during smart context compaction
                "COMPACT".to_string()
            } else if req_count == 0 && tool_count == 0 {
                if idle_remaining > 0 {
                    format!("IDLE({})", idle_remaining)
                } else {
                    "IDLE".to_string()
                }
            } else if tool_count > 0 && req_count == 0 {
                // Show current tool name if available
                let tool_name = if tool_counter::is_tool_active() {
                    if let Some(name) = tool_counter::get_current_tool_name() {
                        name
                    } else {
                        "UNKNOWN".to_string()
                    }
                } else {
                    "UNKNOWN".to_string()
                };
                // Add marquee after tool name with fixed total width
                let marquee = status_info::get_marquee_display();
                let base = format!("TOOL({})", tool_name);
                // Ensure total width is consistent: if marquee is empty, pad to 30 chars
                if marquee.is_empty() {
                    // Pad base to fixed width (35 chars = "TOOL(XXXXXXX)" + 30 spaces)
                    let mut result = base;
                    while result.chars().count() < 35 {
                        result.push(' ');
                    }
                    result
                } else {
                    // Marquee is already 30 chars, so we get base + " " + 30 chars
                    format!("{} {}", base, marquee)
                }
            } else if req_count > 0 && tool_count == 0 {
                format!("INFER({})", req_count)
            } else {
                "ERROR".to_string()
            };

            let mut title = format!(
                "User entry lines: {}, time: {} {} tok: {} queued: {} history: {} ctx: {} urgent: {} pid: {}",
                self.lines.len(),
                &local_time,
                status_zone,
                token_counter::get_count(),
                status_info::get_queued(),
                status_info::get_history(),
                status_info::get_context_bytes(),
                status_info::get_urgent(),
                status_info::get_pid(),
            );

            // Ensure title never exceeds screen width by truncating if needed
            let title_display_width = display_width(&title);
            if title_display_width > screen_width {
                // Truncate to fit screen width
                let mut current_width = 0;
                let mut truncated = String::new();
                for ch in title.chars() {
                    let char_width = char_display_width(ch);
                    if current_width + char_width > screen_width {
                        break;
                    }
                    current_width += char_width;
                    truncated.push(ch);
                }
                title = truncated;
            } else {
                // Pad to fill screen width exactly
                while display_width(&title) < screen_width {
                    title.push(' ');
                }
            }

	    write!(stdout, "{}{}", crossterm::style::Attribute::Reverse, title );
	    stdout.queue(Clear(crossterm::terminal::ClearType::UntilNewLine)).ok();
	    write!(stdout, "{}", crossterm::style::Attribute::NoReverse);
	    stdout.queue(MoveDown(1)).ok();
	    self.cursor_offset_from_bottom -= 1;
        }


        // Now we're at the top line, render each line
        for (idx, i) in (start..end).enumerate() {
            // Clear the line and move to column 0
            stdout.queue(MoveToColumn(0)).ok();
            // Display prompt only on the first line (line 0 of buffer)
            if i == 0 {
                write!(stdout, "{}", prompt).ok();
            }

            // Display the line content
            let line = if i < self.lines.len() { &self.lines[i] } else { "" };
            write!(stdout, "{}", line).ok();
            // stdout.queue(Clear(crossterm::terminal::ClearType::CurrentLine)).ok();
            stdout.queue(Clear(crossterm::terminal::ClearType::UntilNewLine)).ok();

            // Move to next line if not the last line
            if idx < display_count - 1 {
                stdout.queue(MoveDown(1)).ok();
                self.cursor_offset_from_bottom -= 1;
            }
        }

        // Now position cursor at the correct location
        // Move up from the bottom to the cursor line
        let visual_line = self.cursor_line.saturating_sub(self.scroll_offset);
        let lines_from_bottom = display_count.saturating_sub(visual_line + 1);
        if lines_from_bottom > 0 {
            // Move up the required number of lines
            for _ in 0..lines_from_bottom {
                stdout.queue(MoveUp(1)).ok();
                self.cursor_offset_from_bottom += 1;
            }
        }

        // Move to correct column - convert char position to display width
        let current_line = if self.cursor_line < self.lines.len() {
            &self.lines[self.cursor_line]
        } else {
            ""
        };
        // Calculate display width of text before cursor (cursor_col is char index)
        let text_before_cursor: String = current_line.chars().take(self.cursor_col).collect();
        let mut visual_col = display_width(&text_before_cursor);
        if visual_line == 0 {
            visual_col += prompt_len;
        }
        stdout.queue(MoveToColumn(visual_col as u16)).ok();

        // Flush all queued commands
        stdout.flush().ok();
    }

    /// Handles a key event from the terminal.
    ///
    /// This is the main dispatch function for key events. It maps
    /// keyboard input to the appropriate handler function.
    ///
    /// # Arguments
    ///
    /// * `key` - The key event to handle
    ///
    /// # Returns
    ///
    /// * `KeyResult` - The result of handling the key event
    fn handle_key_event(&mut self, key: KeyEvent) -> KeyResult {
        // Dispatch based on current mode
        match self.mode {
            EditMode::Normal => self.handle_normal_mode(key),
            EditMode::Search => self.handle_search_mode(key),
            EditMode::Confirmation => self.handle_confirmation_mode(key),
        }
    }

    /// Handles key events in normal editing mode.
    fn handle_normal_mode(&mut self, key: KeyEvent) -> KeyResult {
        match key.code {
            // Enter: Submit or insert newline
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::ALT) {
                    // Shift-Enter always inserts newline
                    if self.handle_newline() {
                        KeyResult::Redraw
                    } else {
                        KeyResult::Continue
                    }
                } else if self.is_at_end() {
                    // Regular Enter at end submits the input
                    let line = self.text();
                    // Add to history if not empty
                    if !line.trim().is_empty() {
                        self.add_history_entry(&line);
                    }
                    // Reset the line buffer
                    self.reset_input();
                    KeyResult::Return(ReadlineResult::Input(line))
                } else {
                    // Regular Enter not at end inserts newline
                    if self.handle_newline() {
                        KeyResult::Redraw
                    } else {
                        KeyResult::Continue
                    }
                }
            }

            // Ctrl-A: Move cursor to start of line
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.handle_home() {
                    KeyResult::Redraw
                } else {
                    KeyResult::Continue
                }
            }

            // Ctrl-C: Interrupt
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                KeyResult::Return(ReadlineResult::Interrupt)
            }

            // Ctrl-D: EOF (only if line is empty)
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.lines[self.cursor_line].is_empty() {
                    KeyResult::Return(ReadlineResult::Eof)
                } else {
                    KeyResult::Continue
                }
            }

            // Ctrl-E: Move cursor to end of line
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.handle_end() {
                    KeyResult::Redraw
                } else {
                    KeyResult::Continue
                }
            }

            // Ctrl-R: Enter reverse search mode
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.enter_search_mode();
                KeyResult::Redraw
            }

            // Backspace: Delete character before cursor
            KeyCode::Backspace => {
                if self.handle_backspace() {
                    KeyResult::Redraw
                } else {
                    KeyResult::Continue
                }
            }

            // Delete: Delete character at cursor
            KeyCode::Delete => {
                if self.handle_delete() {
                    KeyResult::Redraw
                } else {
                    KeyResult::Continue
                }
            }

            // Left arrow: Move cursor left (or by word with Ctrl)
            KeyCode::Left => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if self.handle_word_left() {
                        KeyResult::Redraw
                    } else {
                        KeyResult::Continue
                    }
                } else if self.handle_left() {
                    KeyResult::Redraw
                } else {
                    KeyResult::Continue
                }
            }

            // Right arrow: Move cursor right (or by word with Ctrl)
            KeyCode::Right => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if self.handle_word_right() {
                        KeyResult::Redraw
                    } else {
                        KeyResult::Continue
                    }
                } else if self.handle_right() {
                    KeyResult::Redraw
                } else {
                    KeyResult::Continue
                }
            }

            // Up arrow: Navigate history up OR move cursor up (if not in history mode)
            KeyCode::Up => {
                // If in history mode, navigate history
                if self.history_index.is_some() || (self.lines.len() == 1 && self.lines[0].len() == 0) {
                    if self.history_up() {
                        KeyResult::Redraw
                    } else {
                        KeyResult::Continue
                    }
                } else if self.cursor_line > 0 {
                    // Not in history mode: move cursor up one line
                    self.cursor_line -= 1;
                    // Adjust cursor_col to fit within the line above
                    let line_len = self.lines[self.cursor_line].chars().count();
                    if self.cursor_col > line_len {
                        self.cursor_col = line_len;
                    }
                    self.update_scroll_offset();
                    KeyResult::Redraw
                } else {
                    KeyResult::Continue
                }
            }

            // Down arrow: Navigate history down OR move cursor down (if not in history mode)
            KeyCode::Down => {
                // If in history mode, navigate history
                if self.history_index.is_some() {
                    if self.history_down() {
                        KeyResult::Redraw
                    } else {
                        KeyResult::Continue
                    }
                } else if self.cursor_line < self.lines.len() - 1 {
                    // Not in history mode: move cursor down one line
                    self.cursor_line += 1;
                    // Adjust cursor_col to fit within the line below
                    let line_len = self.lines[self.cursor_line].chars().count();
                    if self.cursor_col > line_len {
                        self.cursor_col = line_len;
                    }
                    self.update_scroll_offset();
                    KeyResult::Redraw
                } else {
                    KeyResult::Continue
                }
            }

            // Home: Move cursor to start
            KeyCode::Home => {
                if self.handle_home() {
                    KeyResult::Redraw
                } else {
                    KeyResult::Continue
                }
            }

            // End: Move cursor to end
            KeyCode::End => {
                if self.handle_end() {
                    KeyResult::Redraw
                } else {
                    KeyResult::Continue
                }
            }

            // Ctrl-K: Kill to end of line
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.kill_to_end() {
                    KeyResult::Redraw
                } else {
                    KeyResult::Continue
                }
            }

            // Ctrl-U: Kill to start of line
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.kill_to_start() {
                    KeyResult::Redraw
                } else {
                    KeyResult::Continue
                }
            }

            // Ctrl-W: Kill word to left
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.kill_word_left() {
                    KeyResult::Redraw
                } else {
                    KeyResult::Continue
                }
            }

            // Alt-D: Kill word to right
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::ALT) => {
                if self.kill_word_right() {
                    KeyResult::Redraw
                } else {
                    KeyResult::Continue
                }
            }

            // Ctrl-Y: Yank last killed text
            KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.yank() {
                    KeyResult::Redraw
                } else {
                    KeyResult::Continue
                }
            }

            // Alt-B: Move cursor left by word
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::ALT) => {
                if self.handle_word_left() {
                    KeyResult::Redraw
                } else {
                    KeyResult::Continue
                }
            }

            // Alt-F: Move cursor right by word
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::ALT) => {
                if self.handle_word_right() {
                    KeyResult::Redraw
                } else {
                    KeyResult::Continue
                }
            }

            // Regular character: Insert at cursor
            KeyCode::Char(c) => {
                self.handle_char(c);
                KeyResult::Redraw
            }

            // Ignore other keys (Tab, Esc, F-keys, etc.)
            _ => KeyResult::Continue,
        }
    }

    /// Handles key events in reverse search mode.
    fn handle_search_mode(&mut self, key: KeyEvent) -> KeyResult {
        match key.code {
            // Enter: Accept current match and exit search mode
            KeyCode::Enter => {
                // Exit search mode (keeping the matched line)
                self.mode = EditMode::Normal;
                KeyResult::Redraw
            }

            // Ctrl-C or Ctrl-G: Exit search mode, restore original line
            KeyCode::Char('c') | KeyCode::Char('g')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.exit_search_mode();
                KeyResult::Redraw
            }

            // Escape: Exit search mode, restore original line
            KeyCode::Esc => {
                self.exit_search_mode();
                KeyResult::Redraw
            }

            // Ctrl-R: Cycle to next match
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cycle_search_match();
                KeyResult::Redraw
            }

            // Backspace: Delete character from search pattern
            KeyCode::Backspace => {
                if !self.search_pattern.is_empty() {
                    self.search_pattern.pop();
                    self.update_search();
                    KeyResult::Redraw
                } else {
                    KeyResult::Continue
                }
            }

            // Regular character: Add to search pattern
            KeyCode::Char(c) => {
                self.search_pattern.push(c);
                self.update_search();
                KeyResult::Redraw
            }

            // Ignore other keys in search mode
            _ => KeyResult::Continue,
        }
    }

    /// Handles key events in confirmation mode.
    ///
    /// In confirmation mode, the user is prompted with a y/n question.
    /// Keys are handled to return the appropriate confirmation response.
    fn handle_confirmation_mode(&mut self, key: KeyEvent) -> KeyResult {
        use MspcMessage;

        // Get the confirmation ID if this is a tool confirmation
        let confirmation_id = self.confirmation_id.clone();

        match key.code {
            // y or Y: Yes
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.exit_confirmation_mode();
                // Return ToolConfirmationResponse if we have an ID, otherwise regular ConfirmationResponse
                if let Some(id) = confirmation_id {
                    KeyResult::Return(ReadlineResult::Signal(
                        MspcMessage::ToolConfirmationResponse {
                            approved: true,
                            reason: None,
                            confirmation_id: id,
                        }
                    ))
                } else {
                    KeyResult::Return(ReadlineResult::Signal(
                        MspcMessage::ConfirmationResponse(true, None)
                    ))
                }
            }
            // n or N: No
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.exit_confirmation_mode();
                if let Some(id) = confirmation_id {
                    KeyResult::Return(ReadlineResult::Signal(
                        MspcMessage::ToolConfirmationResponse {
                            approved: false,
                            reason: Some("User denied".to_string()),
                            confirmation_id: id,
                        }
                    ))
                } else {
                    KeyResult::Return(ReadlineResult::Signal(
                        MspcMessage::ConfirmationResponse(false, Some("User denied".to_string()))
                    ))
                }
            }
            // Enter: Treat as "yes" (default for convenience)
            KeyCode::Enter => {
                self.exit_confirmation_mode();
                if let Some(id) = confirmation_id {
                    KeyResult::Return(ReadlineResult::Signal(
                        MspcMessage::ToolConfirmationResponse {
                            approved: true,
                            reason: None,
                            confirmation_id: id,
                        }
                    ))
                } else {
                    KeyResult::Return(ReadlineResult::Signal(
                        MspcMessage::ConfirmationResponse(true, None)
                    ))
                }
            }
            // Escape or Ctrl-C: Cancel/No
            KeyCode::Esc | KeyCode::Char('c')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.exit_confirmation_mode();
                if let Some(id) = confirmation_id {
                    KeyResult::Return(ReadlineResult::Signal(
                        MspcMessage::ToolConfirmationResponse {
                            approved: false,
                            reason: Some("Cancelled".to_string()),
                            confirmation_id: id,
                        }
                    ))
                } else {
                    KeyResult::Return(ReadlineResult::Signal(
                        MspcMessage::ConfirmationResponse(false, Some("Cancelled".to_string()))
                    ))
                }
            }
            // Ignore other keys in confirmation mode
            _ => KeyResult::Continue,
        }
    }

    /// Reads a line of input from the user.
    ///
    /// This is the main readline loop. It displays a prompt, reads keyboard
    /// input, handles editing keys, and returns when the user submits the line.
    ///
    /// The loop polls for events with a 100ms timeout, allowing for future
    /// integration with MPSC signal checking.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The prompt string to display (e.g., "> ")
    ///
    /// # Returns
    ///
    /// * `ReadlineResult` - The result of the readline operation
    ///
    /// # Example
    ///
    /// ```ignore
    /// use apchat_vty::{Readline, ReadlineResult};
    ///
    /// let mut readline = Readline::new().unwrap();
    ///
    /// match readline.readline("> ", None, None) {
    ///     Ok(ReadlineResult::Input(line)) => println!("You entered: {}", line),
    ///     Ok(ReadlineResult::Eof) => println!("End of file"),
    ///     Ok(ReadlineResult::Interrupt) => println!("Interrupted"),
    ///     Ok(ReadlineResult::Signal(msg)) => println!("Signal received: {:?}", msg),
    ///     Err(e) => eprintln!("Error: {}", e),
    /// }
    /// ```
    ///
    /// # Arguments
    ///
    /// * `prompt` - The prompt string to display
    /// * `mspc_receiver` - Optional mutable reference to tokio MPSC receiver
    /// * `readline_receiver` - Optional broadcast receiver for TextOutput messages from ReadlineDestination
    /// * `idle_config` - Optional idle timeout configuration
    pub fn readline(
        &mut self,
        prompt: &str,
        mut mspc_receiver: Option<&mut tokio::sync::mpsc::Receiver<MspcMessage>>,
        mut readline_receiver: Option<&mut tokio::sync::broadcast::Receiver<apchat_mspc::output::TextOutput>>,
        idle_config: Option<IdleConfig>,
    ) -> io::Result<ReadlineResult> {
        // Start with a mutable prompt that can be updated via RefreshPrompt signals
        let mut current_prompt = prompt.to_string();

        // Calculate and store the visible prompt width for auto-wrapping
        // IMPORTANT: The prompt displayed includes a [PID] prefix that's added in redraw()
        // We need to account for this when calculating the available width
        let full_prompt = format!("[{}]{}", std::process::id(), current_prompt);
        let prompt_visible = strip_ansi_codes(&full_prompt);
        self.prompt_width = display_width(&prompt_visible);

        // Display the initial prompt
        self.redraw(&current_prompt);

        // Initialize idle timeout state
        self.idle_command = idle_config.as_ref().map(|c| c.input_text.clone());
        self.idle_period_secs = idle_config.as_ref().map(|c| c.timeout_secs);
        self.idle_command_time = idle_config.map(|c| {
            chrono::Local::now() + chrono::Duration::seconds(c.timeout_secs as i64)
        });

        // Check if we need to load a saved state file (from --load flag)
        if let Some(ref load_filename) = self.load_filename {
            print_heart_yellow(&format!("📂 Attempting to load state from: {}", load_filename), true);
            // Return the load command to be processed by the REPL
            return Ok(ReadlineResult::Input(format!("/load {}", load_filename)));
        }

        let mut curr_output_data = format!("");
        let COUNTDOWN = 2; // 2 * 50ms = 100ms redraw interval
        let mut counter = COUNTDOWN;

        // Main event loop
        loop {
            // Poll for events with 50ms timeout
            // This allows for MPSC signal checking with lower latency
            if poll(Duration::from_millis(50))? {
                // Read the event
                let event = read()?;

                // Handle different event types
                match event {
                    Event::Resize(_width, _height) => {
                        // Clear the screen width cache on terminal resize
                        self.clear_screen_width_cache();
                        self.redraw(&current_prompt);
                    }
                    Event::Key(key) => {
                        // Reset idle timer on any key press
                        if let Some(period) = self.idle_period_secs {
                            if period > 0 {
                                self.idle_command_time = Some(
                                    chrono::Local::now() + chrono::Duration::seconds(period as i64)
                                );
                            }
                        }
                        
                        match self.handle_key_event(key) {
                            KeyResult::Continue => {}
                            KeyResult::Redraw => {
                                self.redraw(&current_prompt);
                            }
                            KeyResult::Return(result) => {
                                // Redraw to clear the line and redraw the title bar before returning
                                self.redraw(&current_prompt);
                                return Ok(result);
                            }
                        }
                    }
                    Event::Paste(content) => {
                        // Handle paste events from bracketed paste mode
                        if self.handle_paste(content) {
                            self.redraw(&current_prompt);
                        }
                    }
                    _ => {
                        // Ignore other events (mouse, resize, focus, etc.)
                    }
                }
            }
            counter -= 1;
            if counter == 0 {
                self.redraw(&current_prompt);
                counter = COUNTDOWN;
            }

            // Check idle timeout (only if no active requests, queued tools, or running tools)
            if let Some(injection_time) = self.idle_command_time {
                if chrono::Local::now() >= injection_time 
                    && request_counter::get_count() == 0 
                    && status_info::get_queued() == 0
                    && !tool_counter::is_tool_active() {
                    // Inject the idle command
                    if let Some(ref cmd) = self.idle_command {
                        // If command starts with "@", treat remainder as filename to read
                        let input_to_inject = if cmd.starts_with('@') {
                            let filename = &cmd[1..];
                            if let Ok(contents) = std::fs::read_to_string(filename) {
                                contents
                            } else {
                                format!("Error: Could not read file '{}'", filename)
                            }
                        } else {
                            cmd.clone()
                        };
                        return Ok(ReadlineResult::Input(input_to_inject));
                    }
                }
            }

            // Timeout occurred - check MPSC signals if receiver provided
            if let Some(ref mut receiver) = mspc_receiver {
                // Drain all queued messages without blocking
                while let Ok(msg) = receiver.try_recv() {
                    match &msg {
                        MspcMessage::ConfirmationRequest(prompt, _) => {
                            // Enter confirmation mode and continue the loop
                            self.enter_confirmation_mode(prompt.clone(), None);
                            self.redraw(prompt);
                            continue;
                        }
                        MspcMessage::ToolConfirmationRequest { content, confirmation_id } => {
                            // Enter confirmation mode with the confirmation ID
                            self.enter_confirmation_mode(content.clone(), Some(confirmation_id.clone()));
                            self.redraw(content);
                            continue;
                        }
                        MspcMessage::EmojiText { emoji, content, newline } => {
                            // Issue 137: Handle EmojiText by saving cursor, clearing line, printing emoji text, and restoring
                            let mut stdout = std::io::stdout();
                            // Save cursor position
                            let _ = self.cursor();
                            // Clear the current line
                            stdout.queue(MoveToColumn(0)).ok();
                            stdout.queue(Clear(crossterm::terminal::ClearType::CurrentLine)).ok();
                            // Print emoji text
                            if !emoji.is_empty() && !content.is_empty() {
                                if *newline {
                                    writeln!(stdout, "{} {}", emoji, content).ok();
                                } else {
                                    write!(stdout, "{} {}", emoji, content).ok();
                                }
                            } else if !content.is_empty() {
                                if *newline {
                                    writeln!(stdout, "{}", content).ok();
                                } else {
                                    write!(stdout, "{}", content).ok();
                                }
                            }
                            stdout.flush().ok();
                            // Restore cursor position and redraw prompt
                            self.redraw(&current_prompt);
                            continue;
                        }
                        MspcMessage::RefreshPrompt(new_prompt) => {
                            // Update the current prompt and redraw (e.g., after model switch)
                            current_prompt = new_prompt.clone();
                            self.redraw(&current_prompt);
                            continue;
                        }
                        _ => {
                            // For other messages, clear the line and return
                            let mut stdout = std::io::stdout();
                            stdout.queue(MoveToColumn(0)).ok();
                            stdout
                                .queue(Clear(crossterm::terminal::ClearType::CurrentLine))
                                .ok();
                            stdout.flush().ok();
                            return Ok(ReadlineResult::Signal(msg));
                        }
                    }
                }
            }

            // Timeout occurred - check ReadlineDestination messages if receiver provided
            if let Some(ref mut receiver) = readline_receiver {
                // Drain all queued messages without blocking
                while let Ok(text_output) = receiver.try_recv() {
                    // Reset idle timer on any output
                    if let Some(period) = self.idle_period_secs {
                        if period > 0 {
                            self.idle_command_time = Some(
                                chrono::Local::now() + chrono::Duration::seconds(period as i64)
                            );
                        }
                    }
                    
                    // Print emoji text directly to stdout
                    let emoji = &text_output.emoji;
                    let content = &text_output.content;
                    let newline = text_output.newline;
                    let mut stdout = std::io::stdout();
                    // Save cursor position
                    let _ = self.cursor();
                    // Clear the current line
                    //stdout.queue(MoveToColumn(0)).ok();
                    //stdout.queue(Clear(crossterm::terminal::ClearType::CurrentLine)).ok();

                    // Get terminal width from cache (default to 80 if unavailable)
                    let term_width = self.get_screen_width() as usize;

                    /* Split the output into lines, and scroll up as needed */
                    let lines: Vec<String> = content.split('\n').map(String::from).collect();
                    if lines.len() > 0 {
                        for (i, ref line) in lines.iter().enumerate() {
                            // Calculate current display width with emoji prefix
                            let current_width = if curr_output_data.is_empty() {
                                display_width(emoji) + 1  // emoji + space
                            } else {
                                display_width(&curr_output_data)
                            };

                            let line_width = display_width(line);

                            // Check if adding this content would wrap
                            if !curr_output_data.is_empty() && current_width + line_width > term_width {
                                // Would wrap - flush current data first with newline
                                let lines_up = self.editor_height.saturating_sub(self.cursor_offset_from_bottom) as u16 + 2;
                                scroll_insert_up(lines_up, &curr_output_data, true);
                                curr_output_data = format!("");
                            }

                            curr_output_data.push_str(&line);
                            let lines_up = self.editor_height.saturating_sub(self.cursor_offset_from_bottom) as u16 + 2;
                            if i < lines.len() - 1 {
                              scroll_insert_up(lines_up, &curr_output_data, true);
                              curr_output_data =  format!("");
                            } else {
                                scroll_insert_up(lines_up, &curr_output_data, newline);
                                if newline {
                                    curr_output_data = format!("");
                                }
                            }
                        }
                    }
                    stdout.flush().ok();
                    // Restore cursor position and redraw prompt
                    self.redraw(&current_prompt);
                    continue;
                }
            }
        }
    }
}

impl Readline {
    /// Manually restore terminal settings.
    ///
    /// This method can be called explicitly to restore terminal settings
    /// before the struct is dropped. This is useful when the Readline
    /// instance is stored in a static variable that never gets dropped.
    ///
    /// # Returns
    ///
    /// * `io::Result<()>` - Ok if successful, Err otherwise
    pub fn restore_terminal(&self) -> io::Result<()> {
        if let Some(ref original) = self.original_termios {
            restore_terminal_settings(original)
        } else {
            Ok(()) // No original settings to restore
        }
    }
}

impl Drop for Readline {
    /// Disables raw mode when the `Readline` struct is dropped.
    ///
    /// This ensures terminal mode is properly restored even if panic occurs.
    fn drop(&mut self) {
        // Restore original terminal settings
        if let Some(ref original) = self.original_termios {
            if let Err(e) = restore_terminal_settings(original) {
                eprintln!("Warning: Failed to restore terminal settings: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi_codes_basic() {
        let input = "\x1b[31mRed Text\x1b[0m";
        let output = strip_ansi_codes(input);
        assert_eq!(output, "Red Text");
    }

    #[test]
    fn test_strip_ansi_codes_bold() {
        let input = "\x1b[1mBold Text\x1b[0m";
        let output = strip_ansi_codes(input);
        assert_eq!(output, "Bold Text");
    }

    #[test]
    fn test_strip_ansi_codes_multiple() {
        let input = "\x1b[35m\x1b[1m[Model (name)]\x1b[0m \x1b[32m\x1b[1mYou:\x1b[0m ";
        let output = strip_ansi_codes(input);
        assert_eq!(output, "[Model (name)] You: ");
        assert_eq!(output.chars().count(), 20);
        assert_eq!(input.chars().count(), 46);
    }

    #[test]
    fn test_strip_ansi_codes_no_ansi() {
        let input = "Plain text";
        let output = strip_ansi_codes(input);
        assert_eq!(output, "Plain text");
    }

    #[test]
    fn test_strip_ansi_codes_empty() {
        let input = "";
        let output = strip_ansi_codes(input);
        assert_eq!(output, "");
    }

    #[test]
    fn test_strip_ansi_codes_incomplete_sequence() {
        // Incomplete sequence should be left as-is
        let input = "Text\x1b[31"; // Missing the terminating 'm'
        let output = strip_ansi_codes(input);
        assert_eq!(output, "Text");
    }
    use super::*;

    /// Ensure stdin is backed by a PTY so `tcgetattr(STDIN_FILENO)` succeeds in tests.
    fn ensure_pty_stdin() {
        use std::sync::Once;
        static PTY_INIT: Once = Once::new();
        PTY_INIT.call_once(|| {
            unsafe {
                let mut master: libc::c_int = 0;
                let mut slave: libc::c_int = 0;
                let ret = libc::openpty(
                    &mut master, &mut slave,
                    std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(),
                );
                assert_eq!(ret, 0, "openpty() failed");
                libc::dup2(slave, libc::STDIN_FILENO);
                libc::close(slave);
            }
        });
    }

    /// Helper function to create a Readline instance for testing.
    ///
    /// Allocates a PTY if stdin is not a terminal, then creates the Readline.
    fn create_test_readline() -> io::Result<Readline> {
        ensure_pty_stdin();
        Readline::new()
    }

    #[test]

    fn test_readline_creation() {
        let readline = create_test_readline().expect("Failed to create Readline");

        // Verify initial state
        assert_eq!(readline.line(), "");
        assert_eq!(readline.cursor(), 0);
        assert!(readline.is_raw_mode_enabled());
    }

    #[test]

    fn test_raw_mode_enabled_on_creation() {
        let readline = create_test_readline().expect("Failed to create Readline");

        // Verify raw mode is enabled after creation
        assert!(readline.is_raw_mode_enabled());
        // Note: crossterm's is_raw_mode_enabled() won't work since we're using termios directly
        // The important thing is that our Readline tracks the state correctly
    }

    #[test]

    fn test_raw_mode_disabled_on_drop() {
        // Test with Readline struct
        {
            let readline = create_test_readline().expect("Failed to create Readline");
            assert!(readline.is_raw_mode_enabled());
            // readline goes out of scope here, Drop is called
        }

        // Test passes if we got here without panicking
        // The Drop implementation ensures cleanup is attempted
        // Note: We can't reliably test terminal state in test environments
        // because tests share terminal state
    }

    #[test]

    fn test_multiple_readline_instances() {
        // Skip this test if we can't create a readline instance
        // (e.g., in CI environments without a TTY)
        let readline1 = match create_test_readline() {
            Ok(r) => r,
            Err(_) => return,
        };

        // Creating a second instance should work
        let readline2 = match create_test_readline() {
            Ok(r) => r,
            Err(_) => return,
        };

        // Both instances should exist without panicking
        assert_eq!(readline1.line(), "");
        assert_eq!(readline2.line(), "");

        // Drop first instance
        drop(readline1);

        // Drop second instance
        drop(readline2);

        // Test passes if we got here without panicking
        // The Drop implementation ensures proper cleanup
    }

    #[test]

    fn test_initial_state() {
        let readline = create_test_readline().expect("Failed to create Readline");

        // Verify the initial buffer is empty
        assert_eq!(readline.line(), "");

        // Verify cursor is at the start
        assert_eq!(readline.cursor(), 0);

        // Verify raw mode is enabled (we have saved termios)
        assert!(readline.is_raw_mode_enabled());

        // Verify history is empty
        assert_eq!(readline.get_history_entries().len(), 0);
        assert!(readline.history_index.is_none());
    }

    #[test]

    fn test_add_history_entry() {
        let mut readline = create_test_readline().expect("Failed to create Readline");

        // Add first entry
        readline.add_history_entry("first command");
        assert_eq!(readline.get_history_entries().len(), 1);
        assert_eq!(readline.get_history_entries()[0], "first command");

        // Add second entry
        readline.add_history_entry("second command");
        assert_eq!(readline.get_history_entries().len(), 2);
        assert_eq!(readline.get_history_entries()[1], "second command");
    }

    #[test]

    fn test_add_history_empty_lines() {
        let mut readline = create_test_readline().expect("Failed to create Readline");

        // Empty lines should not be added
        readline.add_history_entry("");
        assert_eq!(readline.get_history_entries().len(), 0);

        // Whitespace-only lines should not be added
        readline.add_history_entry("   ");
        assert_eq!(readline.get_history_entries().len(), 0);

        // Valid command should be added
        readline.add_history_entry("valid command");
        assert_eq!(readline.get_history_entries().len(), 1);
    }

    #[test]

    fn test_add_history_consecutive_duplicates() {
        let mut readline = create_test_readline().expect("Failed to create Readline");

        // Add command
        readline.add_history_entry("same command");
        assert_eq!(readline.get_history_entries().len(), 1);

        // Add same command again - should not be added
        readline.add_history_entry("same command");
        assert_eq!(readline.get_history_entries().len(), 1);

        // Add different command - should be added
        readline.add_history_entry("different command");
        assert_eq!(readline.get_history_entries().len(), 2);

        // Add first command again - should be added (not consecutive)
        readline.add_history_entry("same command");
        assert_eq!(readline.get_history_entries().len(), 3);
    }

    #[test]

    fn test_history_up_navigation() {
        let mut readline = create_test_readline().expect("Failed to create Readline");

        // Add some history
        readline.add_history_entry("command 1");
        readline.add_history_entry("command 2");
        readline.add_history_entry("command 3");

        // Start typing something
        readline.set_line("new input");
        readline.set_cursor(9, None);

        // Navigate up - should go to most recent
        assert!(readline.history_up());
        assert_eq!(readline.line(), "command 3");
        assert_eq!(readline.cursor(), 9);
        assert_eq!(readline.saved_lines.get(0).map(|s| s.as_str()), Some("new input"));

        // Navigate up again
        assert!(readline.history_up());
        assert_eq!(readline.line(), "command 2");

        // Navigate up again
        assert!(readline.history_up());
        assert_eq!(readline.line(), "command 1");

        // Try to navigate up past oldest - should stay at oldest
        assert!(!readline.history_up());
        assert_eq!(readline.line(), "command 1");
    }

    #[test]

    fn test_history_down_navigation() {
        let mut readline = create_test_readline().expect("Failed to create Readline");

        // Add some history
        readline.add_history_entry("command 1");
        readline.add_history_entry("command 2");

        // Navigate to oldest
        readline.history_up();
        readline.history_up();
        assert_eq!(readline.line(), "command 1");

        // Navigate down - should go to newer entry
        assert!(readline.history_down());
        assert_eq!(readline.line(), "command 2");

        // Navigate down again - should restore saved line (empty in this case)
        assert!(readline.history_down());
        assert_eq!(readline.line(), "");
        assert!(readline.history_index.is_none());

        // Navigate down again - should do nothing
        assert!(!readline.history_down());
        assert_eq!(readline.line(), "");
    }

    #[test]

    fn test_history_navigation_with_saved_line() {
        let mut readline = create_test_readline().expect("Failed to create Readline");

        // Add history
        readline.add_history_entry("old command");

        // Start typing
        readline.set_line("typing something");
        readline.set_cursor(17, None);

        // Navigate up - should save current line
        assert!(readline.history_up());
        assert_eq!(readline.saved_lines.get(0).map(|s| s.as_str()), Some("typing something"));
        assert_eq!(readline.line(), "old command");

        // Navigate down - should restore saved line
        assert!(readline.history_down());
        assert_eq!(readline.line(), "typing something");
        // Cursor should be at end of restored line (16, not 17)
        assert_eq!(readline.cursor(), 16);
        assert!(readline.history_index.is_none());
    }

    #[test]

    fn test_history_navigation_empty_history() {
        let mut readline = create_test_readline().expect("Failed to create Readline");

        // Try to navigate with no history
        assert!(!readline.history_up());
        assert!(!readline.history_down());

        // Verify state unchanged
        assert_eq!(readline.line(), "");
        assert!(readline.history_index.is_none());
    }

    #[test]

    fn test_exit_history_navigation() {
        let mut readline = create_test_readline().expect("Failed to create Readline");

        // Add history
        readline.add_history_entry("command 1");

        // Navigate to history
        readline.history_up();
        assert_eq!(readline.line(), "command 1");
        assert!(readline.history_index.is_some());

        // Exit history navigation
        readline.exit_history_navigation();
        assert!(readline.history_index.is_none());
        assert!(readline.saved_lines.is_empty());
    }

    #[test]

    fn test_history_boundary_conditions() {
        let mut readline = create_test_readline().expect("Failed to create Readline");

        // Add single history entry
        readline.add_history_entry("only command");

        // Navigate up from current line
        assert!(readline.history_up());
        assert_eq!(readline.line(), "only command");

        // Try to go past oldest
        assert!(!readline.history_up());
        assert_eq!(readline.line(), "only command");

        // Navigate back to current line
        assert!(readline.history_down());
        assert_eq!(readline.line(), "");

        // Try to go past newest
        assert!(!readline.history_down());
        assert_eq!(readline.line(), "");
    }

    #[test]

    fn test_handle_char() {
        let mut readline = create_test_readline().expect("Failed to create Readline");

        // Insert characters at end
        assert!(readline.handle_char('h'));
        assert_eq!(readline.line(), "h");
        assert_eq!(readline.cursor(), 1);

        assert!(readline.handle_char('i'));
        assert_eq!(readline.line(), "hi");
        assert_eq!(readline.cursor(), 2);

        // Insert in middle
        readline.set_cursor(1, None);
        assert!(readline.handle_char('e'));
        assert_eq!(readline.line(), "hei");
        assert_eq!(readline.cursor(), 2);

        // Insert Unicode character
        readline.set_cursor(3, None);
        assert!(readline.handle_char('😀'));
        assert_eq!(readline.line(), "hei😀");
        assert_eq!(readline.cursor(), 4);
    }

    #[test]

    fn test_handle_char_exits_history_navigation() {
        let mut readline = create_test_readline().expect("Failed to create Readline");
        readline.add_history_entry("old");

        // Navigate to history
        readline.history_up();
        assert_eq!(readline.line(), "old");
        assert!(readline.history_index.is_some());

        // Insert char - should append to preserved history entry (bug fix)
        readline.handle_char('x');
        assert_eq!(readline.line(), "oldx");  // Fixed: preserves history entry
        assert!(readline.history_index.is_none());
    }

    #[test]

    fn test_handle_char_edits_history_in_middle() {
        let mut readline = create_test_readline().expect("Failed to create Readline");
        readline.add_history_entry("hello");

        // Navigate to history
        readline.history_up();
        assert_eq!(readline.line(), "hello");
        assert!(readline.history_index.is_some());

        // Move cursor to middle
        readline.set_cursor(2, None);
        assert_eq!(readline.cursor(), 2);

        // Insert char - should insert 'X' at cursor position (fix for editing bug)
        readline.handle_char('X');
        assert_eq!(readline.line(), "heXllo");  // Fixed: preserves history entry and edits it
        assert_eq!(readline.cursor(), 3);
        assert!(readline.history_index.is_none());
    }

    #[test]

    fn test_handle_backspace() {
        let mut readline = create_test_readline().expect("Failed to create Readline");
        readline.set_line("hello");
        readline.set_cursor(5, None);

        // Delete last character
        assert!(readline.handle_backspace());
        assert_eq!(readline.line(), "hell");
        assert_eq!(readline.cursor(), 4);

        // Delete another
        assert!(readline.handle_backspace());
        assert_eq!(readline.line(), "hel");
        assert_eq!(readline.cursor(), 3);

        // Delete in middle
        readline.set_cursor(2, None);
        assert!(readline.handle_backspace());
        assert_eq!(readline.line(), "hl");
        assert_eq!(readline.cursor(), 1);

        // Try to delete at start
        readline.set_cursor(0, None);
        assert!(!readline.handle_backspace());
        assert_eq!(readline.line(), "hl");
        assert_eq!(readline.cursor(), 0);
    }

    #[test]

    fn test_handle_backspace_exits_history_navigation() {
        let mut readline = create_test_readline().expect("Failed to create Readline");
        readline.add_history_entry("old");

        // Navigate to history
        readline.history_up();
        assert_eq!(readline.line(), "old");

        // Backspace - should exit history navigation and delete
        assert!(readline.handle_backspace());
        assert_eq!(readline.line(), "ol");
        assert!(readline.history_index.is_none());
    }

    #[test]

    fn test_handle_delete() {
        let mut readline = create_test_readline().expect("Failed to create Readline");
        readline.set_line("hello");
        readline.set_cursor(1, None);

        // Delete character at cursor
        assert!(readline.handle_delete());
        assert_eq!(readline.line(), "hllo");
        assert_eq!(readline.cursor(), 1);

        // Delete another
        assert!(readline.handle_delete());
        assert_eq!(readline.line(), "hlo");
        assert_eq!(readline.cursor(), 1);

        // Move to end
        readline.set_cursor(3, None);

        // Try to delete at end
        assert!(!readline.handle_delete());
        assert_eq!(readline.line(), "hlo");
        assert_eq!(readline.cursor(), 3);
    }

    #[test]

    fn test_handle_delete_exits_history_navigation() {
        let mut readline = create_test_readline().expect("Failed to create Readline");
        readline.add_history_entry("old");

        // Navigate to history
        readline.history_up();
        assert_eq!(readline.line(), "old");
        readline.set_cursor(1, None);

        // Delete - should exit history navigation
        assert!(readline.handle_delete());
        assert_eq!(readline.line(), "od");
        assert!(readline.history_index.is_none());
    }

    #[test]

    fn test_handle_left() {
        let mut readline = create_test_readline().expect("Failed to create Readline");
        readline.set_line("hi");
        readline.set_cursor(2, None);

        // Move left
        assert!(readline.handle_left());
        assert_eq!(readline.cursor(), 1);

        // Move left again
        assert!(readline.handle_left());
        assert_eq!(readline.cursor(), 0);

        // Try to move past start
        assert!(!readline.handle_left());
        assert_eq!(readline.cursor(), 0);
    }

    #[test]

    fn test_handle_right() {
        let mut readline = create_test_readline().expect("Failed to create Readline");
        readline.set_line("hi");
        readline.set_cursor(0, None);

        // Move right
        assert!(readline.handle_right());
        assert_eq!(readline.cursor(), 1);

        // Move right again
        assert!(readline.handle_right());
        assert_eq!(readline.cursor(), 2);

        // Try to move past end
        assert!(!readline.handle_right());
        assert_eq!(readline.cursor(), 2);
    }

    #[test]

    fn test_handle_home() {
        let mut readline = create_test_readline().expect("Failed to create Readline");
        readline.set_line("hello");
        readline.set_cursor(5, None);

        // Move to start
        assert!(readline.handle_home());
        assert_eq!(readline.cursor(), 0);

        // Already at start
        assert!(!readline.handle_home());
        assert_eq!(readline.cursor(), 0);
    }

    #[test]

    fn test_handle_end() {
        let mut readline = create_test_readline().expect("Failed to create Readline");
        readline.set_line("hello");
        readline.set_cursor(0, None);

        // Move to end
        assert!(readline.handle_end());
        assert_eq!(readline.cursor(), 5);

        // Already at end
        assert!(!readline.handle_end());
        assert_eq!(readline.cursor(), 5);
    }

    #[test]

    fn test_key_handlers_with_empty_line() {
        let mut readline = create_test_readline().expect("Failed to create Readline");

        // All operations on empty line should be safe
        assert!(!readline.handle_backspace());
        assert!(!readline.handle_delete());
        assert!(!readline.handle_left());
        assert!(!readline.handle_right());
        assert!(!readline.handle_home());
        assert!(!readline.handle_end());

        // But we can still insert
        assert!(readline.handle_char('x'));
        assert_eq!(readline.line(), "x");
    }

    #[test]

    fn test_unicode_handling() {
        let mut readline = create_test_readline().expect("Failed to create Readline");

        // Insert multi-byte Unicode characters
        readline.handle_char('😀');
        readline.handle_char('🎉');

        assert_eq!(readline.line(), "😀🎉");
        assert_eq!(readline.cursor(), 2);

        // Backspace should remove whole characters
        readline.handle_backspace();
        assert_eq!(readline.line(), "😀");
        assert_eq!(readline.cursor(), 1);
    }

    #[test]

    fn test_split_line_if_needed_no_split_short_line() {
        let mut readline = create_test_readline().expect("Failed to create Readline");

        // Short line should not split
        readline.set_line("hello");
        readline.cursor_line = 0;
        readline.cursor_col = 5;

        // No split should occur for short lines
        let old_len = readline.lines.len();
        readline.split_line_if_needed();
        assert_eq!(readline.lines.len(), old_len);
        assert_eq!(readline.lines[0], "hello");
    }

    #[test]

    fn test_split_line_if_needed_word_boundary() {
        let mut readline = create_test_readline().expect("Failed to create Readline");

        // Set a reasonable prompt width (simulating what would be calculated)
        // Actual prompt format: "[12345]User: " = 13 chars (PID + brackets + text)
        readline.prompt_width = 20; // Slightly larger to be safe

        // Create an extremely long line that will definitely split (200+ chars)
        // This should exceed even large terminal widths minus prompt width
        let long_line = "This is a very long line that definitely exceeds the terminal width and should wrap at some point soon because we are adding way more text than any reasonable terminal can display on a single line without wrapping the text into multiple lines";
        readline.set_line(long_line);
        readline.cursor_line = 0;
        readline.cursor_col = long_line.len();

        // Split should occur
        readline.split_line_if_needed();

        // Verify the line was split
        assert!(readline.lines.len() > 1, "Line should have been split into multiple lines");

        // First line should be truncated at word boundary
        assert!(readline.lines[0].len() < long_line.len(), "First line should be shorter than original");

        // Second line should contain the remainder
        assert!(!readline.lines[1].is_empty(), "Second line should contain remainder");
    }

    #[test]

    fn test_split_line_if_needed_forced_break() {
        let mut readline = create_test_readline().expect("Failed to create Readline");

        // Set a reasonable prompt width
        readline.prompt_width = 20;

        // Create a line with no spaces (single long word) - 300 chars
        let long_word = "a".repeat(300);
        readline.set_line(&long_word);
        readline.cursor_line = 0;
        readline.cursor_col = long_word.len();

        // Split should occur with forced break (no word boundary)
        readline.split_line_if_needed();

        // Verify the line was split
        assert!(readline.lines.len() > 1, "Line should have been split even without word boundary");

        // Cursor should have moved forward from line 0 after split
        // The exact position depends on terminal width, but it should be
        // on a line > 0 (the word was split across multiple lines)
        assert!(readline.cursor_line >= 1, "Cursor should move past first line, got line {}", readline.cursor_line);
    }

    #[test]

    fn test_split_line_if_needed_subsequent_line() {
        let mut readline = create_test_readline().expect("Failed to create Readline");

        // Test splitting on a non-first line (no prompt to subtract)
        readline.lines = vec!["first line".to_string(), String::new()];
        readline.cursor_line = 1;
        readline.cursor_col = 0;

        // Add a very long line that should split
        let long_line = "This is another very long line that exceeds terminal width and should wrap appropriately here because we keep adding more and more text until it definitely exceeds whatever terminal width the test environment has";
        for ch in long_line.chars() {
            readline.handle_char(ch);
        }

        // Verify split occurred
        assert!(readline.lines.len() > 2, "Line should have been split");
    }

    #[test]

    fn test_split_line_preserves_scroll_offset() {
        let mut readline = create_test_readline().expect("Failed to create Readline");

        // Set up a multiline scenario
        readline.lines = vec!["line 1".to_string(), "line 2".to_string(), String::new()];
        readline.cursor_line = 2;
        readline.cursor_col = 0;

        // Add a very long line that causes split
        let long_line = "This is a long line that will cause scrolling and splitting in the editor by exceeding terminal width with lots and lots of text that wraps automatically";
        for ch in long_line.chars() {
            readline.handle_char(ch);
        }

        // Scroll offset should be updated to keep cursor visible
        // The cursor line should be within visible range
        let visible_start = readline.scroll_offset;
        let visible_end = readline.scroll_offset + readline.max_lines;
        assert!(readline.cursor_line >= visible_start, "Cursor line should be >= scroll offset");
        assert!(readline.cursor_line < visible_end, "Cursor line should be within visible range");
    }

    #[test]

    fn test_split_line_uses_prompt_width() {
        let mut readline = create_test_readline().expect("Failed to create Readline");

        // Set a large prompt width (e.g., 30 chars)
        readline.prompt_width = 30;

        // Add text to the first line
        let text = "This is a moderately long line";
        for ch in text.chars() {
            readline.handle_char(ch);
        }

        // Cursor should be on line 0, at the end of the text
        assert_eq!(readline.cursor_line, 0, "Cursor should be on first line");

        // Verify prompt_width is stored
        assert_eq!(readline.prompt_width, 30, "Prompt width should be stored correctly");
    }

    #[test]

    fn test_split_line_no_prompt_on_subsequent_lines() {
        let mut readline = create_test_readline().expect("Failed to create Readline");

        // Set a prompt width
        readline.prompt_width = 20;

        // Set up multiline with cursor on second line
        readline.lines = vec!["first line".to_string(), String::new()];
        readline.cursor_line = 1;
        readline.cursor_col = 0;

        // Add a very long line that should split
        // On subsequent lines, there's no prompt to subtract, so it should split
        // at the actual terminal width boundary
        let long_line = "This is another very long line that exceeds terminal width and should wrap appropriately here because we keep adding more and more text until it definitely exceeds whatever terminal width the test environment has";
        for ch in long_line.chars() {
            readline.handle_char(ch);
        }

        // Verify split occurred (cursor moved to line 2 or 3)
        assert!(readline.cursor_line >= 2, "Cursor should be on line 2 or higher after split");
    }
}
