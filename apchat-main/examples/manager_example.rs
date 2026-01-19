// Example usage of InputSourceManager
use std::sync::Arc;

use apchat::mspc::MspcChannel;
use apchat::input_router::InputSourceManager;

#[tokio::main]
async fn main() {
    // Create a new manager
    let mut manager = InputSourceManager::new();
    
    println!("Created InputSourceManager");
    println!("Initial state:");
    println!("  - Terminal reader: {}", manager.terminal_reader.is_none());
    println!("  - Webex reader: {}", manager.webex_reader.is_none());
    println!("  - Websocket handlers: {}", manager.websocket_handlers.is_empty());
    println!("  - Has active readers: {}", !manager.has_active_readers());
    println!("  - Active reader count: {}", manager.active_reader_count());
    
    // Simulate adding a terminal reader
    let channel = Arc::new(MspcChannel::new(100));
    let handle = tokio::spawn(async {
        println!("Terminal reader task started");
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        println!("Terminal reader task completed");
    });
    
    manager.terminal_reader = Some(handle);
    
    println!("\nAfter adding terminal reader:");
    println!("  - Has active readers: {}", manager.has_active_readers());
    println!("  - Active reader count: {}", manager.active_reader_count());
    
    // Cleanup
    manager.cleanup().await;
    
    println!("\nAfter cleanup:");
    println!("  - Has active readers: {}", manager.has_active_readers());
    println!("  - Active reader count: {}", manager.active_reader_count());
    
    println!("\nInputSourceManager example completed successfully!");
}
