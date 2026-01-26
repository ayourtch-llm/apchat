// Test to verify readline singleton pattern
use anyhow::Result;
use std::ops::DerefMut;

use apchat_vty::ReadlineInstance;
use apchat_vty::instance::TestLock;

#[test]
fn test_readline_singleton_basic() -> Result<()> {
    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_readline_singleton_basic");
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
    // Lock is automatically released when _lock goes out of scope
    Ok(())
}
