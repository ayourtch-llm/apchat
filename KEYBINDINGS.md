# Readline Keybindings Reference

## Movement Keys

| Key | Action |
|-----|--------|
| `Left` | Move cursor left one character |
| `Right` | Move cursor right one character |
| `Ctrl-Left` | Move cursor left one word |
| `Ctrl-Right` | Move cursor right one word |
| `Home` | Move cursor to start of line |
| `End` | Move cursor to end of line |
| `Ctrl-A` | Move cursor to start of line ✨ NEW |
| `Ctrl-E` | Move cursor to end of line ✨ NEW |
| `Up` | Navigate to previous history entry |
| `Down` | Navigate to next history entry |

## Editing Keys

| Key | Action |
|-----|--------|
| `Backspace` | Delete character before cursor |
| `Delete` | Delete character at cursor |
| `Ctrl-K` | Kill (cut) text from cursor to end of line |
| `Ctrl-U` | Kill (cut) text from start to cursor |
| `Ctrl-W` | Kill (cut) word to the left of cursor |
| `Alt-D` | Kill (cut) word to the right of cursor |
| `Ctrl-Y` | Yank (paste) last killed text |

## Special Keys

| Key | Action |
|-----|--------|
| `Enter` | Submit the current line |
| `Ctrl-C` | Interrupt (sends interrupt signal) |
| `Ctrl-D` | Exit if line is empty, otherwise delete character at cursor |
| `Ctrl-R` | Enter reverse search mode |

## Paste Behavior ✨ NEW

When you paste multiline text into the REPL, the newlines are automatically converted to spaces to keep the input on a single line.

### Example:
```
Pasting this:    Becomes this:
line 1           line 1 line 2 line 3
line 2
line 3
```

This works by using **bracketed paste mode**, which allows the terminal to distinguish between pasted content and manually typed input.

### Technical Details:
- Bracketed paste mode is automatically enabled when the REPL starts
- The terminal wraps pasted content in special escape sequences
- Our implementation detects these sequences and processes them specially
- Newlines are replaced with spaces, and multiple spaces are collapsed

## History Navigation

- `Up`/`Down`: Navigate through command history
- `Ctrl-R`: Search backward through history
- While searching:
  - Type characters to filter history
  - `Ctrl-R` to cycle through matches
  - `Ctrl-C`, `Ctrl-G`, or `Esc` to exit search mode

## Notes

- All keybindings match standard GNU readline behavior
- Bracketed paste mode requires terminal support (most modern terminals support it)
- Terminals without bracketed paste support will paste as raw keystrokes
- Kill ring (copy/paste buffer) holds up to 60 entries by default
