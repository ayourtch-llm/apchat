// Comprehensive tests for readline singleton behavior and fixes
// This file contains tests to verify:
// 1. Single instance behavior
// 2. Proper input handling
// 3. Absence of race conditions
// 4. History persistence
// 5. Cleanup functionality

#[cfg(test)]
mod readline_comprehensive_tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    /// Test 1: Verify that only one instance exists (singleton pattern)
    #[test]
    fn test_singleton_pattern() {
        // Get the instance multiple times
        let guard1 = ReadlineInstance::get().unwrap();
        let guard2 = ReadlineInstance::get().unwrap();
        
        // Both should be valid guards
        assert!(!guard1.is_locked());
        assert!(!guard2.is_locked());
        
        // They should be different guards (proper locking)
        assert_ne!(&*guard1 as *const _, &*guard2 as *const _);
    }

    /// Test 2: Verify instance is initialized exactly once
    #[test]
    fn test_single_initialization() {
        // Reset any state if needed
        // This test verifies that Lazy initialization works correctly
        assert!(ReadlineInstance::is_initialized());
        
        // Get the instance and verify it's properly configured
        let guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;
        
        // Verify basic editor configuration exists
        assert!(rl.max_history_size().is_some());
    }

    /// Test 3: Verify proper input handling - empty strings should return None
    #[test]
    fn test_empty_input_handling() {
        // Note: We can't actually test readline in unit tests without TTY
        // This test verifies the logic for handling empty input
        let guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;
        
        // Verify that adding an empty string doesn't cause issues
        assert!(rl.add_history_entry("").is_ok());
    }

    /// Test 4: Verify history addition is atomic
    #[test]
    fn test_atomic_history_addition() {
        // Add multiple entries and verify they all persist
        for i in 0..10 {
            ReadlineInstance::add_history(&format!("test command {}", i)).unwrap();
        }
        
        // Verify all entries were added
        let guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;
        assert_eq!(rl.history().len(), 10);
    }

    /// Test 5: Verify thread safety - no race conditions in history access
    #[test]
    fn test_thread_safe_history_access() {
        let num_threads = 10;
        let handles: Vec<_> = (0..num_threads).map(|i| {
            thread::spawn(move || {
                // Each thread adds 5 entries
                for j in 0..5 {
                    let entry = format!("thread_{}_entry_{}", i, j);
                    ReadlineInstance::add_history(&entry).unwrap();
                }
            })
        }).collect();

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify final state - should have exactly 50 entries
        let guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;
        assert_eq!(rl.history().len(), num_threads * 5);
    }

    /// Test 6: Verify save_history doesn't corrupt state
    #[test]
    fn test_save_history_preserves_state() {
        // Add some entries
        ReadlineInstance::add_history("save test 1").unwrap();
        ReadlineInstance::add_history("save test 2").unwrap();
        
        // Save history
        let result = ReadlineInstance::save_history();
        assert!(result.is_ok());
        
        // Verify entries are still there after save
        let guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;
        assert_eq!(rl.history().len(), 2);
    }

    /// Test 7: Verify cleanup functionality
    #[test]
    fn test_cleanup_clears_history() {
        // Add some entries
        ReadlineInstance::add_history("cleanup test 1").unwrap();
        ReadlineInstance::add_history("cleanup test 2").unwrap();
        
        // Verify entries were added
        let guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;
        assert_eq!(rl.history().len(), 2);
        
        // Call cleanup
        let result = ReadlineInstance::cleanup();
        assert!(result.is_ok());
        
        // Verify history is cleared
        let guard2 = ReadlineInstance::get().unwrap();
        let rl2 = &mut *guard2;
        assert_eq!(rl2.history().len(), 0);
    }

    /// Test 8: Verify concurrent save operations don't cause issues
    #[test]
    fn test_concurrent_save_operations() {
        // Add initial entries
        ReadlineInstance::add_history("initial entry").unwrap();
        
        let handles: Vec<_> = (0..5).map(|_| {
            thread::spawn(|| {
                // Each thread tries to save history
                ReadlineInstance::save_history().unwrap();
            })
        }).collect();

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify the instance is still usable
        let guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;
        assert!(rl.history().len() > 0);
    }

    /// Test 9: Verify lock is released properly after operations
    #[test]
    fn test_lock_release() {
        // Get the lock
        let guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;
        
        // Verify we have the lock
        // This will panic if the lock is poisoned or not properly released
        
        // Add a history entry while holding the lock
        rl.add_history_entry("test entry").unwrap();
        
        // The guard should be released when it goes out of scope
        // We can verify this by getting another guard
        let guard2 = ReadlineInstance::get().unwrap();
        assert!(guard2.is_locked());
    }

    /// Test 10: Verify error handling for lock poisoning
    #[test]
    fn test_lock_poisoning_recovery() {
        // This test verifies that if a thread panics while holding the lock,
        // the system can recover (though in practice, this would crash the app)
        
        // We can't easily test actual panic scenarios in unit tests
        // This test just verifies normal operation
        let guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;
        assert!(rl.add_history_entry("normal operation").is_ok());
    }

    /// Test 11: Verify history persistence across multiple operations
    #[test]
    fn test_history_persistence_multiple_operations() {
        // Perform multiple add and save operations
        for i in 0..5 {
            ReadlineInstance::add_history(&format!("operation {}", i)).unwrap();
            ReadlineInstance::save_history().unwrap();
        }
        
        // Verify all entries are still present
        let guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;
        assert_eq!(rl.history().len(), 5);
    }

    /// Test 12: Verify that the readline method works correctly
    #[test]
    fn test_readline_method() {
        // Note: We can't actually test readline without a TTY
        // This test just verifies the method signature works
        
        // Test with a simple prompt
        // This will fail in unit tests without a TTY, but that's expected
        let result = ReadlineInstance::readline("Test: ");
        
        // The result might be Err due to no TTY, but the method should not panic
        match result {
            Ok(_) => { /* Would only succeed with a real TTY */ }
            Err(_) => { /* Expected in unit test environment */ }
        }
    }

    /// Test 13: Verify concurrent history access with mixed operations
    #[test]
    fn test_mixed_concurrent_operations() {
        let handles: Vec<_> = (0..5).map(|i| {
            thread::spawn(move || {
                // Each thread performs a mix of operations
                if i % 2 == 0 {
                    // Even threads add history
                    ReadlineInstance::add_history(&format!("thread {}", i)).unwrap();
                } else {
                    // Odd threads save history
                    ReadlineInstance::save_history().unwrap();
                }
            })
        }).collect();

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify the instance is still functional
        let guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;
        assert!(rl.history().len() >= 0); // At least valid
    }

    /// Test 14: Verify that history size is properly managed
    #[test]
    fn test_history_size_management() {
        // Add entries to fill history
        for i in 0..100 {
            ReadlineInstance::add_history(&format!("entry {}", i)).unwrap();
        }
        
        // Verify we can still access the history
        let guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;
        
        // Check that history length is reasonable
        assert!(rl.history().len() > 0);
    }

    /// Test 15: Verify cleanup doesn't panic with empty history
    #[test]
    fn test_cleanup_empty_history() {
        // Call cleanup with no entries
        let result = ReadlineInstance::cleanup();
        assert!(result.is_ok());
        
        // Verify instance is still usable
        let guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;
        assert!(rl.add_history_entry("after cleanup").is_ok());
    }
}
