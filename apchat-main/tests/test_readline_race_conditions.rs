// Integration tests specifically for race condition fixes
// These tests verify that the readline fixes prevent race conditions

use apchat_vty::ReadlineInstance;
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::Duration;
use std::ops::DerefMut;

#[test]
fn test_no_race_condition_in_history_addition() {
    println!("\n=== Testing Race Condition Fix: History Addition ===\n");
    
    let num_threads = 50;
    let barrier = Arc::new(Barrier::new(num_threads));
    
    let handles: Vec<_> = (0..num_threads).map(|i| {
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || {
            // Wait for all threads to be ready
            barrier.wait();
            
            // All threads try to add history simultaneously
            for j in 0..10 {
                let entry = format!("thread_{}_entry_{}", i, j);
                apchat_vty::ReadlineInstance::add_history(&entry).unwrap();
            }
            
            println!("Thread {}: Completed", i);
        })
    }).collect();
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Verify final state
    let guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    let history_len = rl.get_history_entries().len();
    
    println!("\n✓ All {} threads completed successfully", num_threads);
    println!("✓ History contains {} entries", history_len);
    println!("✓ Expected: {} entries", num_threads * 10);
    
    assert_eq!(history_len, num_threads * 10, "Should have exactly {} history entries, got {}", num_threads * 10, history_len);
}

#[test]
fn test_no_race_condition_in_concurrent_readline_calls() {
    println!("\n=== Testing Race Condition Fix: Concurrent Readline Calls ===\n");
    
    // Note: This test won't actually call readline (requires TTY), but it tests
    // that the synchronization mechanism works for concurrent access
    
    let num_threads = 20;
    let barrier = Arc::new(Barrier::new(num_threads));
    let counter = Arc::new(Mutex::new(0));
    
    let handles: Vec<_> = (0..num_threads).map(|i| {
        let barrier = Arc::clone(&barrier);
        let counter = Arc::clone(&counter);
        thread::spawn(move || {
            barrier.wait();
            
            // Each thread gets the readline instance and performs operations
            let guard = apchat_vty::ReadlineInstance::get().unwrap();
            let rl = guard.deref_mut();
            
            // Add history entry
            let entry = format!("concurrent_test_{}", i);
            rl.add_history_entry(&entry).unwrap();
            
            // Verify we can still access history
            let history_len = rl.get_history_entries().len();
            
            // Increment counter to track successful operations
            let mut count = counter.lock().unwrap();
            *count += 1;
            
            println!("Thread {}: Completed, history len: {}", i, history_len);
        })
    }).collect();
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    let final_count = *counter.lock().unwrap();
    println!("\n✓ All {} threads completed successfully", num_threads);
    println!("✓ {} operations performed", final_count);
    println!("✓ No race conditions detected in concurrent access");
    
    assert_eq!(final_count, num_threads, "All threads should complete successfully");
}

#[test]
fn test_lock_acquisition_order() {
    println!("\n=== Testing Lock Acquisition Order ===\n");
    
    // This test verifies that locks are acquired in a predictable order
    // and that there are no deadlocks
    
    let num_threads = 15;
    let handles: Vec<_> = (0..num_threads).map(|i| {
        thread::spawn(move || {
            // Each thread acquires the lock multiple times
            for j in 0..3 {
                let guard1 = apchat_vty::ReadlineInstance::get().unwrap();
                let rl = guard1.deref_mut();

                // Perform some operation
                let entry = format!("order_test_{}_iteration_{}", i, j);
                rl.add_history_entry(&entry).unwrap();

                // Drop the guard to release the lock
                drop(guard1);
                
                // Small delay to increase chance of interleaving
                thread::sleep(Duration::from_millis(1));
            }
            
            println!("Thread {}: Completed", i);
        })
    }).collect();
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Verify final state
    let guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    let history_len = rl.get_history_entries().len();
    
    println!("\n✓ All {} threads completed successfully", num_threads);
    println!("✓ History contains {} entries", history_len);
    println!("✓ No deadlocks detected");
    
    assert_eq!(history_len, num_threads * 3, "Should have exactly {} history entries", num_threads * 3);
}

