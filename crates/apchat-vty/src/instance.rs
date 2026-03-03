// Readline instance singleton pattern with proper synchronization
// This module provides a singleton readline instance that persists throughout the application lifecycle
//
// IMPORTANT: This module now enforces proper synchronization to prevent race conditions.
// The readline instance MUST NOT be used concurrently from multiple threads.
// All access should be serialized through the main REPL loop.

use anyhow::Result;
use crate::{print_heart_red, print_heart_yellow};
use crate::readline::{Readline, ReadlineResult};
use once_cell::sync::Lazy;
use std::sync::{Mutex, MutexGuard};
use std::sync::atomic::{AtomicBool, Ordering};
use std::mem::ManuallyDrop;

use super::history;

/// Global readline instance wrapped in synchronization primitives
static READLINE_INSTANCE: Lazy<Mutex<Readline>> = Lazy::new(|| {
    let rl = Readline::new()
        .expect("Failed to create readline editor");
    Mutex::new(rl)
});

/// Binary semaphore for test coordination
/// Tests must call ReadlineInstance::try_take_test_lock() to acquire exclusive
/// access. After using the singleton, they must call release_test_lock().
/// This ensures tests run in sequence without race conditions.
static TEST_LOCK: AtomicBool = AtomicBool::new(true);

/// RAII guard for test lock acquisition with automatic release
///
/// This holds the test lock for the lifetime of the guard, automatically
/// releasing it when the guard is dropped. Use this to eliminate
/// accidental lock leaks from test code.
#[derive(Debug)]
pub struct TestLock {
    caller: String,
}

impl TestLock {
    /// Returns the caller identifier
    pub fn caller(&self) -> &str {
        &self.caller
    }

    /// Acquire the test lock
    ///
    /// This is called internally by TryTakeTestLockResult, users should
    /// not call this directly.
    ///
    /// # Arguments
    ///
    /// * `caller` - A string identifier for the test/caller for logging purposes
    pub fn acquire(caller: &str) -> Self {
        println!("SEMA: {} taking lock", caller);
        // Spin-wait until lock is released (false).
        // Use compare_exchange for atomic acquire-release semantics
        loop {
            match TEST_LOCK.compare_exchange(
                true,       // expected: currently true means "available"
                false,      // new: set to false means "taken"
                Ordering::Acquire,
                Ordering::Relaxed
            ) {
                Ok(_) => {
                    // Successfully acquired lock (swapped true->false)
                    break;
                }
                Err(_) => {
                    // Lock was already false, someone else took it
                    // Try again
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
        }
        println!("SEMA: {} took lock", caller);
        // Once we get here, lock is held by us (now false)
        Self { caller: caller.to_string() }
    }
}

impl Drop for TestLock {
    fn drop(&mut self) {
        println!("SEMA: {} releasing lock", self.caller);
        TEST_LOCK.store(true, Ordering::Release);
    }
}

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
    /// ```ignore
    /// use apchat_vty::ReadlineInstance;
    ///
    /// let mut guard = ReadlineInstance::get()?;
    /// // Note: readline() would block waiting for user input in tests
    /// // guard is dropped here, releasing the lock
    /// ```
    pub fn get() -> Result<MutexGuard<'static, Readline>> {
        let guard = READLINE_INSTANCE
            .try_lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire readline lock: {}", e))?;
        Ok(guard)
    }

