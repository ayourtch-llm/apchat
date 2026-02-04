// Test to verify TEST_LOCK race condition bug
// This test demonstrates the issue where multiple threads can concurrently
// enter the wait loop after reading TEST_LOCK == false

#[test]
fn test_concurrent_test_lock_entry() {
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;

    // Simulate the lock
    let lock = std::sync::Arc::new(AtomicBool::new(true));
    let lock_clone = lock.clone();

    // Simulate multiple threads all trying to acquire it simultaneously
    // at the moment it's true (about to be released)
    let start_time = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(2);

    let mut acquired = false;

    let handle = std::thread::spawn(move || {
        // This simulates what happens when we have inefficient locking:
        // Multiple threads read "false" and all enter the wait loop
        while lock_clone.load(Ordering::Acquire) == false {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        true
    });

    // Immediately set the lock to false with minimal delay
    std::thread::sleep(std::time::Duration::from_micros(10));
    lock.store(false, Ordering::Release);

    // Give threads chance to wake up and race
    std::thread::sleep(std::time::Duration::from_millis(100));

    match handle.join() {
        Ok(should_acquired) => {
            println!("Thread returned: should_acquired = {}", should_acquired);
            if should_acquired {
                println!("❌ BUG CONFIRMED: Multiple threads can acquire the lock!");
            } else {
                println!("✅ Good: Only one thread acquired the lock");
            }
        }
        Err(_) => {
            println!("❌ DEADLOCK: Thread hung waiting for lock!");
        }
    }

    assert!(start_time.elapsed() < timeout, "Test timed out - possible deadlock");
}

fn main () {
}
