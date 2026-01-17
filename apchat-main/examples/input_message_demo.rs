// Example demonstrating all new features of InputMessage

use apchat::chat::{InputMessage, MessagePriority, MessageSource};
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== InputMessage Enhancement Demo ===\n");

    // Example 1: Basic usage (backward compatible)
    println!("1. Basic message creation:");
    let basic_msg = InputMessage::new("Hello, world!".to_string());
    println!("   Content: {}", basic_msg.content);
    println!("   Interrupt: {}", basic_msg.interrupt);
    println!("   Priority: {:?}", basic_msg.priority);
    println!("   Source: {:?}\n", basic_msg.source);

    // Example 2: High priority interrupt
    println!("2. High priority interrupt:");
    let interrupt_msg = InputMessage::high_priority_interrupt("EMERGENCY STOP!".to_string());
    println!("   Content: {}", interrupt_msg.content);
    println!("   Interrupt: {}", interrupt_msg.interrupt);
    println!("   Priority: {:?}\n", interrupt_msg.priority);

    // Example 3: File input with priority
    println!("3. File input processing:");
    let file_msg = InputMessage::new("Process this file".to_string())
        .with_source(MessageSource::File("/path/to/input.txt".to_string()))
        .with_priority(MessagePriority::High);
    println!("   Content: {}", file_msg.content);
    println!("   Source: {:?}", file_msg.source);
    println!("   Priority: {:?}\n", file_msg.priority);

    // Example 4: API webhook
    println!("4. API webhook message:");
    let api_msg = InputMessage::new("Webhook triggered".to_string())
        .with_source(MessageSource::Api)
        .with_interrupt(true);
    println!("   Content: {}", api_msg.content);
    println!("   Source: {:?}", api_msg.source);
    println!("   Interrupt: {}\n", api_msg.interrupt);

    // Example 5: Internal system message
    println!("5. Internal system notification:");
    let internal_msg = InputMessage::new("System update available".to_string())
        .with_source(MessageSource::Internal("updater.service".to_string()))
        .with_priority(MessagePriority::Normal);
    println!("   Content: {}", internal_msg.content);
    println!("   Source: {:?}\n", internal_msg.source);

    // Example 6: Priority comparison
    println!("6. Priority comparison:");
    println!("   High > Normal: {}", MessagePriority::High > MessagePriority::Normal);
    println!("   Normal < High: {}", MessagePriority::Normal < MessagePriority::High);

    // Example 7: Default implementation
    println!("\n7. Default message:");
    let default_msg: InputMessage = InputMessage::default();
    println!("   Content: '{}'", default_msg.content);
    println!("   Interrupt: {}", default_msg.interrupt);
    println!("   Priority: {:?}", default_msg.priority);
    println!("   Source: {:?}", default_msg.source);

    println!("\n=== All examples completed successfully! ===");
}
