---
name: functional-core-imperative-shell
description: Use when writing or refactoring code - enforces separation of pure business logic (Functional Core) from side effects (Imperative Shell) using the FCIS pattern
---

# Functional Core, Imperative Shell (FCIS)

Adapted from ed3d-plugins (https://github.com/ed3dai/ed3d-plugins) by Ed Ropple.
Licensed under CC-BY-SA 4.0.

## Overview

**Core principle:** Separate pure business logic (Functional Core) from side effects (Imperative Shell). Pure functions go in one place, I/O operations in another.

**Why this matters:** Pure functions are trivial to test (no mocks needed). I/O code is isolated to thin shells. Bugs become structurally impossible when business logic has no side effects.

## When to Use

**Use FCIS when:**
- Writing any new code
- Refactoring existing code
- Reviewing code for architectural decisions
- Deciding where logic belongs

**Trigger symptoms:**
- "Where should this function go?"
- Creating a new file/module
- Adding database calls to logic
- Adding file I/O to calculations
- Writing tests that need complex mocking

## File Type Definitions

### Functional Core

**Contains ONLY:**
- Pure functions (same input -> same output, always)
- Business logic, validations, calculations, transformations
- Data structure operations
- Logging (EXCEPTION: loggers are permitted in Functional Core)

**NEVER contains:**
- File I/O (reading, writing files)
- Database operations (queries, updates, connections)
- HTTP requests or responses
- Environment variable access
- Non-deterministic functions (current time, random)
- State mutations outside function scope

**Logging exception:** Functions MAY accept and use loggers. For unit tests, pass no-op loggers. This is the ONLY permitted side effect in Functional Core.

**Test signature:** Simple assertions, no mocks except logger (if used).

### Imperative Shell

**Contains ONLY:**
- I/O operations: file system, database, HTTP, environment
- Orchestration: gather data -> call Functional Core -> persist results
- Error handling for I/O failures
- Minimal business logic (coordination only)

**NEVER contains:**
- Complex calculations
- Business rule validations
- Data transformations beyond format conversion

**Test signature:** Integration tests with real dependencies or test doubles.

## Code Flow Pattern

```
1. GATHER (Shell):  Collect data from external sources
2. PROCESS (Core):  Transform input to output (pure)
3. PERSIST (Shell): Save results externally
```

**Every operation follows this sequence.** No exceptions.

## Decision Framework

Before writing a function, ask:

- Can this logic run without file system, database, network, or environment?
  - **YES** -> Functional Core
  - **NO** -> Does it coordinate I/O or contain business logic?
    - **I/O coordination** -> Imperative Shell
    - **Business logic + I/O** -> STOP. Refactor to separate them.

## Common Mistakes and Rationalizations

| Excuse | Reality | What To Do |
|--------|---------|------------|
| "Just one file read in this calculation" | File I/O = side effect. Not Functional Core. | Extract to Shell. Pass data as parameter. |
| "Database is passed as parameter, so it's pure" | Database operations are I/O. Not pure. | Move to Shell. Core receives data, not DB connection. |
| "This validation needs to check if file exists" | File system check = I/O. Not Functional Core. | Shell checks file, passes boolean to Core validation. |
| "Small HTTP call, won't hurt" | HTTP = side effect. Breaks purity guarantee. | Shell makes request, Core processes response data. |
| "Need current timestamp for calculation" | Non-deterministic. Not pure. | Shell passes timestamp as parameter. |
| "Logging is a side effect, should remove" | **WRONG.** Logging is explicitly permitted. | Keep logger. This is the exception. |
| "This function does both logic and I/O, but it's simpler" | Mixed concerns = untestable without mocks. | Split into Core (logic) + Shell (I/O). Test Core simply. |
| "Performance requires mixing" | Prove it with benchmarks. Usually wrong. | Separate first. Optimize with evidence. |

## Red Flags - STOP and Refactor

If you catch yourself doing ANY of these, STOP:

- **File I/O in a "pure" function** (open, read, write, exists checks)
- **Database passed as parameter to Functional Core** (queries, updates, connections)
- **HTTP requests in business logic** (fetch, requests)
- **Environment variables in calculations**
- **Random or current-time in Functional Core** (non-deterministic)
- **Thinking "just this once" about mixing concerns**

**All of these mean:** Extract I/O to Shell. Pass data to Core.

## Refactoring Patterns

### Extract Pure Core from Impure Functions

**Symptom:** Function mixes I/O with logic

```rust
// BEFORE - hard to test
fn process_order(db: &Database, order_id: &str) -> Result<()> {
    let order = db.fetch(order_id)?;           // I/O
    let discount = calculate_discount(&order);  // Pure logic
    let total = apply_discount(&order, discount);  // Pure logic
    db.save(order_id, total)?;                 // I/O
    Ok(())
}

// AFTER - pure core extracted
fn calculate_order_total(order: &Order, rules: &DiscountRules) -> Decimal {
    // Pure function - easy to test
    let discount = calculate_discount(order, rules);
    apply_discount(order, discount)
}

fn process_order(db: &Database, order_id: &str) -> Result<()> {
    // Thin I/O wrapper
    let order = db.fetch(order_id)?;
    let total = calculate_order_total(&order, &get_discount_rules());
    db.save(order_id, total)?;
    Ok(())
}
```

### Return Values Instead of Mutating

```rust
// BEFORE - mutation
fn sort_tasks(tasks: &mut Vec<Task>) {
    tasks.sort_by_key(|t| t.priority);
}

// AFTER - returns new value
fn sorted_tasks(tasks: &[Task]) -> Vec<Task> {
    let mut sorted = tasks.to_vec();
    sorted.sort_by_key(|t| t.priority);
    sorted
}
```

### Replace Hardcoded Dependencies

```rust
// BEFORE - uses global
fn validate_input(data: &str) -> bool {
    data.len() <= CONFIG.max_length
}

// AFTER - dependency injected
fn validate_input(data: &str, max_length: usize) -> bool {
    data.len() <= max_length
}
```

### Refactoring Priority

| Pattern | Impact | Effort | Priority |
|---------|--------|--------|----------|
| Extract pure core | HIGH | Medium | Do first |
| Add missing inverse | HIGH | Low | Quick win |
| Return instead of mutate | MEDIUM | Low | Easy improvement |
| Inject dependencies | MEDIUM | Medium | When testing blocked |

## Summary

**FCIS in three rules:**

1. **Functional Core:** Pure functions only. No I/O except logging. Easy to test.
2. **Imperative Shell:** I/O coordination only. Minimal logic. Calls Core.
3. **When in doubt:** Can it run without external dependencies? -> Functional Core. Otherwise -> Imperative Shell.

**Logging exception:** Loggers permitted everywhere. Pass no-op logger for unit tests.

**Mixed concerns = refactoring needed.** Extract, separate. Do it now, not later.
