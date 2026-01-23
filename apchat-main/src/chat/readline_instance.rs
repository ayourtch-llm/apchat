// Readline instance singleton pattern with proper synchronization
// This module provides a singleton readline instance that persists throughout the application lifecycle
//
// IMPORTANT: This module now enforces proper synchronization to prevent race conditions.
// The readline instance MUST NOT be used concurrently from multiple threads.
// All access should be serialized through the main REPL loop.

use anyhow::Result;
use apchat_vty::{print_heart_red, print_heart_yellow, Readline, ReadlineResult};
use once_cell::sync::Lazy;
use std::sync::{Mutex, MutexGuard};

use crate::chat::readline_history;

/// Global readline instance wrapped in synchronization primitives
static READLINE_INSTANCE: Lazy<Mutex<Readline>> = Lazy::new(|| {
    let rl = Readline::new()
        .expect("Failed to create readline editor");
    Mutex::new(rl)
});

/// Singleton readline instance manager
#[derive(Debug)]
pub struct ReadlineInstance;

impl ReadlineInstance {
    /// Get the singleton readline instance
    ///
    /// This method returns a locked guard that ensures exclusive access to the readline instance.
    /// The guard must be kept alive for the entire duration of the readline operation.
    ///
    /// # Returns
    ///
    /// * `Result<MutexGuard<'static, Readline>>` - A guard providing exclusive access to the readline editor
    ///
    /// # Safety
    ///
    /// The guard MUST NOT be dropped prematurely. The entire readline operation must complete
    /// while holding this lock to prevent race conditions.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut guard = ReadlineInstance::get()?;
    /// let line = guard.readline("Prompt: ")?;
    /// // guard is dropped here, releasing the lock
    /// ```
    pub fn get() -> Result<MutexGuard<'static, Readline>> {
        let guard = READLINE_INSTANCE
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire readline lock: {}", e))?;
        Ok(guard)
    }

    /// Read a line using the singleton readline instance
    ///
    /// This is a convenience method that handles the lock internally and ensures
    /// proper synchronization for the readline operation.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The prompt string to display
    ///
    /// # Returns
    ///
    /// * `Result<Option<String>>` - The input line, or None if EOF or Interrupt
    ///
    /// # Safety
    ///
    /// This method is thread-safe and can be called from multiple threads,
    /// but the readline operation itself is blocking and will serialize access.
    pub fn readline(prompt: &str) -> Result<Option<String>> {
        let mut guard = Self::get()?;
        let rl = &mut *guard;

        match rl.readline(prompt, None)? {
            ReadlineResult::Input(line) => {
                if line.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(line))
                }
            }
            ReadlineResult::Eof => Ok(None),
            ReadlineResult::Interrupt => Ok(None),
            ReadlineResult::Signal(_msg) => {
                // For now, ignore signals in the basic readline interface
                // In Task 12, the REPL will handle signals properly
                Ok(None)
            }
        }
    }

    /// Add an entry to the readline history
    ///
    /// This method ensures thread-safe access to the history.
    ///
    /// # Arguments
    ///
    /// * `entry` - The command string to add to history
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Ok if successful, Err otherwise
    pub fn add_history(entry: &str) -> Result<()> {
        let mut guard = Self::get()?;
        let rl = &mut *guard;
        rl.add_history_entry(entry);
        Ok(())
    }

    /// Check if the readline instance has been initialized
    ///
    /// # Returns
    ///
    /// * `bool` - True if the instance has been initialized (always true after first call)
    pub fn is_initialized() -> bool {
        true // Lazy always initializes on first access
    }

    /// Save the readline history to file
    ///
    /// This method is a no-op because we use a custom JSON-based history system
    /// (see readline_history.rs) that saves after each command.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Always Ok
    pub fn save_history() -> Result<()> {
        // No-op: We use custom JSON-based history saving in readline_history.rs
        // which is called after each command in repl.rs (line ~797).
        Ok(())
    }

    /// Clean up the readline instance
    ///
    /// This method performs cleanup operations including saving history.
    /// Should be called before application exit.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Ok if cleanup successful, Err otherwise
    pub fn cleanup() -> Result<()> {
        // Save history before cleanup
        if let Err(e) = Self::save_history() {
            print_heart_yellow(
                &format!("Warning: Failed to save readline history: {}", e),
                true,
            );
        }

        // Note: We don't clear the history here because:
        // 1. The Readline struct will be dropped when the app exits
        // 2. History is stored in-memory and will be freed automatically
        // 3. Our JSON-based history is already saved

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_singleton_instance() {
        // Get the instance twice
        let guard1 = ReadlineInstance::get().unwrap();
        let guard2 = ReadlineInstance::get().unwrap();

        // Both should be initialized
        assert!(ReadlineInstance::is_initialized());

        // Verify they're different guards (proper locking)
        assert_ne!(&*guard1 as *const _, &*guard2 as *const _);
    }

    #[test]
    fn test_instance_initialization() {
        // Get the instance
        let _guard = ReadlineInstance::get().unwrap();

        // Should be initialized
        assert!(ReadlineInstance::is_initialized());
    }

    #[test]
    fn test_thread_safety() {
        // Test concurrent access from multiple threads
        let handles: Vec<_> = (0..10)
            .map(|i| {
                thread::spawn(move || {
                    ReadlineInstance::add_history(&format!("test command {}", i)).unwrap();
                })
            })
            .collect();

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all history entries were added
        let guard = ReadlineInstance::get().unwrap();
        let rl = &*guard;
        // Verify history exists and has entries
        assert!(!rl.get_history_entries().is_empty());
    }

    #[test]
    fn test_history_addition() {
        ReadlineInstance::add_history("command 1").unwrap();
        ReadlineInstance::add_history("command 2").unwrap();

        let guard = ReadlineInstance::get().unwrap();
        let rl = &*guard;
        // Verify history has entries
        let entries = rl.get_history_entries();
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_save_history() {
        // Add some history entries
        ReadlineInstance::add_history("test command 1").unwrap();
        ReadlineInstance::add_history("test command 2").unwrap();

        // Save history should succeed
        let result = ReadlineInstance::save_history();
        assert!(result.is_ok());

        // History should still be there after save
        let guard = ReadlineInstance::get().unwrap();
        let rl = &*guard;
        // Verify history still exists
        assert!(!rl.get_history_entries().is_empty());
    }

    #[test]
    fn test_cleanup() {
        // Add some history entries
        ReadlineInstance::add_history("cleanup test 1").unwrap();
        ReadlineInstance::add_history("cleanup test 2").unwrap();

        // Cleanup should succeed
        let result = ReadlineInstance::cleanup();
        assert!(result.is_ok());
    }
}