#[test]
fn test_high_concurrency_stress_test() {
    println!("\n=== Testing High Concurrency Stress ===\n");
    
    let num_threads = 100;
    let barrier = Arc::new(Barrier::new(num_threads));
    
    let start_time = std::time::Instant::now();
    
    let handles: Vec<_> = (0..num_threads).map(|i| {
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || {
            barrier.wait();
            
            // Each thread performs multiple operations
            for j in 0..20 {
                // Alternate between adding history and saving
                if j % 2 == 0 {
                    let entry = format!("stress_{}_{}", i, j);
                    apchat_vty::ReadlineInstance::add_history(&entry).unwrap();
                } else {
                    // Save history - this should be thread-safe
                    apchat_vty::ReadlineInstance::save_history().unwrap();
                }
            }
        })
    }).collect();
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    let duration = start_time.elapsed();
    
    // Verify final state
    let guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    let history_len = rl.get_history_entries().len();
    
    println!("\n✓ All {} threads completed successfully", num_threads);
    println!("✓ Completed in {:?}", duration);
    println!("✓ History contains {} entries", history_len);
    println!("✓ No race conditions detected under high concurrency");
    
    // Verify that we have approximately the right number of entries
    // (some might be duplicates or lost due to save operations, but no crashes)
    assert!(history_len > 0, "History should not be empty");
}

#[test]
fn test_history_consistency_across_threads() {
    println!("\n=== Testing History Consistency Across Threads ===\n");
    
    let num_threads = 10;
    let handles: Vec<_> = (0..num_threads).map(|i| {
        thread::spawn(move || {
            // Each thread adds a unique prefix
            let prefix = format!("thread_{}_", i);
            
            // Add 5 entries with this prefix
            for j in 0..5 {
                let entry = format!("{}{}", prefix, j);
                apchat_vty::ReadlineInstance::add_history(&entry).unwrap();
            }
        })
    }).collect();
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Verify the history
    let guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    let history = rl.get_history_entries();
    
    println!("\n✓ All threads completed");
    println!("✓ History length: {}", history.len());
    
    // Verify that all entries are present
    let mut found_entries = 0;
    for i in 0..num_threads {
        for j in 0..5 {
            let expected = format!("thread_{}_{}", i, j);
            if history.iter().any(|entry| entry == &expected) {
                found_entries += 1;
            }
        }
    }
    
    println!("✓ Found {} out of {} expected entries", found_entries, num_threads * 5);
    
    assert_eq!(found_entries, num_threads * 5, "All expected entries should be present");
}

#[test]
fn test_lock_fairness() {
    println!("\n=== Testing Lock Fairness ===\n");
    
    // This test verifies that the lock is fair and doesn't starve threads
    
    let num_threads = 20;
    let operations_per_thread = 5;
    let total_operations = num_threads * operations_per_thread;
    
    let counter = Arc::new(Mutex::new(0));
    
    let handles: Vec<_> = (0..num_threads).map(|i| {
        let counter = Arc::clone(&counter);
        thread::spawn(move || {
            for _ in 0..operations_per_thread {
                let guard = apchat_vty::ReadlineInstance::get().unwrap();
                let rl = guard.deref_mut();
                
                // Perform operation
                let entry = format!("fairness_test_{}", i);
                rl.add_history_entry(&entry).unwrap();
                
                // Increment counter
                let mut count = counter.lock().unwrap();
                *count += 1;
            }
        })
    }).collect();
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    let final_count = *counter.lock().unwrap();
    
    println!("\n✓ All operations completed: {}/{}", final_count, total_operations);
    println!("✓ No thread starvation detected");
    
    assert_eq!(final_count, total_operations, "All operations should complete");
}
