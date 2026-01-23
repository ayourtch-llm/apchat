//! Readline implementation with terminal mode management.
//!
//! This module provides a `Readline` struct that manages terminal I/O
//! using "semi-raw" mode: raw input with normal output.

use crossterm::cursor::{MoveTo, MoveToColumn};
use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::Clear;
use crossterm::QueueableCommand;
use std::io::{self, Write};
use std::time::Duration;

use apchat_mspc::MspcMessage;

// Termios imports for raw mode on stdin only
use libc::{tcsetattr, termios, ECHO, ICANON, ISIG, STDIN_FILENO, TCSANOW};

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
        
        // Set raw mode flags (clear ICANON, ECHO, ISIG)
        term.c_lflag &= !(ICANON | ECHO | ISIG);
        
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
/// ```text
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
    // Compatibility fields for transition (deprecated)
    #[deprecated(note = "Use lines[0] instead")]
    line: String,
    #[deprecated(note = "Use cursor_col instead")]
    cursor: usize,
    #[deprecated(note = "Use saved_lines instead")]
    saved_line: String,
}

impl Readline {
    /// Creates a new `Readline` instance and enables raw mode.
    ///
    /// # Errors
    ///
    /// Returns an error if raw mode cannot be enabled.
    ///
    /// # Example
    ///
    /// ```no_run
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
            // Compatibility fields
            line: String::new(),
            cursor: 0,
            saved_line: String::new(),
        })
    }

    /// Returns the current input line.
    ///
    /// # Example
    ///
    /// ```
    /// use apchat_vty::Readline;
    ///
    /// let readline = Readline::new().unwrap();
    /// assert_eq!(readline.line(), "");
    /// ```
    pub fn line(&self) -> &str {
        &self.line
    }

    /// Returns the current cursor position.
    ///
    /// # Example
    ///
    /// ```
    /// use apchat_vty::Readline;
    ///
    /// let readline = Readline::new().unwrap();
    /// assert_eq!(readline.cursor(), 0);
    /// ```
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Checks if raw mode is currently enabled.
    ///
    /// # Example
    ///
    /// ```no_run
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
    /// ```
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
    /// ```
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
            self.cursor_col = self.lines.last().map(|l| l.len()).unwrap_or(0);
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
                self.cursor_col = self.lines.last().map(|l| l.len()).unwrap_or(0);
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
    /// ```
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
                self.cursor_col = self.lines.last().map(|l| l.len()).unwrap_or(0);
                return true;
            } else {
                // Exit history navigation, restore saved multiline state
                self.history_index = None;
                self.lines = self.saved_lines.clone();
                // Restore cursor to end of last line
                self.cursor_line = self.lines.len().saturating_sub(1);
                self.cursor_col = self.lines.last().map(|l| l.len()).unwrap_or(0);
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
    /// ```
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
    /// ```
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
        self.history_index = None;
        // Restore the complete multiline state from saved_lines
        self.lines = self.saved_lines.clone();
        // Restore cursor to end of last line
        self.cursor_line = self.lines.len().saturating_sub(1);
        self.cursor_col = self.lines.last().map(|l| l.len()).unwrap_or(0);
        self.saved_lines.clear();
    }

    /// Enters reverse search mode (Ctrl-R).
    ///
    /// Saves the current line and cursor position, then switches to search mode.
    fn enter_search_mode(&mut self) {
        self.original_lines = self.lines.clone();
        self.original_cursor_line = self.cursor_line;
        self.original_cursor_col = self.cursor_col;
        // Also save to compatibility fields for now
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
            self.line = self.history[match_idx].clone();
            self.cursor = self.line.chars().count();
        } else {
            // No matches, clear the line
            self.line.clear();
            self.cursor = 0;
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
        self.line = self.history[match_idx].clone();
        self.cursor = self.line.chars().count();
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
    /// ```
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
        if self.history_index.is_some() {
            self.line.clear();
            self.cursor = 0;
            self.history_index = None;
            self.saved_line.clear();
        }

        // Insert character at cursor position
        // Need to convert character position to byte position
        let byte_pos = self.line.chars().take(self.cursor).map(|c| c.len_utf8()).sum();
        self.line.insert(byte_pos, c);
        self.cursor += 1;
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
    /// ```
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
    /// ```
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
    /// ```
    /// use apchat_vty::Readline;
    ///
    /// let mut readline = Readline::new().unwrap();
    /// readline.line = "hi".to_string();
    /// readline.cursor = 2;
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
    /// ```
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
    /// ```
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
        if self.cursor == 0 {
            return false;
        }

        self.cursor = 0;
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
    /// ```
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
        let line_len = self.line.chars().count();
        if self.cursor >= line_len {
            return false;
        }

        self.cursor = line_len;
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
        let line_len = self.line.chars().count();
        if self.cursor >= line_len {
            return false;
        }

        // Get text from cursor to end
        let byte_pos = self.line.chars().take(self.cursor).map(|c| c.len_utf8()).sum();
        let killed = self.line[byte_pos..].to_string();

        // Remove text from cursor to end
        self.line.truncate(byte_pos);

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
        if self.cursor == 0 {
            return false;
        }

        // Get text from start to cursor
        let byte_pos = self.line.chars().take(self.cursor).map(|c| c.len_utf8()).sum();
        let killed = self.line[..byte_pos].to_string();

        // Remove text from start to cursor
        self.line = self.line[byte_pos..].to_string();
        self.cursor = 0;

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
        let line_len = self.line.chars().count();
        if self.cursor >= line_len {
            return false;
        }

        // Find end of current word
        let chars: Vec<char> = self.line.chars().collect();
        let mut end = self.cursor;

        // Skip non-alphanumeric characters
        while end < line_len && !chars[end].is_alphanumeric() {
            end += 1;
        }

        // Skip alphanumeric characters (the word)
        while end < line_len && chars[end].is_alphanumeric() {
            end += 1;
        }

        if end == self.cursor {
            return false;
        }

        // Get the word to kill
        let start_byte = self.line.chars().take(self.cursor).map(|c| c.len_utf8()).sum();
        let end_byte = self.line.chars().take(end).map(|c| c.len_utf8()).sum();
        let killed = self.line[start_byte..end_byte].to_string();

        // Remove the word
        self.line = format!("{}{}", &self.line[..start_byte], &self.line[end_byte..]);

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
        if self.cursor == 0 {
            return false;
        }

        // Find start of previous word
        let chars: Vec<char> = self.line.chars().collect();
        let mut start = self.cursor;

        // Skip non-alphanumeric characters
        while start > 0 && !chars[start - 1].is_alphanumeric() {
            start -= 1;
        }

        // Skip alphanumeric characters (the word)
        while start > 0 && chars[start - 1].is_alphanumeric() {
            start -= 1;
        }

        if start == self.cursor {
            return false;
        }

        // Get the word to kill
        let start_byte = self.line.chars().take(start).map(|c| c.len_utf8()).sum();
        let end_byte = self.line.chars().take(self.cursor).map(|c| c.len_utf8()).sum();
        let killed = self.line[start_byte..end_byte].to_string();

        // Remove the word and update cursor
        self.line = format!("{}{}", &self.line[..start_byte], &self.line[end_byte..]);
        self.cursor = start;

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
        let byte_pos = self.line.chars().take(self.cursor).map(|c| c.len_utf8()).sum();
        self.line.insert_str(byte_pos, text);
        self.cursor += text.chars().count();
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

            // Remove the original line that we split
            self.lines.remove(self.cursor_line + pasted_lines.len() - 1);

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
        if self.cursor == 0 {
            return false;
        }

        let chars: Vec<char> = self.line.chars().collect();
        let mut new_pos = self.cursor;

        // Skip non-alphanumeric characters
        while new_pos > 0 && !chars[new_pos - 1].is_alphanumeric() {
            new_pos -= 1;
        }

        // Skip alphanumeric characters (the word)
        while new_pos > 0 && chars[new_pos - 1].is_alphanumeric() {
            new_pos -= 1;
        }

        self.cursor = new_pos;
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
        let line_len = self.line.chars().count();
        if self.cursor >= line_len {
            return false;
        }

        let chars: Vec<char> = self.line.chars().collect();
        let mut new_pos = self.cursor;

        // Skip alphanumeric characters (the current word)
        while new_pos < line_len && chars[new_pos].is_alphanumeric() {
            new_pos += 1;
        }

        // Skip non-alphanumeric characters
        while new_pos < line_len && !chars[new_pos].is_alphanumeric() {
            new_pos += 1;
        }

        self.cursor = new_pos;
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
        self.lines = vec![String::new()];
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
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
    /// ```no_run
    /// use apchat_vty::Readline;
    ///
    /// let mut readline = Readline::new().unwrap();
    /// readline.line = "hello".to_string();
    /// readline.cursor = 5;
    ///
    /// // Redraw with prompt
    /// readline.redraw("> ");
    /// ```
    pub fn redraw(&mut self, prompt: &str) {
        let stdout = &mut std::io::stdout();

        // In search mode, display the search interface on a single line
        if self.mode == EditMode::Search {
            // Move cursor to start of line (column 0)
            stdout.queue(MoveToColumn(0)).ok();

            // Clear the current line
            stdout.queue(Clear(crossterm::terminal::ClearType::CurrentLine)).ok();

            // Format: (reverse-i-search)`pattern': matched_command
            let matched_text = if self.search_matches.is_empty() {
                ""
            } else {
                // Use the multiline text joined with newlines
                &self.text()
            };
            write!(stdout, "(reverse-i-search)`{}': {}", self.search_pattern, matched_text).ok();

            // Move cursor to end of line (after the matched command)
            let display_len = format!("(reverse-i-search)`{}': {}", self.search_pattern, matched_text);
            stdout.queue(MoveToColumn(display_len.chars().count() as u16)).ok();

            // Flush all queued commands
            stdout.flush().ok();
            return;
        }

        // Normal mode: display multiline input with scrolling
        
        // Calculate visible range based on scroll_offset and max_lines
        let start = self.scroll_offset;
        let end = (start + self.max_lines).min(self.lines.len());

        // Get the prompt visible length (excluding ANSI codes)
        let prompt_visible = strip_ansi_codes(prompt);
        let prompt_len = prompt_visible.chars().count();

        // Clear and redraw each visible line
        for i in start..end {
            // Move to the beginning of this line (column 0, row i - start)
            stdout.queue(MoveTo(0, (i - start) as u16)).ok();

            // Clear the entire line
            stdout.queue(Clear(crossterm::terminal::ClearType::CurrentLine)).ok();

            // Display prompt only on the first line (line 0)
            if i == 0 {
                write!(stdout, "{}", prompt).ok();
            }

            // Display the line content
            write!(stdout, "{}", self.lines[i]).ok();
        }

        // Clear any remaining lines that might have content from before
        // (in case we had more lines displayed previously)
        for i in end..(start + self.max_lines) {
            stdout.queue(MoveTo(0, (i - start) as u16)).ok();
            stdout.queue(Clear(crossterm::terminal::ClearType::CurrentLine)).ok();
        }

        // Position cursor at the correct location
        // The visual line is cursor_line - scroll_offset (relative to where we started drawing)
        let visual_line = self.cursor_line.saturating_sub(self.scroll_offset);
        let mut visual_col = self.cursor_col;

        // Add prompt length to column if we're on the first line
        if visual_line == 0 {
            visual_col += prompt_len;
        }

        // Move cursor to the correct position
        stdout.queue(MoveTo(visual_col as u16, visual_line as u16)).ok();

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
        }
    }

    /// Handles key events in normal editing mode.
    fn handle_normal_mode(&mut self, key: KeyEvent) -> KeyResult {
        match key.code {
            // Enter: Submit or insert newline
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
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
                    self.line.clear();
                    self.cursor = 0;
                    self.history_index = None;
                    self.saved_line.clear();
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
                if self.line.is_empty() {
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
                if self.history_index.is_some() {
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
    /// ```no_run
    /// use apchat_vty::Readline;
    ///
    /// let mut readline = Readline::new().unwrap();
    ///
    /// match readline.readline("> ", None) {
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
    pub fn readline(
        &mut self,
        prompt: &str,
        mut mspc_receiver: Option<&mut tokio::sync::mpsc::Receiver<MspcMessage>>,
    ) -> io::Result<ReadlineResult> {
        // Display the initial prompt
        self.redraw(prompt);

        // Main event loop
        loop {
            // Poll for events with 100ms timeout
            // This allows for MPSC signal checking
            if poll(Duration::from_millis(100))? {
                // Read the event
                let event = read()?;

                // Handle different event types
                match event {
                    Event::Key(key) => {
                        match self.handle_key_event(key) {
                            KeyResult::Continue => {}
                            KeyResult::Redraw => {
                                self.redraw(prompt);
                            }
                            KeyResult::Return(result) => {
                                // Redraw to clear the line before returning
                                let mut stdout = std::io::stdout();
                                stdout.queue(MoveToColumn(0)).ok();
                                stdout
                                    .queue(Clear(crossterm::terminal::ClearType::CurrentLine))
                                    .ok();
                                stdout.flush().ok();
                                return Ok(result);
                            }
                        }
                    }
                    Event::Paste(content) => {
                        // Handle paste events from bracketed paste mode
                        if self.handle_paste(content) {
                            self.redraw(prompt);
                        }
                    }
                    _ => {
                        // Ignore other events (mouse, resize, focus, etc.)
                    }
                }
            }

            // Timeout occurred - check MPSC signals if receiver provided
            if let Some(ref mut receiver) = mspc_receiver {
                // Try to receive a message without blocking
                if let Ok(msg) = receiver.try_recv() {
                    // Clear the line before returning
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

    /// Helper function to create a Readline instance for testing.
    ///
    /// Note: This will temporarily put the terminal in raw mode during tests.
    fn create_test_readline() -> io::Result<Readline> {
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
        readline.line = "new input".to_string();
        readline.cursor = 9;

        // Navigate up - should go to most recent
        assert!(readline.history_up());
        assert_eq!(readline.line(), "command 3");
        assert_eq!(readline.cursor(), 9);
        assert_eq!(readline.saved_line, "new input");

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
        readline.line = "typing something".to_string();
        readline.cursor = 17;

        // Navigate up - should save current line
        assert!(readline.history_up());
        assert_eq!(readline.saved_line, "typing something");
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
        assert_eq!(readline.saved_line, "");
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
        readline.cursor = 1;
        assert!(readline.handle_char('e'));
        assert_eq!(readline.line(), "hei");
        assert_eq!(readline.cursor(), 2);

        // Insert Unicode character
        readline.cursor = 3;
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

        // Insert char - should exit history navigation
        readline.handle_char('x');
        assert_eq!(readline.line(), "x");
        assert!(readline.history_index.is_none());
    }

    #[test]
    fn test_handle_backspace() {
        let mut readline = create_test_readline().expect("Failed to create Readline");
        readline.line = "hello".to_string();
        readline.cursor = 5;

        // Delete last character
        assert!(readline.handle_backspace());
        assert_eq!(readline.line(), "hell");
        assert_eq!(readline.cursor(), 4);

        // Delete another
        assert!(readline.handle_backspace());
        assert_eq!(readline.line(), "hel");
        assert_eq!(readline.cursor(), 3);

        // Delete in middle
        readline.cursor = 2;
        assert!(readline.handle_backspace());
        assert_eq!(readline.line(), "hl");
        assert_eq!(readline.cursor(), 1);

        // Try to delete at start
        readline.cursor = 0;
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
        readline.line = "hello".to_string();
        readline.cursor = 1;

        // Delete character at cursor
        assert!(readline.handle_delete());
        assert_eq!(readline.line(), "hllo");
        assert_eq!(readline.cursor(), 1);

        // Delete another
        assert!(readline.handle_delete());
        assert_eq!(readline.line(), "hlo");
        assert_eq!(readline.cursor(), 1);

        // Move to end
        readline.cursor = 3;

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
        readline.cursor = 1;

        // Delete - should exit history navigation
        assert!(readline.handle_delete());
        assert_eq!(readline.line(), "od");
        assert!(readline.history_index.is_none());
    }

    #[test]
    fn test_handle_left() {
        let mut readline = create_test_readline().expect("Failed to create Readline");
        readline.line = "hi".to_string();
        readline.cursor = 2;

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
        readline.line = "hi".to_string();
        readline.cursor = 0;

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
        readline.line = "hello".to_string();
        readline.cursor = 5;

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
        readline.line = "hello".to_string();
        readline.cursor = 0;

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
}
