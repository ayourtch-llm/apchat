// Simple test to verify InputSourceManager compiles and works
use std::sync::Arc;
use tokio::task::JoinHandle;

use apchat::mspc::MspcChannel;
use apchat::input_router::InputSourceManager;

#[tokio::test]
async fn test_manager_creation_and_cleanup() {
    // Create a new manager
    let mut manager = InputSourceManager::new();
    
    // Verify initial state
    assert!(manager.terminal_reader.is_none());
    assert!(manager.webex_reader.is_none());
    assert!(manager.websocket_handlers.is_empty());
    assert!(!manager.has_active_readers());
    assert_eq!(manager.active_reader_count(), 0);
    
    // Create a dummy task
    let handle = tokio::spawn(async {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });
    
    // Add it to the manager
    manager.terminal_reader = Some(handle);
    
    // Verify it's active
    assert!(manager.has_active_readers());
    assert_eq!(manager.active_reader_count(), 1);
    
    // Cleanup should work without panicking
    manager.cleanup().await;
    
    // Verify cleanup worked
    assert!(manager.terminal_reader.is_none());
    assert!(!manager.has_active_readers());
    assert_eq!(manager.active_reader_count(), 0);
}