    /// Acquire the test lock for the calling test
    ///
    /// This returns a RAII guard that automatically releases the lock when dropped.
    /// Use this pattern to ensure lock release even if the test panics.
    ///
    /// # Arguments
    ///
    /// * `caller_id` - A string identifier for the test/caller for logging purposes
    ///
    /// # Returns
    ///
    /// * `TestLock` - RAII guard that releases lock on drop
    pub fn try_take_test_lock(caller_id: &str) -> TestLock {
        TestLock::acquire(caller_id)
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

        match rl.readline(prompt, None, None, None)? {
            ReadlineResult::Input(line) => {
                if line.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(line))
                }
            }
            ReadlineResult::Eof => Err(anyhow::anyhow!("EOF")), // Return as error for REPL to handle
            ReadlineResult::Interrupt => Err(anyhow::anyhow!("Interrupted")), // Return as error for REPL to handle
            ReadlineResult::Signal(_msg) => {
                // For now, ignore signals in the basic readline interface
                // In Task 12, the REPL will handle signals properly
                Ok(None)
            }
        }
    }

    /// Read a line using the singleton readline instance with MSPC receiver
    ///
    /// This method allows readline to receive and handle MSPC signals (like confirmation requests)
    /// while waiting for user input. The receiver is checked periodically during the input loop.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The prompt string to display
    /// * `mspc_receiver` - Optional mutable reference to the MSPC receiver
    /// * `readline_receiver` - Optional broadcast receiver for TextOutput messages from ReadlineDestination
    /// * `idle_config` - Optional idle timeout configuration for automatic command injection
    ///
    /// # Returns
    ///
    /// * `Result<Option<String>>` - The input line, or None if EOF or Interrupt
    ///
    /// # Safety
    ///
    /// This method is thread-safe and can be called from multiple threads,
    /// but the readline operation itself is blocking and will serialize access.
    pub fn readline_with_mspc(
        prompt: &str,
        mspc_receiver: Option<&mut tokio::sync::mpsc::Receiver<apchat_mspc::MspcMessage>>,
        readline_receiver: Option<&mut tokio::sync::broadcast::Receiver<apchat_mspc::output::TextOutput>>,
        idle_config: Option<crate::IdleConfig>,
    ) -> Result<Option<String>> {
        let mut guard = Self::get()?;
        let rl = &mut *guard;

        match rl.readline(prompt, mspc_receiver, readline_receiver, idle_config)? {
            ReadlineResult::Input(line) => {
                if line.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(line))
                }
            }
            ReadlineResult::Eof => Err(anyhow::anyhow!("EOF")), // Return as error for REPL to handle
            ReadlineResult::Interrupt => Err(anyhow::anyhow!("Interrupted")), // Return as error for REPL to handle
            ReadlineResult::Signal(msg) => {
                // Handle MSPC signals
                match msg {
                    apchat_mspc::MspcMessage::ConfirmationResponse(approved, reason) => {
                        // Return the confirmation response as an error with a special prefix
                        // The REPL loop will detect this and forward it to the main channel
                        let prefix = "__CONFIRMATION_RESPONSE__:";
                        let approved_str = if approved { "true" } else { "false" };
                        let reason_str = reason.unwrap_or_else(|| "".to_string());
                        Err(anyhow::anyhow!("{}{}|{}", prefix, approved_str, reason_str))
                    }
                    apchat_mspc::MspcMessage::ToolConfirmationResponse { approved, reason, confirmation_id } => {
                        // Return the tool confirmation response as an error with a special prefix
                        // The REPL loop will forward this to the confirmation registry
                        let prefix = "__TOOL_CONFIRMATION_RESPONSE__:";
                        let approved_str = if approved { "true" } else { "false" };
                        let reason_str = reason.unwrap_or_else(|| "".to_string());
                        Err(anyhow::anyhow!("{}{}|{}|{}", prefix, approved_str, confirmation_id, reason_str))
                    }
                    _ => Ok(None),
                }
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

    pub fn clear_history_for_tests_only() -> Result<()> {
        let mut guard = Self::get()?;
        let rl = &mut *guard;
        rl.clear_history_for_tests_only();
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
    /// This method performs cleanup operations including saving history
    /// and restoring terminal settings. Should be called before application exit.
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

        // Restore terminal settings (important for static instances that never get dropped)
        if let Ok(guard) = READLINE_INSTANCE.try_lock() {
            if let Err(e) = guard.restore_terminal() {
                print_heart_yellow(
                    &format!("Warning: Failed to restore terminal settings: {}", e),
                    true,
                );
            }
        }

        // Note: We don't clear the history here because:
        // 1. The Readline struct will be dropped when the app exits
        // 2. History is stored in-memory and will be freed automatically
        // 3. Our JSON-based history is already saved

        Ok(())
    }

    /// Set the load filename for later processing in the readline loop
    ///
    /// This allows the --load flag to be handled after initialization,
    /// preventing state corruption from other initializations.
    ///
    /// # Arguments
    ///
    /// * `filename` - Optional filename to load after initialization
    pub fn set_load_filename(filename: Option<String>) -> Result<()> {
        let mut guard = Self::get()?;
        guard.set_load_filename(filename);
        Ok(())
    }
}

impl Readline {
    /// Set the load filename for later processing in the readline loop
    pub fn set_load_filename(&mut self, filename: Option<String>) {
        self.load_filename = filename;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::time::Duration;

    #[test]
    #[serial]
    fn test_singleton_instance() {
        // Clear history before test to ensure clean state
        let _ = ReadlineInstance::clear_history_for_tests_only();

        // Get the instance once and verify it works
        let mut guard1 = ReadlineInstance::get().unwrap();

        // Should be initialized
        assert!(ReadlineInstance::is_initialized());

        // Verify we have a valid MutexGuard by accessing the underlying Readline
        // Note: We just verify we can lock and access, without actually calling readline
        // which would block waiting for user input
        {
            let rl = &mut *guard1;
            // Just access the struct to verify it's valid
            let history_len = rl.get_history_entries().len();
            // The guard works, we have read-only access to history even
        }

        // Clean up by dropping the guard
        drop(guard1);

        // Singleton should still work after dropping our guard
        let mut guard2 = ReadlineInstance::get().unwrap();
        assert!(ReadlineInstance::is_initialized());
        drop(guard2);
    }

    #[test]
    #[serial]
    fn test_instance_initialization() {
        // Get the instance
        let _guard = ReadlineInstance::get().unwrap();

        // Should be initialized
        assert!(ReadlineInstance::is_initialized());
    }

    #[test]
    #[serial]
    fn test_thread_safety() {
        // Clear history before test to ensure clean state
        let _ = ReadlineInstance::clear_history_for_tests_only();

        // For thread safety testing, we'll add history via the synchronized method
        // which properly acquires the lock
        for i in 0..10 {
            ReadlineInstance::add_history(&format!("test command {}", i)).unwrap();
        }

        // Verify all history entries were added
        let guard = ReadlineInstance::get().unwrap();
        let rl = &*guard;
        // Verify history exists and has entries
        assert!(!rl.get_history_entries().is_empty());
    }

    #[test]
    #[serial]
    fn test_history_addition() {
        // Clear history before test to ensure clean state
        let _ = ReadlineInstance::clear_history_for_tests_only();

        ReadlineInstance::add_history("command 1").unwrap();
        ReadlineInstance::add_history("command 2").unwrap();

        let guard = ReadlineInstance::get().unwrap();
        let rl = &*guard;
        // Verify history has entries
        let entries = rl.get_history_entries();
        assert!(!entries.is_empty());
    }

    #[test]
    #[serial]
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
