# Readline-Safe Output Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable `print_with_emoji` to output text above the readline prompt without interfering with user input when readline is active.

**Architecture:** 
1. Add `is_readline_active` boolean field to `Readline` struct to track when readline is blocking
2. Add `output_text()` method to `Readline` that uses terminal escape sequences to scroll output above the prompt
3. Add `get_full_height()` method to expose editor height for cursor positioning
4. Use a global callback to bridge `apchat-vty` (low-level crate) and `apchat-main` (application) without creating circular dependency

**Tech Stack:**
- Rust, crossterm terminal manipulation, ANSI escape sequences
- External crate: scopeguard (for RAII-style flag cleanup)
- Files: `crates/apchat-vty/src/readline.rs`, `crates/apchat-vty/src/lib.rs`, `apchat-main/src/chat/readline_instance.rs`, `apchat-main/src/main.rs`
- Testing: Unit tests in `crates/apchat-vty/src/readline.rs`

---


## Task 1: Add `is_readline_active` field to `Readline` struct

**Files:**
- Modify: `crates/apchat-vty/src/readline.rs` (Readline struct definition)

**Step 1: Locate the Readline struct definition**

Open the file and find the struct definition:
```bash
grep -n "pub struct Readline" crates/apchat-vty/src/readline.rs
```

You should see output like: `317:pub struct Readline {`

**Step 2: Add the field to the struct**

At line ~317, add the `is_readline_active` field at the end of the struct, just before the closing brace:

```rust
pub struct Readline {
    /// ... existing fields above ...
    /// Compatibility field: cursor position (deprecated, use cursor_col)
    cursor: usize,
    /// Whether readline is currently blocking in a readline() call
    is_readline_active: bool,
}
```

**Step 3: Initialize the field in `new()` constructor**

Find the `new()` method (around line 327):
```bash
grep -n "pub fn new()" crates/apchat-vty/src/readline.rs
```

Add initialization at the end of the struct construction, just before the closing `)`:

```rust
pub fn new() -> io::Result<Self> {
    let original_termios = Some(enable_raw_mode_on_stdin()?);

    Ok(Readline {
        // ... existing initializations ...
        cursor: 0,
        is_readline_active: false,
    })
}
```

**Step 4: Verify code compiles**

```bash
cd crates/apchat-vty
cargo check
```

Expected: Compiles successfully

**Step 5: Commit**

```bash
git add crates/apchat-vty/src/readline.rs
git commit -m "feat: add is_readline_active field to Readline struct"
```

---

## Task 2: Add `get_full_height()` method to `Readline`

**Files:**
- Modify: `crates/apchat-vty/src/readline.rs` (add to impl Readline block)

**Step 1: Find the impl Readline block after new()**

```bash
grep -n "impl Readline" crates/apchat-vty/src/readline.rs
```

Add the method after the `new()` method (around line 370+).

**Step 2: Add the getter method**

```rust
impl Readline {
    // ... existing methods ...

    /// Returns the full editor height in lines
    ///
    /// This is the number of lines the readline editor occupies,
    /// which can be > 1 for multiline input.
    pub fn get_full_height(&self) -> usize {
        self.editor_height
    }
}
```

**Step 3: Add unit test for `get_full_height()`**

Find the `mod tests` block at the end of the file (around line 2036+):
```bash
grep -n "#\[cfg(test)\]" crates/apchat-vty/src/readline.rs
```

Add this test inside the `mod tests` block:

```rust
#[test]
fn test_get_full_height() {
    let mut readline = Readline::new().unwrap();

    // Initially editor height is 1
    assert_eq!(readline.get_full_height(), 1);

    // After adding lines, height increases
    readline.lines = vec!["line1".to_string(), "line2".to_string(), "line3".to_string()];
    readline.editor_height = 3;
    assert_eq!(readline.get_full_height(), 3);
}
```

**Step 4: Run test to verify it passes**

```bash
cd crates/apchat-vty
cargo test test_get_full_height
```

Expected: `test test_get_full_height ... ok`

**Step 5: Commit**

