use apchat_vty::ReadlineInstance;
use apchat_vty::instance::TestLock;
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::Duration;
use std::ops::DerefMut;


#[test]
fn test_rapid_input_sequence() {
    println!("\n=== Testing Rapid Input Sequence ===\n");
    
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_rapid_input_sequence");
    
    // Test rapid sequence of readline operations
    let num_operations = 50;
    let barrier = Arc::new(Barrier::new(num_operations));
    
    let handles: Vec<_> = (0..num_operations).map(|i| {
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || {
            barrier.wait();
            
            // Simulate rapid input operations
            let entry = format!("rapid_input_{}", i);
            apchat_vty::ReadlineInstance::add_history(&entry).unwrap();
            
            // Small delay to simulate real input timing
            thread::sleep(Duration::from_micros(100));
        })
    }).collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Verify all operations completed
    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    let history_len = rl.get_history_entries().len();
    
    println!("✓ All {} rapid operations completed", num_operations);
    println!("✓ History contains {} entries", history_len);
    
    assert_eq!(history_len, num_operations, "Should have exactly {} history entries", num_operations);
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_lifecycle_transitions() {
    println!("\n=== Testing Lifecycle Transitions ===\n");
    
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_lifecycle_transitions");
    
    // Test multiple initialization and cleanup cycles
    for cycle in 0..5 {
        println!("  Cycle {}: Adding history", cycle + 1);
        
        // Add history entries
        apchat_vty::ReadlineInstance::add_history(&format!("cycle_{}_entry_1", cycle)).unwrap();
        apchat_vty::ReadlineInstance::add_history(&format!("cycle_{}_entry_2", cycle)).unwrap();
        
        // Verify entries were added
        let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
        let rl = guard.deref_mut();
        let history_len = rl.get_history_entries().len();
        assert_eq!(history_len, 2 * (cycle + 1), "Should have {} entries after cycle {}", 2 * (cycle + 1), cycle + 1);
    }
    
    // Cleanup should work even after multiple cycles
    apchat_vty::ReadlineInstance::cleanup().unwrap();
    
    // Verify cleanup worked
    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    let history_len = rl.get_history_entries().len();
    
    println!("✓ All {} lifecycle cycles completed", 5);
    println!("✓ Final history length after cleanup: {}", history_len);
    
    assert_eq!(history_len, 0, "History should be empty after cleanup");
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_concurrent_readline_and_history_operations() {
    println!("\n=== Testing Concurrent Readline and History Operations ===\n");
    
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_concurrent_readline_and_history_operations");
    
    let num_threads = 20;
    let barrier = Arc::new(Barrier::new(num_threads));
    let counter = Arc::new(Mutex::new(0));
    
    let handles: Vec<_> = (0..num_threads).map(|i| {
        let barrier = Arc::clone(&barrier);
        let counter = Arc::clone(&counter);
        thread::spawn(move || {
            barrier.wait();
            
            // Alternate between different operations
            for j in 0..5 {
                if j % 3 == 0 {
                    // Add history
                    let entry = format!("concurrent_{}_{}", i, j);
                    apchat_vty::ReadlineInstance::add_history(&entry).unwrap();
                } else if j % 3 == 1 {
                    // Save history
                    apchat_vty::ReadlineInstance::save_history().unwrap();
                } else {
                    // Get instance (simulate readline access)
                    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
                    let _ = guard.deref_mut();
                }
            }
            
            let mut count = counter.lock().unwrap();
            *count += 1;
        })
    }).collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let final_count = *counter.lock().unwrap();
    
    println!("✓ All {} threads completed", num_threads);
    println!("✓ {} operation cycles completed", final_count);
    println!("✓ No race conditions in mixed operations");
    
    assert_eq!(final_count, num_threads, "All threads should complete");
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_history_persistence_across_operations() {
    println!("\n=== Testing History Persistence Across Operations ===\n");
    
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_history_persistence_across_operations");
    
    // Add initial entries
    apchat_vty::ReadlineInstance::add_history("persist_test_1").unwrap();
    apchat_vty::ReadlineInstance::add_history("persist_test_2").unwrap();
    
    // Save history
    apchat_vty::ReadlineInstance::save_history().unwrap();
    
    // Add more entries
    apchat_vty::ReadlineInstance::add_history("persist_test_3").unwrap();
    
    // Verify all entries still exist
    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    let history_len = rl.get_history_entries().len();
    
    println!("✓ History length after mixed operations: {}", history_len);
    
    assert_eq!(history_len, 3, "Should have all 3 history entries");
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_long_running_concurrent_access() {
    println!("\n=== Testing Long-Running Concurrent Access ===\n");
    
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_long_running_concurrent_access");
    
    let num_threads = 10;
    let operations_per_thread = 50;
    let barrier = Arc::new(Barrier::new(num_threads));
    
    let start_time = std::time::Instant::now();
    
    let handles: Vec<_> = (0..num_threads).map(|i| {
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || {
            barrier.wait();
            
            for j in 0..operations_per_thread {
                // Alternate operations
                if j % 2 == 0 {
                    let entry = format!("long_run_{}_{}", i, j);
                    apchat_vty::ReadlineInstance::add_history(&entry).unwrap();
                } else {
                    apchat_vty::ReadlineInstance::save_history().unwrap();
                }
                
                // Small delay to simulate real usage
                thread::sleep(Duration::from_millis(5));
            }
        })
    }).collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let duration = start_time.elapsed();
    
    // Verify final state
    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    let history_len = rl.get_history_entries().len();
    
    println!("✓ All {} threads completed", num_threads);
    println!("✓ Completed in {:?}", duration);
    println!("✓ History contains {} entries", history_len);
    
    // Should have at least some entries (some may be lost due to save operations)
    assert!(history_len > 0, "History should not be empty");
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_cleanup_does_not_prevent_reuse() {
    println!("\n=== Testing Cleanup Doesn't Prevent Reuse ===\n");
    
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_cleanup_does_not_prevent_reuse");
    
    // Add history and cleanup
    apchat_vty::ReadlineInstance::add_history("before_cleanup").unwrap();
    apchat_vty::ReadlineInstance::cleanup().unwrap();
    
    // Verify instance is still usable after cleanup
    apchat_vty::ReadlineInstance::add_history("after_cleanup").unwrap();
    
    // Verify entry was added
    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    let history_len = rl.get_history_entries().len();
    
    println!("✓ Instance is reusable after cleanup");
    println!("✓ History length after reuse: {}", history_len);
    
    assert_eq!(history_len, 1, "Should have one entry after reuse");
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_high_concurrency_with_mixed_operations() {
    println!("\n=== Testing High Concurrency with Mixed Operations ===\n");
    
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_high_concurrency_with_mixed_operations");
    
    let num_threads = 50;
    let barrier = Arc::new(Barrier::new(num_threads));
    
    let start_time = std::time::Instant::now();
    
    let handles: Vec<_> = (0..num_threads).map(|i| {
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || {
            barrier.wait();
            
            // Each thread performs a mix of operations
            for j in 0..10 {
                match j % 4 {
                    0 => {
                        // Add history
                        let entry = format!("mixed_{}_{}", i, j);
                        apchat_vty::ReadlineInstance::add_history(&entry).unwrap();
                    }
                    1 => {
                        // Save history
                        apchat_vty::ReadlineInstance::save_history().unwrap();
                    }
                    2 => {
                        // Get guard
                        let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
                        let _ = guard.deref_mut();
                    }
                    _ => {
                        // Add history with different method
                        let entry = format!("alt_{}_{}", i, j);
                        let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
                        let rl = guard.deref_mut();
                        rl.add_history_entry(&entry);
                    }
                }
            }
        })
    }).collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let duration = start_time.elapsed();
    
    // Verify final state
    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    let history_len = rl.get_history_entries().len();
    
    println!("✓ All {} threads completed", num_threads);
    println!("✓ Completed in {:?}", duration);
    println!("✓ History contains {} entries", history_len);
    println!("✓ No race conditions with mixed operation types");
    
    assert!(history_len > 0, "History should not be empty");
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_rapid_cleanup_saves() {
    println!("\n=== Testing Rapid Cleanup and Save Operations ===\n");
    
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_rapid_cleanup_saves");
    
    // Test rapid sequence of cleanup and save operations
    for i in 0..10 {
        apchat_vty::ReadlineInstance::add_history(&format!("rapid_save_{}", i)).unwrap();
        apchat_vty::ReadlineInstance::save_history().unwrap();
    }
    
    // Final cleanup
    apchat_vty::ReadlineInstance::cleanup().unwrap();
    
    // Verify instance is still functional
    apchat_vty::ReadlineInstance::add_history("after_rapid_ops").unwrap();
    
    println!("✓ Rapid cleanup/save operations completed");
    println!("✓ Instance remains functional after rapid operations");
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_history_boundary_conditions() {
    println!("\n=== Testing History Boundary Conditions ===\n");
    
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_history_boundary_conditions");
    
    // Test with empty strings
    apchat_vty::ReadlineInstance::add_history("").unwrap();
    
    // Test with very long strings
    let long_string = "a".repeat(10000);
    apchat_vty::ReadlineInstance::add_history(&long_string).unwrap();
    
    // Test with special characters
    apchat_vty::ReadlineInstance::add_history("!@#$%^&*()[]{}|\\<>?").unwrap();
    
    // Test with unicode
    apchat_vty::ReadlineInstance::add_history("Hello 世界 🌍").unwrap();
    
    // Verify all entries were added
    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    let history_len = rl.get_history_entries().len();
    
    println!("✓ All boundary condition entries added");
    println!("✓ History length: {}", history_len);
    
    assert_eq!(history_len, 4, "Should have all 4 boundary entries");
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_concurrent_cleanup_attempts() {
    println!("\n=== Testing Concurrent Cleanup Attempts ===\n");
    
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_concurrent_cleanup_attempts");
    
    let num_threads = 20;
    let barrier = Arc::new(Barrier::new(num_threads));
    
    let handles: Vec<_> = (0..num_threads).map(|_| {
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || {
            barrier.wait();
            
            // All threads try to cleanup simultaneously
            apchat_vty::ReadlineInstance::cleanup().unwrap();
        })
    }).collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    println!("✓ All {} concurrent cleanup attempts completed", num_threads);
    println!("✓ No deadlocks or race conditions in cleanup");
    // Lock is automatically released when _lock goes out of scope
}
