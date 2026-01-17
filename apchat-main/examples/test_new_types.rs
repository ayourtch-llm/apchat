// Test that new types can be imported and used
use apchat::chat::{InputMessage, MessagePriority, MessageSource};

fn main() {
    // Test basic construction
    let msg1 = InputMessage::new("test".to_string());
    assert!(!msg1.interrupt);
    assert_eq!(msg1.priority, MessagePriority::Normal);
    assert_eq!(msg1.source, MessageSource::StdIn);
    
    // Test builder pattern
    let msg2 = InputMessage::new("urgent".to_string())
        .with_interrupt(true)
        .with_priority(MessagePriority::High)
        .with_source(MessageSource::File("test.txt".to_string()));
    
    assert!(msg2.interrupt);
    assert_eq!(msg2.priority, MessagePriority::High);
    
    println!("All tests passed!");
}
