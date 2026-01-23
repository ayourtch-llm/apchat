// Example usage of InputSourceManager
use std::sync::Arc;

use apchat::mspc::MspcChannel;
use apchat::input_router::InputSourceManager;
use apchat_vty::{print_heart_red, print_heart_yellow};

#[tokio::main]
async fn main() {
    // Create a new manager
    let mut manager = InputSourceManager::new();
    
    print_heart_red("Created InputSourceManager", true);
    print_heart_red("Initial state:", true);
    print_heart_red(&format!("  - Terminal reader: {}", manager.terminal_reader.is_none()), true);
    print_heart_red(&format!("  - Webex reader: {}", manager.webex_reader.is_none()), true);
    print_heart_red(&format!("  - Websocket handlers: {}", manager.websocket_handlers.is_empty()), true);
    print_heart_red(&format!("  - Has active readers: {}", !manager.has_active_readers()), true);
    print_heart_red(&format!("  - Active reader count: {}", manager.active_reader_count()), true);
    
    // Simulate adding a terminal reader
    let channel = Arc::new(MspcChannel::new(100));
    let handle = tokio::spawn(async {
        print_heart_red("Terminal reader task started", true);
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        print_heart_red("Terminal reader task completed", true);
    });
    
    manager.terminal_reader = Some(handle);
    
    print_heart_red("\nAfter adding terminal reader:", true);
    print_heart_red(&format!("  - Has active readers: {}", manager.has_active_readers()), true);
    print_heart_red(&format!("  - Active reader count: {}", manager.active_reader_count()), true);
    
    // Cleanup
    manager.cleanup().await;
    
    print_heart_red("\nAfter cleanup:", true);
    print_heart_red(&format!("  - Has active readers: {}", manager.has_active_readers()), true);
    print_heart_red(&format!("  - Active reader count: {}", manager.active_reader_count()), true);
    
    print_heart_red("\nInputSourceManager example completed successfully!", true);
}
