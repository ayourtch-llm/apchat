// Example usage of the test lock pattern for readline singleton coordination
//
// This demonstrates how tests should use the try_take_test_lock() and
// release_test_lock() methods to ensure tests run in sequence without
// interfering with the readline singleton.

use anyhow::Result;
use apchat_vty::ReadlineInstance;

/// Test 1: The "Poster" test that initializes the readline instance
/// This test will run first, initialize the singleton, and release the lock.
#[test]
fn test_readline_init_setup() -> Result<()> {
    println!("=== Test 1: Initialize Setup ===");

    // Wait until the lock is available
    // Since TEST_LOCK starts at false, we'll spin-wait until it's true
    // Once here, we have exclusive access to initialize the singleton
    ReadlineInstance::try_take_test_lock();
    println!("✓ Acquired lock, initialized singleton can proceed");

    // Now we can safely initialize the singleton
    let mut guard = ReadlineInstance::get()?;
    guard.clear_history_for_tests_only();
    ReadlineInstance::add_history("initial setup command")?;

    println!("✓ Completed initialization");

    // IMPORTANT: Release the lock when done
    // This allows the next test to claim it
    ReadlineInstance::release_test_lock();
    println!("✓ Lock released\n");

    Ok(())
}

/// Test 2: Tests that depend on the properly initialized singleton
/// This test will wait for the lock to be released by test 1,
/// then safely use the initialized singleton.
#[test]
fn test_readline_singleton_basic() -> Result<()> {
    println!("=== Test 2: Basic Singleton Test ===");

    // Wait for test 1 to release the lock
    // This blocks until safe initialization is complete
    ReadlineInstance::try_take_test_lock();
    println!("✓ Acquired lock after test 1");

    // Now singleton is safely initialized - no race conditions
    let guard = ReadlineInstance::get()?;
    let history = guard.get_history_entries();

    // Verify history from test 1 is present
    assert_eq!(history.len(), 1, "Should have 1 history entry from setup");
    assert_eq!(history[0], "initial setup command", "History entry matches");

    println!("✓ Verified singleton works correctly");

    // Release lock for next test
    ReadlineInstance::release_test_lock();
    println!("✓ Lock released\n");

    Ok(())
}

/// Test 3: Simulates using the readline instance for input
#[test]
fn test_readline_input_simulation() -> Result<()> {
    println!("=== Test 3: Input Simulation ===");

    // Wait for lock
    ReadlineInstance::try_take_test_lock();

    // Simulate readline input
    let guard = ReadlineInstance::get()?;
    guard.add_history("user input test");

    // Verify we got our input
    let history = guard.get_history_entries();
    assert_eq!(history.len(), 2);

    println!("✓ Simulated input successfully");

    ReadlineInstance::release_test_lock();
    println!("✓ Lock released\n");

    Ok(())
}

/// Test 4: Tests all adopt the lock pattern
#[test]
fn test_readline_race_condition_prevention() -> Result<()> {
    println!("=== Test 4: Race Condition Prevention ===");

    // This test waits their turn in the sequence
    ReadlineInstance::try_take_test_lock();

    let guard = ReadlineInstance::get()?;
    guard.add_history("test 4 command");

    // Verify all history entries are in order
    let history = guard.get_history_entries();
    assert_eq!(history.len(), 3);
    assert_eq!(history[0], "initial setup command");
    assert_eq!(history[1], "user input test");
    assert_eq!(history[2], "test 4 command");

    println!("✓ All sequence-dependent tests ran without interference");

    ReadlineInstance::release_test_lock();
    println!("✓ Lock released\n");

    Ok(())
}

fn main() {
}
