// Tests for proper input handling
// These tests verify that the readline fixes handle input correctly

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn test_empty_input_handling() {
    println!("\n=== Testing Empty Input Handling ===\n");
    
    // Add an empty string - should not cause issues
    let guard = crate::chat::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    
    assert!(rl.add_history_entry("").is_ok());
    println!("✓ Empty string added without panic");
    
    // Verify history still works
    assert!(rl.add_history_entry("normal entry").is_ok());
    println!("✓ Normal entries still work after empty string");
}

#[test]
fn test_whitespace_input_handling() {
    println!("\n=== Testing Whitespace Input Handling ===\n");
    
    // Add various whitespace strings
    let guard = crate::chat::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    
    assert!(rl.add_history_entry("   ").is_ok());
    assert!(rl.add_history_entry("\t").is_ok());
    assert!(rl.add_history_entry("\n").is_ok());
    assert!(rl.add_history_entry(" \t\n ").is_ok());
    println!("✓ Whitespace strings handled without issues");
    
    // Verify history still works
    assert_eq!(rl.history().len(), 4);
    println!("✓ All whitespace entries added to history");
}

#[test]
fn test_special_characters_handling() {
    println!("\n=== Testing Special Characters Handling ===\n");
    
    let guard = crate::chat::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    
    // Add entries with special characters
    let special_entries = vec![
        "test with !@#$%^&*()",
        "test with \\/""'|:;,.<>?",
        "test with \x00\x01\x02",
        "test with Unicode: 你好世界",
        "test with emoji: 😀🎉",
    ];
    
    for entry in &special_entries {
        assert!(rl.add_history_entry(entry).is_ok(), "Failed to add: {}", entry);
    }
    
    println!("✓ All special character entries handled");
    assert_eq!(rl.history().len(), 5);
    println!("✓ All entries added to history");
}

#[test]
fn test_long_input_handling() {
    println!("\n=== Testing Long Input Handling ===\n");
    
    let guard = crate::chat::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    
    // Add a very long string
    let long_string = "a".repeat(10000);
    assert!(rl.add_history_entry(&long_string).is_ok());
    println!("✓ Long string (10,000 chars) handled");
    
    // Add another long string
    let long_string2 = "b".repeat(5000);
    assert!(rl.add_history_entry(&long_string2).is_ok());
    println!("✓ Another long string (5,000 chars) handled");
    
    // Verify both are in history
    assert_eq!(rl.history().len(), 2);
    println!("✓ Long entries added to history");
}

#[test]
fn test_concurrent_input_handling() {
    println!("\n=== Testing Concurrent Input Handling ===\n");
    
    let num_threads = 20;
    let barrier = Arc::new(Barrier::new(num_threads));
    
    let handles: Vec<_> = (0..num_threads).map(|i| {
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || {
            barrier.wait();
            
            // Each thread adds various types of input
            let guard = crate::chat::ReadlineInstance::get().unwrap();
            let rl = guard.deref_mut();
            
            // Normal input
            let entry1 = format!("thread {} normal", i);
            assert!(rl.add_history_entry(&entry1).is_ok());
            
            // Empty input
            assert!(rl.add_history_entry("").is_ok());
            
            // Whitespace input
            assert!(rl.add_history_entry("   ").is_ok());
            
            // Special characters
            let entry4 = format!("thread {} special: !@#$", i);
            assert!(rl.add_history_entry(&entry4).is_ok());
            
            println!("Thread {}: Completed", i);
        })
    }).collect();
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Verify final state
    let guard = crate::chat::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    let history_len = rl.history().len();
    
    println!("\n✓ All {} threads completed", num_threads);
    println!("✓ History contains {} entries", history_len);
    println!("✓ No issues with concurrent input handling");
    
    assert_eq!(history_len, num_threads * 4, "Should have {} entries", num_threads * 4);
}

#[test]
fn test_input_validation() {
    println!("\n=== Testing Input Validation ===\n");
    
    let guard = crate::chat::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    
    // Test various inputs that should be accepted
    let valid_inputs = vec![
        "",
        "   ",
        "a",
        "a".repeat(1000),
        "Unicode: 你好",
        "Special: !@#$%",
        "Mixed: abc123 !@# 你好",
    ];
    
    for input in &valid_inputs {
        assert!(rl.add_history_entry(input).is_ok(), "Failed to add valid input: {:?}", input);
    }
    
    println!("✓ All valid inputs accepted");
    assert_eq!(rl.history().len(), valid_inputs.len());
    println!("✓ All valid inputs added to history");
}

