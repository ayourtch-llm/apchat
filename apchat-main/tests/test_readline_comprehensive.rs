// Comprehensive tests for readline singleton behavior and fixes
// This file contains tests to verify:
// 1. Single instance behavior
// 2. Proper input handling
// 3. History management
// 4. Test coordination with atomic lock

#[cfg(test)]
mod readline_comprehensive_tests {
    use super::*;
    use apchat_vty::ReadlineInstance;
    use apchat_vty::instance::TestLock;
    
    /// Clear history by resetting input state
    fn clear_history() {
        // Clear history by using reset_input() which doesn't add to history
        if let Ok(mut guard) = apchat_vty::ReadlineInstance::get() {
            let rl = &mut *guard;
            rl.clear_history_for_tests_only();
        }
    }

    /// Test 1: Verify proper input handling - empty strings in history
    #[test]
    #[ignore = "Requires TTY environment - ReadlineInstance::get() fails without terminal"]
    fn test_empty_input_handling() {
        // Acquire test lock with RAII guard - releases automatically on drop
        let _lock = TestLock::acquire("test_empty_input_handling");
        
        // add_history doesn't check for empty strings, so this should just work
        apchat_vty::ReadlineInstance::add_history("").unwrap();
        // Lock is automatically released when _lock goes out of scope
    }

    /// Test 2: Verify history addition works
    #[test]
    #[ignore = "Requires TTY environment - ReadlineInstance::get() fails without terminal"]
    fn test_atomic_history_addition() {
        // Acquire test lock with RAII guard - releases automatically on drop
        let _lock = TestLock::acquire("test_atomic_history_addition");
        
        // Clear input to minimize test interdependencies
        clear_history();

        // Add multiple entries and verify they persist
        for i in 0..10 {
            apchat_vty::ReadlineInstance::add_history(&format!("test command {}", i)).unwrap();
        }
eprintln!("Added some entries");
        // Verify all entries were added
        let guard = apchat_vty::ReadlineInstance::get().unwrap();
        let rl = &*guard;

        // Should have at least the 10 entries we just added
        let count = rl.get_history_entries().len();
eprintln!("Added {} entries", count);
        assert!(count >= 10, "Expected at least 10 entries, got {}", count);
eprintln!("Gonna release the lock now");
        // Lock is automatically released when _lock goes out of scope
    }

    /// Test 3: Verify history persists across accesses
    #[test]
    #[ignore = "Requires TTY environment - ReadlineInstance::get() fails without terminal"]
    fn test_history_persistence() {
        // Acquire test lock with RAII guard - releases automatically on drop
        let _lock = TestLock::acquire("test_history_persistence");
        
        // Clear input first to minimize cross-test contamination
        clear_history();
        
        // Add some entries
        apchat_vty::ReadlineInstance::add_history("save test 1").unwrap();
        apchat_vty::ReadlineInstance::add_history("save test 2").unwrap();

        // Verify entries are still there
        let guard = apchat_vty::ReadlineInstance::get().unwrap();
        let rl = &*guard;
        let count = rl.get_history_entries().len();
        // Should have at least these 2 entries
        assert!(count >= 2, "Expected at least 2 entries, got {}", count);
        // Lock is automatically released when _lock goes out of scope
    }

    /// Test 4: Verify normal operation
    #[test]
    #[ignore = "Requires TTY environment - ReadlineInstance::get() fails without terminal"]
    fn test_normal_operation() {
        // Acquire test lock with RAII guard - releases automatically on drop
        let _lock = TestLock::acquire("test_normal_operation");
        
        apchat_vty::ReadlineInstance::add_history("normal operation").unwrap();
        
        let guard = apchat_vty::ReadlineInstance::get().unwrap();
        let rl = &*guard;
        assert!(!rl.get_history_entries().is_empty());
        // Lock is automatically released when _lock goes out of scope
    }

    /// Test 5: Verify history persists across multiple operations
    #[test]
    #[ignore = "Requires TTY environment - ReadlineInstance::get() fails without terminal"]
    fn test_history_persistence_multiple_operations() {
        // Acquire test lock with RAII guard - releases automatically on drop
        let _lock = TestLock::acquire("test_history_persistence_multiple_operations");
        
        // Clear history before starting
        clear_history();
        
        // Perform multiple add operations
        for i in 0..5 {
            if let Ok(mut guard) = apchat_vty::ReadlineInstance::get() {
                let rl = &mut *guard;
                rl.add_history_entry(&format!("operation {}", i));
            }
        }

        // Verify all entries are still present
        let guard = apchat_vty::ReadlineInstance::get().unwrap();
        let rl = &*guard;
        // We expect at least 5 entries from this test (5 operations)
        // Plus any entries from previous tests. 13 is valid (5 + accumulated).
        let count = rl.get_history_entries().len();
        assert!(count >= 5, "Expected at least 5 entries from this test, got {}", count);
        // Lock is automatically released when _lock goes out of scope
    }

    /// Test 6: Verify that history size is properly managed
    #[test]
    #[ignore = "Requires TTY environment - ReadlineInstance::get() fails without terminal"]
    fn test_history_size_management() {
        // Acquire test lock with RAII guard - releases automatically on drop
        let _lock = TestLock::acquire("test_history_size_management");
        
        clear_history();
 
        // Add entries sequentially
        for i in 0..100 {
            apchat_vty::ReadlineInstance::add_history(&format!("entry {}", i)).unwrap();
        }

        // Verify we can still access the history
        let guard = apchat_vty::ReadlineInstance::get().unwrap();
        let rl = &*guard;

        // Check that history length is correct - should be exactly 100 after reset
        let count = rl.get_history_entries().len();
        assert_eq!(count, 100, "Expected exactly 100 entries, got {}", count);
        // Lock is automatically released when _lock goes out of scope
    }
}
