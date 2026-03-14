# Unified Diff Display Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the verbose edit display format in `edit_file` and `plan_edits` tools with compact unified diff format using the `similar` crate.

**Architecture:** Create a helper function that generates unified diff output using the `similar` crate, then replace the current line-by-line display logic in both `edit_file` and `plan_edits` tools.

**Tech Stack:** Rust, `similar` crate (already in dependencies at version 2.6 with inline feature)

---

## Task 1: Create diff generation helper function

**Files:**
- Create: `crates/apchat-tools/src/diff_utils.rs`
- Modify: `crates/apchat-tools/src/lib.rs`

**Step 1: Create the diff utility module**

Create a new file `crates/apchat-tools/src/diff_utils.rs`:

```rust
use similar::{ChangeTag, TextDiff};

/// Generate a unified diff string between old and new content
/// Returns a formatted diff string suitable for terminal display
pub fn generate_unified_diff(old_content: &str, new_content: &str) -> String {
    let diff = TextDiff::from_lines(old_content, new_content);
    let mut result = String::new();
    
    for change in diff.iter_all_changes() {
        let (sign, symbol) = match change.tag() {
            ChangeTag::Delete => ("-", colored::Colorize::red),
            ChangeTag::Insert => ("+", colored::Colorize::green),
            ChangeTag::Equal => (" ", colored::Colorize::bright_black),
        };
        
        result.push_str(&format!("{}{}{}", symbol, sign, change));
    }
    
    result
}

/// Generate a compact unified diff with line numbers (git-style)
pub fn generate_unified_diff_with_context(old_content: &str, new_content: &str) -> String {
    let diff = TextDiff::from_lines(old_content, new_content);
    let mut result = String::new();
    
    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            result.push_str("---\n");
        }
        
        let (old_start, old_end, new_start, new_end) = group.range_format();
        result.push_str(&format!("@@ -{} +{} @@\n", old_start, new_start));
        
        for change in group.iter_changes() {
            let (sign, symbol) = match change.tag() {
                ChangeTag::Delete => ("-", colored::Colorize::red),
                ChangeTag::Insert => ("+", colored::Colorize::green),
                ChangeTag::Equal => (" ", colored::Colorize::bright_black),
            };
            
            result.push_str(&format!("{}{}{}", symbol, sign, change));
        }
    }
    
    result
}
```

**Step 2: Add module to lib.rs**

Modify `crates/apchat-tools/src/lib.rs` - add this line:

```rust
mod diff_utils;
```

**Step 3: Verify compilation**

Run: `cargo build --package apchat-tools`
Expected: Successful compilation

**Step 4: Commit**

```bash
git add crates/apchat-tools/src/diff_utils.rs crates/apchat-tools/src/lib.rs
git commit -m "feat: add diff utility functions using similar crate"
```

---

## Task 2: Update edit_file tool to use unified diff

**Files:**
- Modify: `crates/apchat-tools/src/file_ops.rs:271-285`

**Step 1: Replace the diff display section**

Find lines 271-285 in `crates/apchat-tools/src/file_ops.rs` (the section that starts with `print_heart_red(&format!("{}", "─ Old content:".red()), true);`)

Replace the entire block:

```rust
        // Simple diff display
        print_heart_red(&format!("{}", "─ Old content:".red()), true);
        for line in old_content.lines() {
            print_heart_red(&format!("{} {}", "-".red(), line), true);
        }
        print_heart_red(&format!(""), true);
        print_heart_red(&format!("{}", "+ New content:".green()), true);
        for line in new_content.lines() {
            print_heart_red(&format!("{} {}", "+".green(), line), true);
        }
```

With:

```rust
        // Unified diff display
        let diff = diff_utils::generate_unified_diff(&old_content, &new_content);
        print_heart_red(&format!("{}", diff), true);
```

**Step 2: Add use statement**

At the top of `crates/apchat-tools/src/file_ops.rs`, ensure the module is accessible (it should be via the lib.rs mod declaration, but if needed):

```rust
use crate::diff_utils;
```

**Step 3: Verify compilation**

Run: `cargo build --package apchat-tools`
Expected: Successful compilation

**Step 4: Test the edit_file tool**

Run: `cargo run -- --help`
Expected: Application builds and runs

**Step 5: Commit**

```bash
git add crates/apchat-tools/src/file_ops.rs
git commit -m "feat: use unified diff format in edit_file tool"
```

---

## Task 3: Update plan_edits tool to use unified diff

**Files:**
- Modify: `crates/apchat-tools/src/model_management.rs:560-580`

**Step 1: Find the diff display section in plan_edits**

Locate the section around line 560-580 that displays old/new content (look for the pattern with `print_heart_red` showing lines with `-` and `+` prefixes).

**Step 2: Replace with unified diff**

Replace the old content display logic with:

```rust
            // Show unified diff
            let diff = diff_utils::generate_unified_diff(&edit.old_content, &edit.new_content);
            print_heart_red(&format!("{}\n", diff), true);
```

**Step 3: Add use statement if needed**

At the top of `crates/apchat-tools/src/model_management.rs`:

```rust
use crate::diff_utils;
```

**Step 4: Verify compilation**

Run: `cargo build --package apchat-tools`
Expected: Successful compilation

**Step 5: Commit**

```bash
git add crates/apchat-tools/src/model_management.rs
git commit -m "feat: use unified diff format in plan_edits tool"
```

---

## Task 4: Test both tools end-to-end

**Step 1: Create a test file**

```bash
echo "fn hello() {
    println!(\"Hello, World!\");
}" > /tmp/test_display.rs
```

**Step 2: Test edit_file with a simple change**

Use the application to edit the test file and verify the diff display is compact and readable.

**Step 3: Test plan_edits with multiple changes**

Create a plan with multiple edits and verify all diffs display correctly.

**Step 4: Verify no regressions**

Run: `cargo test --package apchat-tools`
Expected: All tests pass

**Step 5: Commit**

```bash
git commit -am "test: verify unified diff display works correctly"
```

---

## Success Criteria

- ✅ `edit_file` displays compact unified diff instead of verbose line-by-line format
- ✅ `plan_edits` displays compact unified diff for each planned edit
- ✅ Diff shows context lines (unchanged) in dim/bright_black
- ✅ Deleted lines shown with `-` prefix in red
- ✅ Added lines shown with `+` prefix in green
- ✅ All existing tests pass
- ✅ No compilation warnings