```bash
git add crates/apchat-vty/src/readline.rs
git commit -m "feat: add get_full_height() method to Readline"
```

---

## Task 3: Implement `output_text()` method in `Readline`

**Files:**
- Modify: `crates/apchat-vty/src/readline.rs` (add to impl Readline block)

**Step 1: Add helper function for direct output (fallback)**

Add this function at the top of the file, after the imports (around line 35+). It's used as a fallback when readline is not active.

```rust
/// Direct output function (fallback when readline not available)
fn print_with_emoji_direct(emoji: &str, text: &str, newline: bool, mut writer: impl io::Write) {
    let lines: Vec<&str> = text.split('\n').collect();

    for (i, line) in lines.iter().enumerate() {
        if i < lines.len() - 1 {
            let _ = writeln!(writer, "{} {}", emoji, line);
        } else {
            let _ = write!(writer, "{} {}", emoji, line);
        }
    }

    if newline {
        let _ = writeln!(writer);
    }

    let _ = writer.flush();
}
```

**Step 2: Add `output_text()` method to Readline impl block**

Add this method after `get_full_height()` in the `impl Readline` block:

```rust
impl Readline {
    // ... existing methods ...

    /// Output text while preserving the readline prompt
    ///
    /// If readline is currently active (blocking in readline() call),
    /// this method uses terminal escape sequences to scroll output
    /// above the readline prompt without interfering with user input.
    ///
    /// # Arguments
    /// * `emoji` - The emoji to prepend to each line
    /// * `text` - The text to output (may contain newlines)
    /// * `newline` - Whether to add trailing newline
    /// * `writer` - The output destination (stdout or stderr)
    ///
    /// # Returns
    /// * `io::Result<()>` - Ok on success, Err on write failure
    pub fn output_text(&mut self, emoji: &str, text: &str, newline: bool, mut writer: impl io::Write) -> io::Result<()> {
        // If readline is not active, use direct output
        if !self.is_readline_active {
            print_with_emoji_direct(emoji, text, newline, writer);
            return Ok(());
        }

        // Save cursor position (at the readline prompt)
        write!(writer, "\x1b[s")?;

        // Move to bottom of output area (line above readline editor)
        write!(writer, "\x1b[0G")?;  // Move to column 0
        write!(writer, "\x1b[{}A", self.editor_height)?;  // Move up by editor height

        // Split text into lines
        let lines: Vec<&str> = text.split('\n').collect();

        // For each output line: insert line, move to col 0, print
        for (i, line) in lines.iter().enumerate() {
            // Insert line (scrolls content above upward, makes room for new line)
            write!(writer, "\x1b[L")?;
            write!(writer, "\x1b[0G")?;  // Move to column 0

            // Print emoji and line content
            if i < lines.len() - 1 {
                writeln!(writer, "{} {}", emoji, line)?;
            } else {
                write!(writer, "{} {}", emoji, line)?;
            }
        }

        // Add trailing newline if requested
        if newline {
            writeln!(writer)?;
        }

        // Restore cursor position (back to readline prompt)
        write!(writer, "\x1b[u")?;
        writer.flush()?;

        Ok(())
    }
}
```

**Step 3: Add unit test for `output_text()` when readline inactive**

Add this test in the `mod tests` block:

```rust
#[test]
fn test_output_text_when_inactive() {
    let mut readline = Readline::new().unwrap();

    // is_readline_active is false by default
    assert!(!readline.is_readline_active);

    // Create a buffer to capture output
    let mut output = Vec::new();

    // Should use direct output (no escape sequences)
    let result = readline.output_text("❤️", "test line", true, &mut output);
    assert!(result.is_ok());

    let output_str = String::from_utf8(output).unwrap();
    assert!(output_str.contains("❤️ test line"));
    // Should NOT contain escape sequences when inactive
    assert!(!output_str.contains("\x1b"));
}
```

**Step 4: Run tests to verify they pass**

```bash
cd crates/apchat-vty
cargo test test_output_text
```

Expected: All tests pass

**Step 5: Commit**

```bash
git add crates/apchat-vty/src/readline.rs
git commit -m "feat: add output_text() method to Readline"
```

---