#[test]
fn test_history_boundary_conditions() {
    println!("\n=== Testing History Boundary Conditions ===\n");
    
    let guard = crate::chat::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    
    // Test with zero-length string
    assert!(rl.add_history_entry("").is_ok());
    println!("✓ Zero-length string handled");
    
    // Test with single character
    assert!(rl.add_history_entry("a").is_ok());
    println!("✓ Single character handled");
    
    // Test with maximum reasonable length
    assert!(rl.add_history_entry(&"x".repeat(20000)).is_ok());
    println!("✓ Maximum length string handled");
    
    // Verify all are in history
    assert_eq!(rl.history().len(), 3);
    println!("✓ All boundary condition inputs in history");
}

#[test]
fn test_input_thread_safety() {
    println!("\n=== Testing Input Thread Safety ===\n");
    
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
                let guard = crate::chat::ReadlineInstance::get().unwrap();
                let rl = guard.deref_mut();
                assert!(rl.add_history_entry(&input).is_ok());
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
    let guard = crate::chat::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    let history_len = rl.history().len();
    
    println!("\n✓ All {} threads completed", final_count);
    println!("✓ History contains {} entries", history_len);
    println!("✓ No race conditions in input handling");
    
    assert_eq!(final_count, num_threads, "All threads should complete");
    assert_eq!(history_len, num_threads * 4, "Should have {} entries", num_threads * 4);
}

#[test]
fn test_input_after_cleanup() {
    println!("\n=== Testing Input After Cleanup ===\n");
    
    // Add some initial entries
    let guard1 = crate::chat::ReadlineInstance::get().unwrap();
    let rl1 = guard1.deref_mut();
    rl1.add_history_entry("initial entry").unwrap();
    println!("✓ Initial entry added");
    
    // Clean up
    crate::chat::ReadlineInstance::cleanup().unwrap();
    println!("✓ Cleanup performed");
    
    // Verify history is cleared
    let guard2 = crate::chat::ReadlineInstance::get().unwrap();
    let rl2 = guard2.deref_mut();
    assert_eq!(rl2.history().len(), 0);
    println!("✓ History cleared after cleanup");
    
    // Add new entries after cleanup
    assert!(rl2.add_history_entry("after cleanup entry").is_ok());
    println!("✓ New entries can be added after cleanup");
    
    // Verify the new entry is there
    assert_eq!(rl2.history().len(), 1);
    println!("✓ New entry is in history");
}

#[test]
fn test_duplicate_input_handling() {
    println!("\n=== Testing Duplicate Input Handling ===\n");
    
    let guard = crate::chat::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    
    // Add the same entry multiple times
    for _ in 0..5 {
        assert!(rl.add_history_entry("duplicate entry").is_ok());
    }
    
    println!("✓ Duplicate entries handled without issues");
    
    // Verify all are in history (rustyline doesn't automatically deduplicate)
    let history = rl.history();
    let duplicate_count = history.iter().filter(|&&entry| entry == "duplicate entry").count();
    assert_eq!(duplicate_count, 5);
    println!("✓ All {} duplicates present in history", duplicate_count);
}

#[test]
fn test_mixed_input_types() {
    println!("\n=== Testing Mixed Input Types ===\n");
    
    let guard = crate::chat::ReadlineInstance::get().unwrap();
    let rl = guard.deref_mut();
    
    // Add various types of input
    let inputs = vec![
        "", // empty
        "   ", // whitespace
        "normal", // normal
        "!@#$%", // special chars
        "你好", // unicode
        "😀", // emoji
        "a".repeat(10000), // long
    ];
    
    for input in &inputs {
        assert!(rl.add_history_entry(input).is_ok());
    }
    
    println!("✓ All mixed input types handled");
    assert_eq!(rl.history().len(), inputs.len());
    println!("✓ All {} entries in history", inputs.len());
}
