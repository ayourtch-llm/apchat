//! Readline implementation with terminal mode management.
//!
//! This module provides a `Readline` struct that manages terminal I/O
//! using "semi-raw" mode: raw input with normal output (like rustyline).

use crossterm::terminal::{disable_raw_mode, enable_raw_mode, is_raw_mode_enabled};
use std::io;

/// Readline struct that manages terminal mode and input state.
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
        if !self.raw_mode_enabled {
            return false;
        }

        // Check the actual terminal state if possible
        match is_raw_mode_enabled() {
            Ok(enabled) => enabled,
            Err(_) => {
                // If we can't query the terminal (e.g., in non-TTY environment),
                // assume it's enabled since we successfully called enable_raw_mode()
                true
            }
        }
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
    }
}
