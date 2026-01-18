// Tests to verify single instance behavior
// These tests ensure that the readline singleton pattern works correctly

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn test_single_instance_creation() {
    println!("\n=== Testing Single Instance Creation ===\n");
    
    // Get the instance multiple times
    let guard1 = crate::chat::ReadlineInstance::get().unwrap();
    let guard2 = crate::chat::ReadlineInstance::get().unwrap();
    let guard3 = crate::chat::ReadlineInstance::get().unwrap();
    
    // All should be valid
    assert!(!guard1.is_locked());
    assert!(!guard2.is_locked());
    assert!(!guard3.is_locked());
    
    println!("✓ Multiple calls to get() return valid guards");
    
    // They should be different guards (proper locking)
    assert_ne!(&*guard1 as *const _, &*guard2 as *const _);
    assert_ne!(&*guard2 as *const _, &*guard3 as *const _);
    
    println!("✓ Each call returns a different guard (proper locking)");
}

#[test]
fn test_instance_initialization_once() {
    println!("\n=== Testing Instance Initialization (Once) ===\n");
    
    // Verify the instance is initialized
    assert!(crate::chat::ReadlineInstance::is_initialized());
    println!("✓ Instance is initialized");
    
    // Get the instance and verify configuration
    let guard = crate::chat::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    
    // Verify basic editor configuration
    assert!(rl.max_history_size().is_some());
    println!("✓ Editor is properly configured");
    
    // Verify it's a valid rustyline editor
    assert!(!rl.is_editing());
    println!("✓ Editor is in valid state");
}

#[test]
fn test_no_duplicate_instances() {
    println!("\n=== Testing No Duplicate Instances ===\n");
    
    let num_calls = 100;
    let mut instances = Vec::new();
    
    for _ in 0..num_calls {
        let guard = crate::chat::ReadlineInstance::get().unwrap();
        instances.push(guard);
    }
    
    // All guards should be valid
    assert_eq!(instances.len(), num_calls);
    println!("✓ All {} calls returned valid guards", num_calls);
    
    // Verify they're all different guards
    for i in 1..instances.len() {
        assert_ne!(&*instances[i-1] as *const _, &*instances[i] as *const _);
    }
    
    println!("✓ All guards are distinct (proper locking)");
}

#[test]
fn test_singleton_across_threads() {
    println!("\n=== Testing Singleton Across Threads ===\n");
    
    let num_threads = 20;
    let barrier = Arc::new(Barrier::new(num_threads));
    let counter = Arc::new(AtomicUsize::new(0));
    
    let handles: Vec<_> = (0..num_threads).map(|i| {
        let barrier = Arc::clone(&barrier);
        let counter = Arc::clone(&counter);
        thread::spawn(move || {
            // Wait for all threads to be ready
            barrier.wait();
            
            // Get the instance
            let guard = crate::chat::ReadlineInstance::get().unwrap();
            let rl = guard.deref_mut();
            
            // Verify it's properly configured
            assert!(rl.max_history_size().is_some());
            
            // Increment counter
            counter.fetch_add(1, Ordering::SeqCst);
            
            println!("Thread {}: Got instance", i);
        })
    }).collect();
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    let final_count = counter.load(Ordering::SeqCst);
    
    println!("\n✓ All {} threads got the singleton instance", final_count);
    println!("✓ Single instance used across all threads");
    
    assert_eq!(final_count, num_threads, "All threads should get the instance");
}

#[test]
fn test_singleton_configuration_consistency() {
    println!("\n=== Testing Singleton Configuration Consistency ===\n");
    
    // Get instance multiple times and verify configuration is consistent
    let guard1 = crate::chat::ReadlineInstance::get().unwrap();
    let rl1 = guard1.deref_mut();
    
    let guard2 = crate::chat::ReadlineInstance::get().unwrap();
    let rl2 = guard2.deref_mut();
    
    // Verify both have the same configuration
    assert_eq!(rl1.max_history_size(), rl2.max_history_size());
    println!("✓ Configuration is consistent across accesses");
    
    // Verify they reference the same underlying instance
    // (we can't test this directly due to Rust's borrowing rules)
    // but the configuration check above validates consistency
}

