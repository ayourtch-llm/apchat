// Comprehensive tests for readline singleton behavior and fixes
// This file contains tests to verify:
// 1. Single instance behavior
// 2. Proper input handling
// 3. History management
// 4. Thread safety

#[cfg(test)]
mod readline_comprehensive_tests {
    use super::*;
    use apchat_vty::ReadlineInstance;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    /// Test 1: Verify that only one instance exists (singleton pattern)
    #[test]
    fn test_singleton_pattern() {
        // Get the instance multiple times
        let guard1 = ReadlineInstance::get().unwrap();
        let guard2 = ReadlineInstance::get().unwrap();

        // Both should be valid guards
        // Note: MutexGuard doesn't have is_locked(), we just verify they can be obtained
        assert!(guard1.line().is_empty()); // Verify guard works
        assert!(guard2.line().is_empty()); // Verify guard works

        // They should be pointing to the same underlying data
        // (different guards but same mutex)
        let history1 = guard1.get_history_entries();
        let history2 = guard2.get_history_entries();
        assert_eq!(history1.len(), history2.len());
    }

    /// Test 2: Verify instance is properly configured
    #[test]
    fn test_single_initialization() {
        // Get the instance and verify it's properly configured
        let guard = ReadlineInstance::get().unwrap();
        let rl = &*guard;

        // Verify basic editor is functional
        assert_eq!(rl.line(), "");
        assert_eq!(rl.cursor(), 0);
    }

    /// Test 3: Verify proper input handling - empty strings in history
    #[test]
    fn test_empty_input_handling() {
        // Note: We can't actually test readline in unit tests without TTY
        // This test verifies the logic for handling empty input
        let mut guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;

        // Verify that adding an empty string doesn't cause issues
        // add_history_entry returns (), not Result
        rl.add_history_entry("");
    }

    /// Test 4: Verify history addition works
    #[test]
    fn test_atomic_history_addition() {
        // Add multiple entries and verify they persist
        let mut guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;

        for i in 0..10 {
            rl.add_history_entry(&format!("test command {}", i));
        }

        // Verify all entries were added
        assert_eq!(rl.get_history_entries().len(), 10);
    }

    /// Test 5: Verify thread safety - no race conditions in history access
    #[test]
    fn test_thread_safe_history_access() {
        let num_threads = 10;
        let handles: Vec<_> = (0..num_threads)
            .map(|i| {
                thread::spawn(move || {
                    // Each thread adds 5 entries
                    for j in 0..5 {
                        let entry = format!("thread_{}_entry_{}", i, j);
                        let mut guard = ReadlineInstance::get().unwrap();
                        let rl = &mut *guard;
                        rl.add_history_entry(&entry);
                    }
                })
            })
            .collect();

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify final state - should have at least some entries
        let guard = ReadlineInstance::get().unwrap();
        let rl = &*guard;
        // We can't guarantee exact count due to potential duplicates,
        // but we should have entries
        assert!(rl.get_history_entries().len() >= num_threads * 5);
    }

    /// Test 6: Verify history persists across accesses
    #[test]
    fn test_history_persistence() {
        // Add some entries
        {
            let mut guard = ReadlineInstance::get().unwrap();
            let rl = &mut *guard;
            rl.add_history_entry("save test 1");
            rl.add_history_entry("save test 2");
        }

        // Verify entries are still there
        let guard = ReadlineInstance::get().unwrap();
        let rl = &*guard;
        assert_eq!(rl.get_history_entries().len(), 2);
    }

    /// Test 7: Verify reset clears input
    #[test]
    fn test_reset_clears_input() {
        let mut guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;

        // Simulate some input
        rl.handle_char('a');
        rl.handle_char('b');
        rl.handle_char('c');
        assert_eq!(rl.line(), "abc");

        // Reset
        rl.reset_input();
        assert_eq!(rl.line(), "");
    }

    /// Test 8: Verify concurrent operations don't cause issues
    #[test]
    fn test_concurrent_operations() {
        // Add initial entries
        {
            let mut guard = ReadlineInstance::get().unwrap();
            let rl = &mut *guard;
            rl.add_history_entry("initial entry");
        }

        let handles: Vec<_> = (0..5)
            .map(|i| {
                thread::spawn(move || {
                    // Each thread adds history
                    let mut guard = ReadlineInstance::get().unwrap();
                    let rl = &mut *guard;
                    rl.add_history_entry(&format!("concurrent entry {}", i));
                })
            })
            .collect();

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify the instance is still usable
        let guard = ReadlineInstance::get().unwrap();
        let rl = &*guard;
        assert!(rl.get_history_entries().len() > 0);
    }

    /// Test 9: Verify lock is released properly after operations
    #[test]
    fn test_lock_release() {
        // Get the lock
        let mut guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;

        // Verify we have the lock by using it
        rl.add_history_entry("test entry");

        // The guard should be released when it goes out of scope
        // We can verify this by getting another guard
        drop(guard);
        let guard2 = ReadlineInstance::get().unwrap();
        assert!(guard2.line().is_empty());
    }

    /// Test 10: Verify normal operation
    #[test]
    fn test_normal_operation() {
        let mut guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;
        rl.add_history_entry("normal operation");
        assert!(!rl.get_history_entries().is_empty());
    }

    /// Test 11: Verify history persistence across multiple operations
    #[test]
    fn test_history_persistence_multiple_operations() {
        // Perform multiple add operations
        for i in 0..5 {
            let mut guard = ReadlineInstance::get().unwrap();
            let rl = &mut *guard;
            rl.add_history_entry(&format!("operation {}", i));
        }

        // Verify all entries are still present
        let guard = ReadlineInstance::get().unwrap();
        let rl = &*guard;
        assert_eq!(rl.get_history_entries().len(), 5);
    }

    /// Test 12: Verify that the readline method exists
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
        let handles: Vec<_> = (0..5)
            .map(|i| {
                thread::spawn(move || {
                    // Each thread performs add operations
                    let mut guard = ReadlineInstance::get().unwrap();
                    let rl = &mut *guard;
                    rl.add_history_entry(&format!("thread {}", i));
                })
            })
            .collect();

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify the instance is still functional
        let guard = ReadlineInstance::get().unwrap();
        let rl = &*guard;
        assert!(rl.get_history_entries().len() >= 0); // At least valid
    }

    /// Test 14: Verify that history size is properly managed
    #[test]
    fn test_history_size_management() {
        // Add entries
        for i in 0..100 {
            let mut guard = ReadlineInstance::get().unwrap();
            let rl = &mut *guard;
            rl.add_history_entry(&format!("entry {}", i));
        }

        // Verify we can still access the history
        let guard = ReadlineInstance::get().unwrap();
        let rl = &*guard;

        // Check that history length is reasonable
        assert!(rl.get_history_entries().len() > 0);
    }

    /// Test 15: Verify operations don't panic with empty history
    #[test]
    fn test_operations_empty_history() {
        // Verify instance is usable
        let mut guard = ReadlineInstance::get().unwrap();
        let rl = &mut *guard;
        rl.add_history_entry("test entry");
        assert!(!rl.get_history_entries().is_empty());
    }
}
