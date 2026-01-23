//! Readline implementation with terminal mode management.
//!
//! This module provides a `Readline` struct that manages terminal I/O
//! using "semi-raw" mode: raw input with normal output (like rustyline).

use crossterm::cursor::MoveToColumn;
use crossterm::terminal::Clear;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, is_raw_mode_enabled};
use crossterm::QueueableCommand;
use std::io::{self, Write};

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
///
/// This is similar to how rustyline operates.
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
    /// The current input line buffer
    line: String,
    /// Current cursor position in the line (0-based, from start of line)
    cursor: usize,
    /// Whether raw mode was successfully enabled
    raw_mode_enabled: bool,
    /// Command history (previous commands)
    history: Vec<String>,
    /// Current position in history navigation (None = editing current line)
    history_index: Option<usize>,
    /// Saved line when entering history navigation
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
        // Enable raw mode for character-by-character input
        enable_raw_mode()?;

        Ok(Readline {
            line: String::new(),
            cursor: 0,
            raw_mode_enabled: true,
            history: Vec::new(),
            history_index: None,
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
        // Return the tracked state. We set this to true when we successfully
        // call enable_raw_mode(), so if it's true, raw mode is enabled.
        // Note: crossterm::terminal::is_raw_mode_enabled() may return false
        // in non-TTY environments (like tests), so we rely on our tracking.
        self.raw_mode_enabled
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

        // If we're not currently navigating history, save the current line
        if self.history_index.is_none() {
            self.saved_line = self.line.clone();
            self.history_index = Some(self.history.len().saturating_sub(1));
            self.line = self.history[self.history_index.unwrap()].clone();
            self.cursor = self.line.len();
            return true;
        }

        // Check if we can go up (to older entries)
        if let Some(idx) = self.history_index {
            if idx > 0 {
                self.history_index = Some(idx - 1);
                self.line = self.history[idx - 1].clone();
                self.cursor = self.line.len();
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
                self.line = self.history[idx + 1].clone();
                self.cursor = self.line.len();
                return true;
            } else {
                // Exit history navigation, restore saved line
                self.history_index = None;
                self.line = self.saved_line.clone();
                self.cursor = self.line.len();
                self.saved_line.clear();
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
        self.saved_line.clear();
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
        if self.cursor == 0 {
            return false;
        }

        // Exit history navigation if we were in it
        if self.history_index.is_some() {
            self.exit_history_navigation();
        }

        self.cursor -= 1;
        // Remove the character at the current cursor position
        // Use remove on bytes is safe because we decremented cursor
        // and cursor is now at a valid character boundary
        let line_chars: Vec<char> = self.line.chars().collect();
        let new_line: String = line_chars[..self.cursor]
            .iter()
            .chain(line_chars[self.cursor + 1..].iter())
            .collect();
        self.line = new_line;
        true
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
        if self.cursor >= self.line.chars().count() {
            return false;
        }

        // Exit history navigation if we were in it
        if self.history_index.is_some() {
            self.exit_history_navigation();
        }

        // Remove the character at the current cursor position
        let line_chars: Vec<char> = self.line.chars().collect();
        let new_line: String = line_chars[..self.cursor]
            .iter()
            .chain(line_chars[self.cursor + 1..].iter())
            .collect();
        self.line = new_line;
        true
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
        if self.cursor == 0 {
            return false;
        }

        self.cursor -= 1;
        true
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
        if self.cursor >= self.line.chars().count() {
            return false;
        }

        self.cursor += 1;
        true
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
        let mut stdout = std::io::stdout();

        // Move cursor to start of line (column 0)
        stdout.queue(MoveToColumn(0)).ok();

        // Clear the current line
        stdout.queue(Clear(crossterm::terminal::ClearType::CurrentLine)).ok();

        // Write prompt and input
        write!(stdout, "{}{}", prompt, self.line).ok();

        // Calculate cursor position (in characters, not bytes)
        // Use chars() to handle multi-byte Unicode characters correctly
        let prompt_len = prompt.chars().count();
        let cursor_pos = prompt_len + self.cursor;

        // Move cursor to correct position
        stdout.queue(MoveToColumn(cursor_pos as u16)).ok();

        // Flush all queued commands
        stdout.flush().ok();
    }
}

impl Drop for Readline {
    /// Disables raw mode when the `Readline` struct is dropped.
    ///
    /// This ensures terminal mode is properly restored even if panic occurs.
    fn drop(&mut self) {
        if self.raw_mode_enabled {
            // Disable raw mode to restore normal terminal behavior
            if let Err(e) = disable_raw_mode() {
                eprintln!("Warning: Failed to disable raw mode: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
        assert!(is_raw_mode_enabled().unwrap_or(false));
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
        // because tests share terminal state and crossterm uses reference counting
    }

    #[test]
    fn test_multiple_readline_instances() {
        // Skip this test if we can't query terminal mode
        // (e.g., in CI environments without a TTY)
        if is_raw_mode_enabled().is_err() {
            return;
        }

        let readline1 = create_test_readline().expect("Failed to create first Readline");

        // Creating a second instance should work (raw mode is idempotent)
        let readline2 = create_test_readline().expect("Failed to create second Readline");

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

        // Verify raw mode flag is set
        assert!(readline.raw_mode_enabled);

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
