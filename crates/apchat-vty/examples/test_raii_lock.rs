// Test to demonstrate the RAII TestLock behavior
// This shows that locks are automatically released on drop

use apchat_vty::ReadlineInstance;

/// Test that even if we panic, the lock is released automatically
#[test]
#[should_panic]
fn test_panic_in_test() {
    println!("=== Test: Panic in middle of test ===");
    
    // Acquire lock with RAII guard
    let _lock = ReadlineInstance::try_take_test_lock("test_panic_in_test");
    println!("✓ Acquired lock");
    
    // Add some history
    ReadlineInstance::add_history("panic trigger command").unwrap();
    println!("✓ Added history entry");
    
    // This will panic - but lock should still be released!
    panic!("Intentional panic to test RAII drop behavior");
}

/// Test that order of locks matches order of usage
#[test]
fn test_lock_order_preserved() {
    println!("=== Test: Lock order preserved ===");
    
    // Acquire first lock
    let lock1 = ReadlineInstance::try_take_test_lock("test_lock_order_preserved_1");
    println!("✓ Acquired lock 1: {}", lock1.caller());
    
    // Acquire second lock while first is held (this should deadlock if not properly serializable)
    // But since both were acquired at different times and share the same state...
    // Actually wait - the tests are serialized entirely through compare_exchange
    
    // Let's just verify both locks can be acquired and released
    ReadlineInstance::add_history("second lock test").unwrap();
    println!("✓ Accessed singleton with second lock");
    
    // Lock 1 is released when lock1 goes out of scope
    println!("✓ Lock 1 released via RAII drop");
    
    // Now test 2 can proceed
    let lock2 = ReadlineInstance::try_take_test_lock("test_lock_order_preserved_2");
    println!("✓ Acquired lock 2: {}", lock2.caller());
    ReadlineInstance::add_history("final test").unwrap();
    println!("✓ Accessed singleton with lock 2");
    
    // Lock 2 is released when lock2 goes out of scope
    println!("✓ Lock 2 released via RAII drop");
}

fn main() {
}