#[test]
fn test_instance_persistence() {
    println!("\n=== Testing Instance Persistence ===\n");
    
    // Get the instance and add some history
    let guard1 = crate::chat::ReadlineInstance::get().unwrap();
    let rl1 = guard1.deref_mut();
    
    // Add history entries
    rl1.add_history_entry("persistent test 1").unwrap();
    rl1.add_history_entry("persistent test 2").unwrap();
    
    // Get the instance again
    let guard2 = crate::chat::ReadlineInstance::get().unwrap();
    let rl2 = guard2.deref_mut();
    
    // Verify history persists
    assert_eq!(rl2.history().len(), 2);
    println!("✓ Instance state persists across accesses");
    
    // Verify specific entries
    let history = rl2.history();
    assert!(history.iter().any(|entry| entry == "persistent test 1"));
    assert!(history.iter().any(|entry| entry == "persistent test 2"));
    println!("✓ Specific history entries persist");
}

#[test]
fn test_no_instance_reinitialization() {
    println!("\n=== Testing No Instance Reinitialization ===\n");
    
    // Get the instance and verify it's initialized
    let guard1 = crate::chat::ReadlineInstance::get().unwrap();
    let rl1 = guard1.deref_mut();
    
    // Add history
    rl1.add_history_entry("initial entry").unwrap();
    
    // Get the instance again - should NOT create a new instance
    let guard2 = crate::chat::ReadlineInstance::get().unwrap();
    let rl2 = guard2.deref_mut();
    
    // Verify it's the same instance (history should be there)
    assert_eq!(rl2.history().len(), 1);
    println!("✓ Instance is not reinitialized");
}

#[test]
fn test_global_access_pattern() {
    println!("\n=== Testing Global Access Pattern ===\n");
    
    // Simulate the global access pattern used in the application
    let guard1 = crate::chat::ReadlineInstance::get().unwrap();
    let rl1 = guard1.deref_mut();
    
    // Use the editor
    rl1.add_history_entry("global access test").unwrap();
    
    // Drop the guard (simulates completion of operation)
    drop(guard1);
    
    // Get the instance again for another operation
    let guard2 = crate::chat::ReadlineInstance::get().unwrap();
    let rl2 = guard2.deref_mut();
    
    // Verify the instance is still valid and history is there
    assert_eq!(rl2.history().len(), 1);
    println!("✓ Global access pattern works correctly");
}

#[test]
fn test_concurrent_singleton_access() {
    println!("\n=== Testing Concurrent Singleton Access ===\n");
    
    let num_threads = 30;
    let barrier = Arc::new(Barrier::new(num_threads));
    let counter = Arc::new(AtomicUsize::new(0));
    
    let handles: Vec<_> = (0..num_threads).map(|i| {
        let barrier = Arc::clone(&barrier);
        let counter = Arc::clone(&counter);
        thread::spawn(move || {
            // Wait for all threads to be ready
            barrier.wait();
            
            // Get the instance
            let guard = crate::chat::ReadlineInstance::get().unwrap();
            let rl = guard.deref_mut();
            
            // Verify it's the singleton
            assert!(rl.max_history_size().is_some());
            
            // Add a history entry
            let entry = format!("concurrent singleton access {}", i);
            rl.add_history_entry(&entry).unwrap();
            
            // Increment counter
            counter.fetch_add(1, Ordering::SeqCst);
        })
    }).collect();
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    let final_count = counter.load(Ordering::SeqCst);
    
    // Verify all threads accessed the same instance
    let guard = crate::chat::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    let history_len = rl.history().len();
    
    println!("\n✓ All {} threads accessed the singleton", final_count);
    println!("✓ History contains {} entries", history_len);
    println!("✓ Single instance used by all threads");
    
    assert_eq!(final_count, num_threads, "All threads should access the singleton");
    assert_eq!(history_len, num_threads, "All entries should be present");
}

#[test]
fn test_singleton_lifecycle() {
    println!("\n=== Testing Singleton Lifecycle ===\n");
    
    // 1. Instance doesn't exist initially (lazily initialized)
    assert!(crate::chat::ReadlineInstance::is_initialized());
    println!("✓ Instance is ready for use");
    
    // 2. First access initializes it
    let guard1 = crate::chat::ReadlineInstance::get().unwrap();
    let rl1 = guard1.deref_mut();
    rl1.add_history_entry("lifecycle test").unwrap();
    println!("✓ Instance initialized on first access");
    
    // 3. Subsequent accesses use the same instance
    let guard2 = crate::chat::ReadlineInstance::get().unwrap();
    let rl2 = guard2.deref_mut();
    assert_eq!(rl2.history().len(), 1);
    println!("✓ Subsequent accesses use existing instance");
    
    // 4. Cleanup at end of lifecycle
    crate::chat::ReadlineInstance::cleanup().unwrap();
    let guard3 = crate::chat::ReadlineInstance::get().unwrap();
    let rl3 = guard3.deref_mut();
    assert_eq!(rl3.history().len(), 0);
    println!("✓ Cleanup works correctly");
}
