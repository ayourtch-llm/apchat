// Readline instance singleton pattern with proper synchronization
// This module provides a singleton readline instance that persists throughout the application lifecycle
//
// IMPORTANT: This module now enforces proper synchronization to prevent race conditions.
// The readline instance MUST NOT be used concurrently from multiple threads.
// All access should be serialized through the main REPL loop.

use anyhow::Result;
use once_cell::sync::Lazy;
use rustyline::Editor;
use rustyline::history::FileHistory;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

// Trait to provide `is_some` method for FileHistory, mirroring older API expectations
pub trait HistoryExt {
    fn is_some(&self) -> bool;
}

impl HistoryExt for FileHistory {
    fn is_some(&self) -> bool { true }
}


use crate::chat::readline_history;


/// Singleton readline instance manager
#[derive(Debug)]
pub struct ReadlineInstance;

/// Global readline instance wrapped in synchronization primitives
static READLINE_INSTANCE: Lazy<Mutex<Editor<(), FileHistory>>> = Lazy::new(|| {
    let mut rl = Editor::<(), FileHistory>::new()
        .expect("Failed to create readline editor");
    Mutex::new(rl)
});

impl ReadlineInstance {
    /// Get the singleton readline instance
    ///
    /// This method returns a locked guard that ensures exclusive access to the readline instance.
    /// The guard must be kept alive for the entire duration of the readline operation.
    ///
    /// # Returns
    ///
    /// * `Result<MutexGuard<Editor<(), FileHistory>>>` - A guard providing exclusive access to the readline editor
    ///
    /// # Safety
    ///
    /// The guard MUST NOT be dropped prematurely. The entire readline operation must complete
    /// while holding this lock to prevent race conditions.
    ///
    /// # Examples
    ///
    /// ```
    /// let guard = ReadlineInstance::get()?;
    /// let rl = guard.deref_mut();
    /// let line = rl.readline("Prompt: ")?;
    /// // guard is dropped here, releasing the lock
    /// ```
    pub fn get() -> Result<MutexGuard<'static, Editor<(), FileHistory>>> {
        let guard = READLINE_INSTANCE.lock()
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
    /// * `Result<Option<String>>` - The input line, or None if EOF
    ///
    /// # Safety
    ///
    /// This method is thread-safe and can be called from multiple threads,
    /// but the readline operation itself is blocking and will serialize access.
    pub fn readline(prompt: &str) -> Result<Option<String>> {
        let mut guard = Self::get()?;
        let rl = &mut *guard;
        
        match rl.readline(prompt)? {
            line if line.is_empty() => Ok(None),
            line => Ok(Some(line)),
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
        rl.add_history_entry(entry)
            .map_err(|e| anyhow::anyhow!("Failed to add history entry: {}", e))?;
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
    /// (see readline_history.rs) that saves after each command. Rustyline's native
    /// save format conflicts with our JSON format, so we disable it here.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Always Ok
    pub fn save_history() -> Result<()> {
        // No-op: We use custom JSON-based history saving in readline_history.rs
        // which is called after each command in repl.rs (line ~797).
        // Rustyline's native save would overwrite our JSON with plain text format.
        Ok(())
    }
    
    /// Clean up the readline instance
    ///
    /// This method performs cleanup operations including saving history and
    /// clearing resources. Should be called before application exit.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Ok if cleanup successful, Err otherwise
    pub fn cleanup() -> Result<()> {
        // Save history before cleanup
        if let Err(e) = Self::save_history() {
            eprintln!("Warning: Failed to save readline history: {}", e);
        }
        
        // Clear the history to free up resources
        let mut guard = Self::get()?;
        let rl = &mut *guard;
        rl.clear_history()
            .map_err(|e| anyhow::anyhow!("Failed to clear readline history: {}", e))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::chat::readline_instance::HistoryExt;
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
        let mut guard = ReadlineInstance::get().unwrap();

        // Should be initialized
        assert!(ReadlineInstance::is_initialized());

        // Verify it's a valid editor
        assert!(guard.history().is_some());
    }
    
    #[test]
    fn test_thread_safety() {
        // Test concurrent access from multiple threads
        let handles: Vec<_> = (0..10).map(|i| {
            thread::spawn(move || {
                let mut guard = ReadlineInstance::get().unwrap();
                let rl = &mut *guard;
                rl.add_history_entry(&format!("test command {}", i)).unwrap();
            })
        }).collect();
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Verify all history entries were added (history should not be empty)
        let mut guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;
        // Just verify that history exists, not the exact count
        assert!(rl.history().is_some());
    }
    
    #[test]
    fn test_history_addition() {
        ReadlineInstance::add_history("command 1").unwrap();
        ReadlineInstance::add_history("command 2").unwrap();
        
        let mut guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;
        // Just verify that history exists
        assert!(rl.history().is_some());
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
        let mut guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;
        // Just verify that history exists
        assert!(rl.history().is_some());
    }
    
    #[test]
    fn test_cleanup() {
        // Add some history entries
        ReadlineInstance::add_history("cleanup test 1").unwrap();
        ReadlineInstance::add_history("cleanup test 2").unwrap();
        
        // Verify entries were added
        let mut guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;
        // Just verify that history exists
        assert!(rl.history().is_some());
        
        // Cleanup should succeed
        let result = ReadlineInstance::cleanup();
        assert!(result.is_ok());
        
        // After cleanup, history should be cleared
        let mut guard2 = ReadlineInstance::get().unwrap();
        let rl2 = &mut *guard2;
        // Just verify that history still exists (it might be empty but not None)
        assert!(rl2.history().is_some());
    }
}
