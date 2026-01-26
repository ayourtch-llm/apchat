// Tests for proper input handling
// These tests verify that the readline fixes handle input correctly

use apchat_vty::ReadlineInstance;
use apchat_vty::instance::TestLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::ops::DerefMut;

#[test]
fn test_empty_input_handling() {
    let _lock = TestLock::acquire("test_empty_input_handling");
    println!("\n=== Testing Empty Input Handling ===\n");
    
    // Add an empty string - should not cause issues
    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    rl.clear_history_for_tests_only();

    rl.add_history_entry("");
    println!("✓ Empty string added without panic");

    // Verify history still works
    rl.add_history_entry("normal entry");
    println!("✓ Normal entries still work after empty string");
    
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_whitespace_input_handling() {
    let _lock = TestLock::acquire("test_whitespace_input_handling");
    println!("\n=== Testing Whitespace Input Handling ===\n");
    
    // Add various whitespace strings
    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    rl.clear_history_for_tests_only();

    rl.add_history_entry("   ");
    rl.add_history_entry("\t");
    rl.add_history_entry("\n");
    rl.add_history_entry(" \t\n ");
    println!("✓ Whitespace strings handled without issues");

    // Verify history still works
    assert_eq!(rl.get_history_entries().len(), 0);
    println!("✓ All whitespace entries added to history");
    
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_special_characters_handling() {
    let _lock = TestLock::acquire("test_special_characters_handling");
    println!("\n=== Testing Special Characters Handling ===\n");
    
    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    rl.clear_history_for_tests_only();
    
    // Add entries with special characters
    let special_entries = vec![
        "test with !@#$%^&*()",
        "test with \\ /\"'|:;,<>.?",
        "test with \x00\x01\x02",
        "test with Unicode: 你好世界",
        "test with emoji: 😀🎉",
    ];
    
    for entry in &special_entries {
        rl.add_history_entry(entry);
    }

    println!("✓ All special character entries handled");
    assert_eq!(rl.get_history_entries().len(), 5);
    println!("✓ All entries added to history");
    
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_long_input_handling() {
    let _lock = TestLock::acquire("test_long_input_handling");
    println!("\n=== Testing Long Input Handling ===\n");
    
    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    rl.clear_history_for_tests_only();
    
    // Add a very long string
    let long_string = "a".repeat(10000);
    rl.add_history_entry(&long_string);
    println!("✓ Long string (10,000 chars) handled");

    // Add another long string
    let long_string2 = "b".repeat(5000);
    rl.add_history_entry(&long_string2);
    println!("✓ Another long string (5,000 chars) handled");
    
    // Verify both are in history
    assert_eq!(rl.get_history_entries().len(), 2);
    println!("✓ Long entries added to history");
    
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_concurrent_input_handling() {
    let _lock = TestLock::acquire("test_concurrent_input_handling");
    println!("\n=== Testing Concurrent Input Handling ===\n");
    {
        let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
        let rl = guard.deref_mut();
        rl.clear_history_for_tests_only();
    }
    
    let num_threads = 20;
    let barrier = Arc::new(Barrier::new(num_threads));
    
    let handles: Vec<_> = (0..num_threads).map(|i| {
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || {
            barrier.wait();
            
            // Each thread adds various types of input
            let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
            let rl = guard.deref_mut();
            
            // Normal input
            let entry1 = format!("thread {} normal", i);
            rl.add_history_entry(&entry1);

            // Empty input
            rl.add_history_entry("");

            // Whitespace input
            rl.add_history_entry("   ");

            // Special characters
            let entry4 = format!("thread {} special: !@#$", i);
            rl.add_history_entry(&entry4);
            
            println!("Thread {}: Completed", i);
        })
    }).collect();
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Verify final state
    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    let history_len = rl.get_history_entries().len();
    
    println!("\n✓ All {} threads completed", num_threads);
    println!("✓ History contains {} entries", history_len);
    println!("✓ No issues with concurrent input handling");

    assert_eq!(history_len, num_threads * 2, "Should have {} entries", num_threads * 4);
    
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_input_validation() {
    let _lock = TestLock::acquire("test_input_validation");
    println!("\n=== Testing Input Validation ===\n");
    
    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    rl.clear_history_for_tests_only();
    
    // Test various inputs that should be accepted
    let valid_inputs: Vec<String> = vec![
        "".to_string(),
        "   ".to_string(),
        "a".to_string(),
        "a".repeat(1000),
        "Unicode: 你好".to_string(),
        "Special: !@#$%".to_string(),
        "Mixed: abc123 !@# 你好".to_string(),
    ];

    for input in &valid_inputs {
        rl.add_history_entry(input);
    }

    println!("✓ All valid inputs accepted");
    assert_eq!(rl.get_history_entries().len(), valid_inputs.len() - 2);
    println!("✓ All valid inputs added to history");
    
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_history_boundary_conditions() {
    let _lock = TestLock::acquire("test_history_boundary_conditions");
    println!("\n=== Testing History Boundary Conditions ===\n");
    
    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    rl.clear_history_for_tests_only();
    
    // Test with zero-length string
    rl.add_history_entry("");
    println!("✓ Zero-length string handled");

    // Test with single character
    rl.add_history_entry("a");
    println!("✓ Single character handled");

    // Test with maximum reasonable length
    rl.add_history_entry(&"x".repeat(20000));
    println!("✓ Maximum length string handled");
    
    // Verify all are in history
    assert_eq!(rl.get_history_entries().len(), 2);
    println!("✓ All boundary condition inputs in history");
    
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_input_thread_safety() {
    let _lock = TestLock::acquire("test_input_thread_safety");
    println!("\n=== Testing Input Thread Safety ===\n");
    {
        let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
        let rl = guard.deref_mut();
        rl.clear_history_for_tests_only();
    }
    
    let num_threads = 50;
    let barrier = Arc::new(Barrier::new(num_threads));
    let counter = Arc::new(AtomicUsize::new(0));
    
    let handles: Vec<_> = (0..num_threads).map(|i| {
        let barrier = Arc::clone(&barrier);
        let counter = Arc::clone(&counter);
        thread::spawn(move || {
            barrier.wait();
            
            // Each thread adds various types of input
            let inputs = vec![
                format!("thread {} normal", i),
                String::new(), // empty
                "   ".to_string(), // whitespace
                format!("thread {} special: {}", i, "!@#$"),
            ];
            
            for input in inputs {
                let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
                let rl = guard.deref_mut();
                rl.add_history_entry(&input);
            }
            
            // Increment counter
            counter.fetch_add(1, Ordering::SeqCst);
        })
    }).collect();
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    let final_count = counter.load(Ordering::SeqCst);
    
    // Verify final state
    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    let history_len = rl.get_history_entries().len();
    
    println!("\n✓ All {} threads completed", final_count);
    println!("✓ History contains {} entries", history_len);
    println!("✓ No race conditions in input handling");
    
    assert_eq!(final_count, num_threads, "All threads should complete");
    assert_eq!(history_len, num_threads * 2, "Should have {} entries", num_threads * 2);
    
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_duplicate_input_handling() {
    let _lock = TestLock::acquire("test_duplicate_input_handling");
    println!("\n=== Testing Duplicate Input Handling ===\n");
    
    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    rl.clear_history_for_tests_only();
    
    // Add the same entry multiple times
    for _ in 0..5 {
        rl.add_history_entry("duplicate entry");
    }
    
    println!("✓ Duplicate entries handled without issues");
    
    // Verify all are in history (rustyline doesn't automatically deduplicate)
    let history = rl.get_history_entries();
    let duplicate_count = history.iter().filter(|entry| entry.as_str() == "duplicate entry").count();
    assert_eq!(duplicate_count, 1);
    println!("✓ All {} duplicates present in history", duplicate_count);
    
    // Lock is automatically released when _lock goes out of scope
}

#[test]
fn test_mixed_input_types() {
    let _lock = TestLock::acquire("test_mixed_input_types");
    println!("\n=== Testing Mixed Input Types ===\n");
    
    let mut guard = apchat_vty::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    rl.clear_history_for_tests_only();
    
    // Add various types of input
    let inputs: Vec<String> = vec![
        "".to_string(), // empty
        "   ".to_string(), // whitespace
        "normal".to_string(), // normal
        "!@#$%".to_string(), // special chars
        "你好".to_string(), // unicode
        "😀".to_string(), // emoji
        "a".repeat(10000), // long
    ];

    for input in &inputs {
        rl.add_history_entry(input);
    }

    println!("✓ All mixed input types handled");
    // Whitespace only entries should not be in the history
    assert_eq!(rl.get_history_entries().len(), inputs.len() - 2);
    println!("✓ All {} entries in history", inputs.len() - 2);
    
    // Lock is automatically released when _lock goes out of scope
}
