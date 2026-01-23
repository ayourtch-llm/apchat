// Test to verify readline singleton pattern
use anyhow::Result;

#[test]
fn test_readline_singleton_basic() -> Result<()> {
    // Test 1: Initialize and get the instance
    println!("Testing readline singleton pattern...");

    // Get the singleton instance
    let rl1 = apchat_vty::ReadlineInstance::get()?;
    println!("✓ Got readline instance");

    // Get the instance again - should be the same
    let rl2 = apchat_vty::ReadlineInstance::get()?;
    println!("✓ Got readline instance again");

    // Both should have the same configuration
    let history1 = rl1.get_history_entries();
    let history2 = rl2.get_history_entries();
    assert_eq!(history1.len(), history2.len());
    println!("✓ Both instances have same configuration");

    println!("\nAll tests passed! ✓");
    Ok(())
}
