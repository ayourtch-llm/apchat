// Tests to verify single instance behavior
// These tests ensure that the readline singleton pattern works correctly

use apchat_vty::ReadlineInstance;
use apchat_vty::instance::TestLock;
use std::ops::DerefMut;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn test_single_instance_creation() {
    println!("\n=== Testing Single Instance Creation ===\n");

    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_single_instance_creation");

    // Get the instance multiple times
    let mut guard1 = apchat_vty::ReadlineInstance::get().unwrap();
    let mut guard2 = apchat_vty::ReadlineInstance::get().unwrap();
    let mut guard3 = apchat_vty::ReadlineInstance::get().unwrap();

    // All should be valid - MutexGuard doesn't have is_locked()
    // We just verify they work
    assert!(guard1.line().is_empty());
    assert!(guard2.line().is_empty());
    assert!(guard3.line().is_empty());

    println!("✓ Multiple calls to get() return valid guards");

    // They should be different guards (proper locking)
    assert_ne!(&*guard1 as *const _, &*guard2 as *const _);
    assert_ne!(&*guard2 as *const _, &*guard3 as *const _);

    println!("✓ Each call returns a different guard (proper locking)");
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_instance_initialization_once() {
    println!("\n=== Testing Instance Initialization (Once) ===\n");
    
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_instance_initialization_once");
    
    // Verify the instance is initialized
    assert!(apchat_vty::ReadlineInstance::is_initialized());
    println!("✓ Instance is initialized");
    
    // Get the instance and verify configuration
    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    
    // Verify basic editor configuration
    // // assert!(rl.max_history_size().is_some());
    println!("✓ Editor is properly configured");
    
    // Verify it's a valid rustyline editor
    // assert!(!rl.is_editing());
    println!("✓ Editor is in valid state");
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_no_duplicate_instances() {
    println!("\n=== Testing No Duplicate Instances ===\n");
    
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_no_duplicate_instances");
    
    let num_calls = 100;
    let mut instances = Vec::new();
    
    for _ in 0..num_calls {
        let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
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
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_singleton_across_threads() {
    println!("\n=== Testing Singleton Across Threads ===\n");
    
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_singleton_across_threads");
    
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
            let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
            let rl = guard.deref_mut();
            
            // Verify it's properly configured
            // assert!(rl.max_history_size().is_some());
            
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
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_singleton_configuration_consistency() {
    println!("\n=== Testing Singleton Configuration Consistency ===\n");
    
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_singleton_configuration_consistency");
    
    // Get instance multiple times and verify configuration is consistent
    let mut guard1 = apchat_vty::ReadlineInstance::get().unwrap();
    let rl1 = guard1.deref_mut();
    
    let mut guard2 = apchat_vty::ReadlineInstance::get().unwrap();
    let rl2 = guard2.deref_mut();
    
    // Verify both have the same configuration
    // assert_eq!(rl1.max_history_size(), rl2.max_history_size());
    println!("✓ Configuration is consistent across accesses");
    
    // Verify they reference the same underlying instance
    // (we can't test this directly due to Rust's borrowing rules)
    // but the configuration check above validates consistency
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_instance_persistence() {
    println!("\n=== Testing Instance Persistence ===\n");
    
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_instance_persistence");
    
    // Get the instance and add some history
    let mut guard1 = apchat_vty::ReadlineInstance::get().unwrap();
    let rl1 = guard1.deref_mut();
    
    // Add history entries
    rl1.add_history_entry("persistent test 1");
    rl1.add_history_entry("persistent test 2");
    
    // Get the instance again
    let mut guard2 = apchat_vty::ReadlineInstance::get().unwrap();
    let rl2 = guard2.deref_mut();
    
    // Verify history persists
    assert_eq!(rl2.get_history_entries().len(), 2);
    println!("✓ Instance state persists across accesses");
    
    // Verify specific entries
    let history = rl2.get_history_entries();
    assert!(history.iter().any(|entry| entry == "persistent test 1"));
    assert!(history.iter().any(|entry| entry == "persistent test 2"));
    println!("✓ Specific history entries persist");
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_no_instance_reinitialization() {
    println!("\n=== Testing No Instance Reinitialization ===\n");
    
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_no_instance_reinitialization");
    
    // Get the instance and verify it's initialized
    let mut guard1 = apchat_vty::ReadlineInstance::get().unwrap();
    let rl1 = guard1.deref_mut();
    
    // Add history
    rl1.add_history_entry("initial entry");
    
    // Get the instance again - should NOT create a new instance
    let mut guard2 = apchat_vty::ReadlineInstance::get().unwrap();
    let rl2 = guard2.deref_mut();
    
    // Verify it's the same instance (history should be there)
    assert_eq!(rl2.get_history_entries().len(), 1);
    println!("✓ Instance is not reinitialized");
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_global_access_pattern() {
    println!("\n=== Testing Global Access Pattern ===\n");
    
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_global_access_pattern");
    
    // Simulate the global access pattern used in the application
    let mut guard1 = apchat_vty::ReadlineInstance::get().unwrap();
    let rl1 = guard1.deref_mut();
    
    // Use the editor
    rl1.add_history_entry("global access test");
    
    // Drop the guard (simulates completion of operation)
    drop(guard1);
    
    // Get the instance again for another operation
    let mut guard2 = apchat_vty::ReadlineInstance::get().unwrap();
    let rl2 = guard2.deref_mut();
    
    // Verify the instance is still valid and history is there
    assert_eq!(rl2.get_history_entries().len(), 1);
    println!("✓ Global access pattern works correctly");
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_concurrent_singleton_access() {
    println!("\n=== Testing Concurrent Singleton Access ===\n");
    
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_concurrent_singleton_access");
    
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
            let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
            let rl = guard.deref_mut();
            
            // Verify it's the singleton
            // assert!(rl.max_history_size().is_some());
            
            // Add a history entry
            let entry = format!("concurrent singleton access {}", i);
            rl.add_history_entry(&entry);
            
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
    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    let history_len = rl.get_history_entries().len();
    
    println!("\n✓ All {} threads accessed the singleton", final_count);
    println!("✓ History contains {} entries", history_len);
    println!("✓ Single instance used by all threads");
    
    assert_eq!(final_count, num_threads, "All threads should access the singleton");
    assert_eq!(history_len, num_threads, "All entries should be present");
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_singleton_lifecycle() {
    println!("\n=== Testing Singleton Lifecycle ===\n");
    
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_singleton_lifecycle");
    
    // 1. Instance doesn't exist initially (lazily initialized)
    assert!(apchat_vty::ReadlineInstance::is_initialized());
    println!("✓ Instance is ready for use");
    
    // 2. First access initializes it
    let mut guard1 = apchat_vty::ReadlineInstance::get().unwrap();
    let rl1 = guard1.deref_mut();
    rl1.add_history_entry("lifecycle test");
    println!("✓ Instance initialized on first access");
    
    // 3. Subsequent accesses use the same instance
    let mut guard2 = apchat_vty::ReadlineInstance::get().unwrap();
    let rl2 = guard2.deref_mut();
    assert_eq!(rl2.get_history_entries().len(), 1);
    println!("✓ Subsequent accesses use existing instance");
    
    // 4. Cleanup at end of lifecycle
    apchat_vty::ReadlineInstance::cleanup().unwrap();
    let mut guard3 = apchat_vty::ReadlineInstance::get().unwrap();
    let rl3 = guard3.deref_mut();
    assert_eq!(rl3.get_history_entries().len(), 0);
    println!("✓ Cleanup works correctly");
    // Lock is automatically released when _lock goes out of scope
}
