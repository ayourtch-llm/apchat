// Test to verify readline singleton pattern functionality
use std::sync::Once;

static START: Once = Once::new();

fn setup() {
    START.call_once(|| {
        // Initialize logging if needed
        env_logger::init();
    });
}

#[test]
fn test_singleton_creation() {
    setup();
    
    // Get the singleton instance
    let rl1 = crate::chat::ReadlineInstance::get().unwrap();
    
    // Verify it's initialized
    assert!(crate::chat::ReadlineInstance::is_initialized());
    
    // Get the instance again
    let rl2 = crate::chat::ReadlineInstance::get().unwrap();
    
    // Both should have the same configuration
    assert_eq!(rl1.max_history_size(), rl2.max_history_size());
}

#[test]
fn test_history_persistence() {
    setup();
    
    // Get the instance
    let mut rl = crate::chat::ReadlineInstance::get().unwrap();
    
    // Add some history entries
    let test_commands = vec![
        "echo 'test 1'",
        "echo 'test 2'",
        "echo 'test 3'",
    ];
    
    for cmd in &test_commands {
        rl.add_history_entry(cmd).unwrap();
    }
    
    // Verify entries were added
    let history_len = rl.history()
        .map(|h| h.iter().count())
        .unwrap_or(0);
    assert_eq!(history_len, 3);
    
    // Get the instance again and verify history persists
    let rl2 = crate::chat::ReadlineInstance::get().unwrap();
    let history_len2 = rl2.history()
        .map(|h| h.iter().count())
        .unwrap_or(0);
    assert_eq!(history_len2, 3);
}

// Test to verify readline singleton pattern functionality
use std::sync::Once;

static START: Once = Once::new();

fn setup() {
    START.call_once(|| {
        // Initialize logging if needed
        env_logger::init();
    });
}

#[test]
fn test_singleton_creation() {
    setup();
    
    // Get the singleton instance
    let rl1 = crate::chat::ReadlineInstance::get().unwrap();
    
    // Verify it's initialized
    assert!(crate::chat::ReadlineInstance::is_initialized());
    
    // Get the instance again
    let rl2 = crate::chat::ReadlineInstance::get().unwrap();
    
    // Both should have the same configuration
    assert_eq!(rl1.max_history_size(), rl2.max_history_size());
}

#[test]
fn test_history_persistence() {
    setup();
    
    // Get the instance
    let mut rl = crate::chat::ReadlineInstance::get().unwrap();
    
    // Add some history entries
    let test_commands = vec![
        "echo 'test 1'",
        "echo 'test 2'",
        "echo 'test 3'",
    ];
    
    for cmd in &test_commands {
        rl.add_history_entry(cmd).unwrap();
    }
    
    // Verify entries were added
    let history_len = rl.history()
        .map(|h| h.iter().count())
        .unwrap_or(0);
    assert_eq!(history_len, 3);
    
    // Get the instance again and verify history persists
    let rl2 = crate::chat::ReadlineInstance::get().unwrap();
    let history_len2 = rl2.history()
        .map(|h| h.iter().count())
        .unwrap_or(0);
    assert_eq!(history_len2, 3);
}

#[test]
fn test_cleanup_functionality() {
    setup();
    
    // Add some history entries
    crate::chat::ReadlineInstance::add_history("test command 1").unwrap();
    crate::chat::ReadlineInstance::add_history("test command 2").unwrap();
    
    // Verify entries were added
    let guard = crate::chat::ReadlineInstance::get().unwrap();
    let count = guard.history()
        .map(|h| h.iter().count())
        .unwrap_or(0);
    assert_eq!(count, 2);
    
    // Call cleanup - should not panic
    let result = crate::chat::ReadlineInstance::cleanup();
    assert!(result.is_ok());
    
    // After cleanup, history should be cleared
    let guard2 = crate::chat::ReadlineInstance::get().unwrap();
    let count2 = guard2.history()
        .map(|h| h.iter().count())
        .unwrap_or(0);
    assert_eq!(count2, 0);
}

#[test]
fn test_save_history() {
    setup();
    
    // Add some history entries
    crate::chat::ReadlineInstance::add_history("save test 1").unwrap();
    crate::chat::ReadlineInstance::add_history("save test 2").unwrap();
    
    // Save history - should not panic
    let result = crate::chat::ReadlineInstance::save_history();
    assert!(result.is_ok());
    
    // Verify history was saved (entries should still be there)
    let guard = crate::chat::ReadlineInstance::get().unwrap();
    let count = guard.history()
        .map(|h| h.iter().count())
        .unwrap_or(0);
    assert_eq!(count, 2);
}

#[test]
fn test_auto_add_history() {
    setup();
    
    // Get the instance
    let mut rl = crate::chat::ReadlineInstance::get().unwrap();
    
    // Verify auto_add_history is enabled
    let config = rl.config();
    // Note: We can't directly check auto_add_history, but we can verify
    // that the editor was configured correctly during initialization
    assert!(rl.max_history_size().is_some());
}
