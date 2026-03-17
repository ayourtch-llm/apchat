// Test to verify readline singleton pattern
mod pty_test_helper;

use anyhow::Result;
use std::ops::DerefMut;

use apchat_vty::ReadlineInstance;
use apchat_vty::instance::TestLock;

#[test]
fn test_readline_singleton_basic() -> Result<()> {
    pty_test_helper::ensure_pty_stdin();

    // Acquire test lock with RAII guard - releases automatically on drop
    let _lock = TestLock::acquire("test_readline_singleton_basic");
    println!("Testing readline singleton pattern...");

    let history1 = {
        // Get the singleton instance
        let mut guard = apchat_vty::ReadlineInstance::get()?;
        println!("Got readline instance");
        guard.get_history_entries().len()
    };

   let history2 = {
        // Get the singleton instance
        let mut guard = apchat_vty::ReadlineInstance::get()?;
        println!("Got readline instance again");
        guard.get_history_entries().len()
    };

    // Both should have the same configuration
    assert_eq!(history1, history2);
    println!("Both instances have same configuration");

    println!("\nAll tests passed!");
    // Lock is automatically released when _lock goes out of scope
    Ok(())
}
