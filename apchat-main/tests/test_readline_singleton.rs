// Test to verify readline singleton pattern
use anyhow::Result;

fn main() -> Result<()> {
    // Test 1: Initialize and get the instance
    println!("Testing readline singleton pattern...");
    
    // Get the singleton instance
    let rl1 = crate::chat::ReadlineInstance::get()?;
    println!("✓ Got readline instance");
    
    // Verify it's initialized
    assert!(crate::chat::ReadlineInstance::is_initialized());
    println!("✓ Instance is initialized");
    
    // Get the instance again - should be the same
    let rl2 = crate::chat::ReadlineInstance::get()?;
    println!("✓ Got readline instance again");
    
    // Both should have the same configuration
    assert_eq!(rl1.max_history_size(), rl2.max_history_size());
    println!("✓ Both instances have same configuration");
    
    println!("\nAll tests passed! ✓");
    Ok(())
}
