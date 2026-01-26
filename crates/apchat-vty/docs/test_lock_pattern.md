# Test Lock Pattern for Readline Singleton

## Overview

The `ReadlineInstance` now includes a binary test lock (`TEST_LOCK`) that ensures tests run in sequence when they need exclusive access to the readline singleton. This prevents race conditions when multiple tests try to use the singleton concurrently.

## How It Works

1. **Initial State**: `TEST_LOCK` starts at `false`
2. **Test Acquisition**: Tests call `ReadlineInstance::try_take_test_lock()`
   - Blocks (spins-wait) until lock is released by a previous test
   - Sets lock to `true` when acquired
3. **Test Execution**: Test runs with exclusive access to the singleton
4. **Lock Release**: Test calls `ReadlineInstance::release_test_lock()`
   - Sets lock back to `false` to allow next test to proceed

## Atomic Ordering

- **Acquire (load)**: Ensures test sees all writes from previous test
  ```rust
  while TEST_LOCK.load(Ordering::Acquire) == false { ... }
  ```
- **Release (store)**: Marks lock as available to other tests
  ```rust
  TEST_LOCK.store(false, Ordering::Release);
  ```

## Usage Pattern

### Test 1: The "Poster" (initializes singleton)

```rust
#[test]
fn test_init_singleton() -> Result<()> {
    // Wait until lock is available
    ReadlineInstance::try_take_test_lock();

    // Now safe to initialize singleton
    let mut guard = ReadlineInstance::get()?;
    guard.clear_history_for_tests_only();

    // Do initialization work...

    // IMPORTANT: Release lock when done
    ReadlineInstance::release_test_lock();

    Ok(())
}
```

### Test 2: Uses initialized singleton

```rust
#[test]
fn test_use_singleton() -> Result<()> {
    // Wait for initialization to complete
    ReadlineInstance::try_take_test_lock();

    // Use safely initialized singleton
    let guard = ReadlineInstance::get()?;
    // assert!...

    // Release lock
    ReadlineInstance::release_test_lock();

    Ok(())
}
```

## Example Sequence

```rust
// Test A (starts first)
test_early_setup()
  → try_take_lock()     [BLOCKS until lock released]
  → initializes singleton
  → release_lock()      [unblocks Test B]

// Test B (second)
test_early_setup()
  → try_take_lock()     [blocks briefly then gets lock]
  → uses singleton
  → release_lock()

// Test C (third)
test_early_setup()
  → try_take_lock()     [blocks briefly then gets lock]
  → uses singleton
  → release_lock()
```

## Migration Guide

### Old Pattern (causes race conditions)

```rust
#[test]
fn test_feature_a() -> Result<()> {
    // No coordination - might run concurrently with other tests
    let guard = ReadlineInstance::get()?;
    // ... use singleton
    Ok(())
}
```

### New Pattern (sequential execution)

```rust
#[test]
fn test_feature_a() -> Result<()> {
    // Wait your turn
    ReadlineInstance::try_take_test_lock();
    defer!(ReadlineInstance::release_test_lock(), true);
    
    // Now safe
    let guard = ReadlineInstance::get()?;
    // ... use singleton
    Ok(())
}
```

**Use `defer!` macro or explicit release in `finally` block to ensure cleanup.**

## Key Points

- ❌ **Don't forget to release:** Forgetting to call `release_test_lock()` will block all subsequent tests
- ✅ **Just like a mutex:** The lock follows mutual exclusion semantics
- ✅ **Thread-safe:** Uses atomic operations with proper memory ordering
- ✅ **Minimal overhead:** 1ms sleep per spin, negligible for real test durations
- ✅ **Sequential only:** Truly serial execution, no parallelism between tests that use the lock

## Test Integration

To integrate this into your existing tests:

1. **Add at start of each test needing exclusive access:**
   ```rust
   ReadlineInstance::try_take_test_lock();
   ```

2. **Add at end (with automatic cleanup):**

   Option A - With macro:
   ```rust
   defer!(ReadlineInstance::release_test_lock(), true);
   ```

   Option B - In finally block:
   ```rust
   ReadlineInstance::try_take_test_lock();
   defer!(ReadlineInstance::release_test_lock(), true);
   // ... test code
   ```

   Option C - Explicit at end:
   ```rust
   ReadlineInstance::try_take_test_lock();
   // ... test code
   ReadlineInstance::release_test_lock();
   ```

## Related Tests

See `tests/test_readline_singleton.rs` and related test files for real-world examples of this pattern in action.