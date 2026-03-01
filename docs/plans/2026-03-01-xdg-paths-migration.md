# XDG Paths Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace all `~/.okaychat` references with XDG-compliant `apchat` directories using a new `ApChatPaths` API.

**Architecture:** Create `apchat-common` crate with `ApChatPaths` module providing XDG-compliant path methods. Update all 21 occurrences across 10 files to use the new API.

**Tech Stack:** Rust, `dirs` crate, XDG Base Directory Specification

---

## Prerequisites

**Worktree:** Ensure you're in a dedicated worktree for this change.

**Files to reference:**
- Current usages: See subagent investigation summary in conversation history
- XDG spec: https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html

---

### Task 1: Create `apchat-common` crate structure

**Files:**
- Create: `crates/apchat-common/Cargo.toml`
- Create: `crates/apchat-common/src/lib.rs`
- Create: `crates/apchat-common/src/paths.rs`

**Step 1: Create Cargo.toml**

```toml
[package]
name = "apchat-common"
version = "0.1.0"
edition = "2021"
license = "MIT"

[dependencies]
dirs = "5.0"
anyhow = "1.0"
```

**Step 2: Create lib.rs**

```rust
pub mod paths;

pub use paths::ApChatPaths;
```

**Step 3: Create paths.rs**

```rust
use std::path::PathBuf;
use std::fs;

pub struct ApChatPaths;

impl ApChatPaths {
    /// Base config directory (~/.config/apchat)
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("apchat")
    }

    /// Base data directory (~/.local/share/apchat)
    pub fn data_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"))
            .join("apchat")
    }

    /// Base cache directory (~/.cache/apchat)
    pub fn cache_dir() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("~/.cache"))
            .join("apchat")
    }

    /// Credentials file (config_dir/credentials.toml)
    pub fn credentials_file() -> PathBuf {
        Self::config_dir().join("credentials.toml")
    }

    /// Environment file (config_dir/env)
    pub fn env_file() -> PathBuf {
        Self::config_dir().join("env")
    }

    /// Histories directory (data_dir/histories)
    pub fn histories_dir() -> PathBuf {
        Self::data_dir().join("histories")
    }

    /// Sessions directory (data_dir/sessions)
    pub fn sessions_dir() -> PathBuf {
        Self::data_dir().join("sessions")
    }

    /// Logs directory (cache_dir/logs)
    pub fn logs_dir() -> PathBuf {
        Self::cache_dir().join("logs")
    }

    /// Fastembed cache directory (cache_dir/fastembed)
    pub fn fastembed_dir() -> PathBuf {
        Self::cache_dir().join("fastembed")
    }

    /// Candle cache directory (cache_dir/candle)
    pub fn candle_dir() -> PathBuf {
        Self::cache_dir().join("candle")
    }

    /// Ensure a directory exists, creating it if necessary
    pub fn ensure_dir(path: &PathBuf) -> std::io::Result<()> {
        fs::create_dir_all(path)
    }

    /// Initialize all base directories
    pub fn init_all() -> std::io::Result<()> {
        Self::ensure_dir(&Self::config_dir())?;
        Self::ensure_dir(&Self::data_dir())?;
        Self::ensure_dir(&Self::cache_dir())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_dir_contains_apchat() {
        let dir = ApChatPaths::config_dir();
        assert!(dir.to_string_lossy().contains("apchat"));
    }

    #[test]
    fn test_data_dir_contains_apchat() {
        let dir = ApChatPaths::data_dir();
        assert!(dir.to_string_lossy().contains("apchat"));
    }

    #[test]
    fn test_cache_dir_contains_apchat() {
        let dir = ApChatPaths::cache_dir();
        assert!(dir.to_string_lossy().contains("apchat"));
    }

    #[test]
    fn test_credentials_file_in_config() {
        let file = ApChatPaths::credentials_file();
        assert!(file.to_string_lossy().contains("apchat"));
        assert!(file.file_name().map_or(false, |n| n == "credentials.toml"));
    }

    #[test]
    fn test_histories_dir_in_data() {
        let dir = ApChatPaths::histories_dir();
        assert!(dir.to_string_lossy().contains("histories"));
    }

    #[test]
    fn test_logs_dir_in_cache() {
        let dir = ApChatPaths::logs_dir();
        assert!(dir.to_string_lossy().contains("logs"));
    }
}
```

**Step 4: Run tests**

```bash
cd crates/apchat-common
cargo test
```

Expected: All 6 tests pass

**Step 5: Commit**

```bash
git add crates/apchat-common
git commit -m "feat: add apchat-common crate with ApChatPaths XDG API"
```

---

### Task 2: Update workspace Cargo.toml

**Files:**
- Modify: `Cargo.toml` (workspace root)

**Step 1: Add apchat-common to workspace members**

Find the `members` array and add:

```toml
members = [
    # ... existing members ...
    "crates/apchat-common",
]
```

**Step 2: Verify workspace builds**

```bash
cargo build --workspace
```

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "feat: add apchat-common to workspace members"
```

---

### Task 3: Update `crates/apchat-logging/src/lib.rs`

**Files:**
- Modify: `crates/apchat-logging/src/lib.rs`
- Modify: `crates/apchat-logging/Cargo.toml`

**Step 1: Add dependency**

Edit `crates/apchat-logging/Cargo.toml`:

```toml
[dependencies]
apchat-common = { path = "../apchat-common" }
# ... other dependencies
```

**Step 2: Update lib.rs**

Find lines 32, 40, 51 and replace:

**Old pattern:**
```rust
let home_dir = dirs::home_dir().unwrap();
let okaychat_dir = home_dir.join(".okaychat");
let logs_dir = okaychat_dir.join("logs");
```

**New pattern:**
```rust
use apchat_common::ApChatPaths;

let logs_dir = ApChatPaths::logs_dir();
ApChatPaths::ensure_dir(&logs_dir)?;
```

**Step 3: Test logging crate**

```bash
cd crates/apchat-logging
cargo test
```

**Step 4: Commit**

```bash
git add crates/apchat-logging
git commit -m "refactor: use ApChatPaths in apchat-logging"
```

---

### Task 4: Update `crates/apchat-vty/src/history.rs`

**Files:**
- Modify: `crates/apchat-vty/src/history.rs`
- Modify: `crates/apchat-vty/Cargo.toml`

**Step 1: Add dependency**

Edit `crates/apchat-vty/Cargo.toml`:

```toml
[dependencies]
apchat-common = { path = "../apchat-common" }
```

**Step 2: Update history.rs**

Find lines 80, 86, 97 and replace:

**Old:**
```rust
let home_dir = dirs::home_dir().unwrap();
let okaychat_dir = home_dir.join(".okaychat");
let logs_dir = okaychat_dir.join("logs");
```

**New:**
```rust
use apchat_common::ApChatPaths;

let logs_dir = ApChatPaths::logs_dir();
```

**Step 3: Test vty crate**

```bash
cd crates/apchat-vty
cargo test
```

**Step 4: Commit**

```bash
git add crates/apchat-vty
git commit -m "refactor: use ApChatPaths in apchat-vty"
```

---

### Task 5: Update `crates/apchat-mcp-pty-server/src/main.rs`

**Files:**
- Modify: `crates/apchat-mcp-pty-server/src/main.rs`
- Modify: `crates/apchat-mcp-pty-server/Cargo.toml`

**Step 1: Add dependency**

```toml
[dependencies]
apchat-common = { path = "../apchat-common" }
```

**Step 2: Update main.rs**

Find lines 164, 501 and replace:

**Old:**
```rust
let credentials_path = home_dir.join(".okaychat").join("credentials.toml");
```

**New:**
```rust
use apchat_common::ApChatPaths;

let credentials_path = ApChatPaths::credentials_file();
```

**Step 3: Build to verify**

```bash
cd crates/apchat-mcp-pty-server
cargo build
```

**Step 4: Commit**

```bash
git add crates/apchat-mcp-pty-server
git commit -m "refactor: use ApChatPaths in apchat-mcp-pty-server"
```

---

### Task 6: Update `crates/apchat-tools/src/terminal_tools.rs`

**Files:**
- Modify: `crates/apchat-tools/src/terminal_tools.rs`
- Modify: `crates/apchat-tools/Cargo.toml`

**Step 1: Add dependency**

```toml
[dependencies]
apchat-common = { path = "../apchat-common" }
```

**Step 2: Update terminal_tools.rs**

Find lines 656, 704 and replace:

**Old:**
```rust
let credentials_path = home_dir.join(".okaychat").join("credentials.toml");
```

**New:**
```rust
use apchat_common::ApChatPaths;

let credentials_path = ApChatPaths::credentials_file();
```

**Step 3: Build to verify**

```bash
cd crates/apchat-tools
cargo build
```

**Step 4: Commit**

```bash
git add crates/apchat-tools
git commit -m "refactor: use ApChatPaths in apchat-tools terminal_tools"
```

---

### Task 7: Update `crates/apchat-tools/src/memory/db.rs`

**Files:**
- Modify: `crates/apchat-tools/src/memory/db.rs`

**Step 1: Update db.rs**

Find line 18 and replace `.okaychat` references with `ApChatPaths::data_dir()` or appropriate method.

**Step 2: Build to verify**

```bash
cd crates/apchat-tools
cargo build
```

**Step 3: Commit** (can combine with Task 6 if same crate)

```bash
git add crates/apchat-tools
git commit -m "refactor: use ApChatPaths in apchat-tools memory"
```

---

### Task 8: Update `crates/apchat-webex/src/config.rs`

**Files:**
- Modify: `crates/apchat-webex/src/config.rs`
- Modify: `crates/apchat-webex/Cargo.toml`

**Step 1: Add dependency**

```toml
[dependencies]
apchat-common = { path = "../apchat-common" }
```

**Step 2: Update config.rs**

Find lines 8-71 and replace:

**Old:**
```rust
let path = PathBuf::from("~/.okaychat/env");
```

**New:**
```rust
use apchat_common::ApChatPaths;

let path = ApChatPaths::env_file();
```

**Step 3: Test webex crate**

```bash
cd crates/apchat-webex
cargo test
```

**Step 4: Commit**

```bash
git add crates/apchat-webex
git commit -m "refactor: use ApChatPaths in apchat-webex"
```

---

### Task 9: Update `crates/apchat-skills` embeddings backends

**Files:**
- Modify: `crates/apchat-skills/src/embeddings/fastembed_backend.rs`
- Modify: `crates/apchat-skills/src/embeddings/candle_backend.rs`
- Modify: `crates/apchat-skills/Cargo.toml`

**Step 1: Add dependency**

```toml
[dependencies]
apchat-common = { path = "../apchat-common" }
```

**Step 2: Update fastembed_backend.rs**

Find line 20 and replace:

**Old:**
```rust
let dir = home_dir.join(".okaychat").join("fastembed");
```

**New:**
```rust
use apchat_common::ApChatPaths;

let dir = ApChatPaths::fastembed_dir();
```

**Step 3: Update candle_backend.rs**

Find line 30 and replace:

**Old:**
```rust
let dir = home_dir.join(".okaychat").join("candle");
```

**New:**
```rust
use apchat_common::ApChatPaths;

let dir = ApChatPaths::candle_dir();
```

**Step 4: Build to verify**

```bash
cd crates/apchat-skills
cargo build
```

**Step 5: Commit**

```bash
git add crates/apchat-skills
git commit -m "refactor: use ApChatPaths in apchat-skills embeddings"
```

---

### Task 10: Update `apchat-main/src/cli.rs`

**Files:**
- Modify: `apchat-main/src/cli.rs`
- Modify: `apchat-main/Cargo.toml`

**Step 1: Add dependency**

```toml
[dependencies]
apchat-common = { path = "../crates/apchat-common" }
```

**Step 2: Update cli.rs**

Find lines 162, 181 and replace:

**Old:**
```rust
#[arg(long, default_value = "~/.okaychat/sessions", env = "OKAYCHAT_SESSIONS_DIR")]
```

**New:**
```rust
#[arg(long, env = "APCHAT_SESSIONS_DIR")]
sessions_dir: Option<PathBuf>,
```

Then where the default is used:

**Old:**
```rust
let sessions_dir = cli.sessions_dir.unwrap_or_else(|| PathBuf::from("~/.okaychat/sessions"));
```

**New:**
```rust
let sessions_dir = cli.sessions_dir.unwrap_or_else(ApChatPaths::sessions_dir);
```

Also update env var documentation from `OKAYCHAT_SESSIONS_DIR` to `APCHAT_SESSIONS_DIR`.

**Step 3: Build to verify**

```bash
cd apchat-main
cargo build
```

**Step 4: Commit**

```bash
git add apchat-main
git commit -m "refactor: use ApChatPaths in apchat-main CLI"
```

---

### Task 11: Update `apchat-main/src/chat/history.rs`

**Files:**
- Modify: `apchat-main/src/chat/history.rs`

**Step 1: Update history.rs**

Find lines 307, 348 and replace:

**Old:**
```rust
let histories_dir = home_dir.join(".okaychat").join("histories");
```

**New:**
```rust
use apchat_common::ApChatPaths;

let histories_dir = ApChatPaths::histories_dir();
```

**Step 2: Build to verify**

```bash
cd apchat-main
cargo build
```

**Step 3: Commit**

```bash
git add apchat-main
git commit -m "refactor: use ApChatPaths in apchat-main chat history"
```

---

### Task 12: Update `apchat-main/src/app/subagent_tests.rs`

**Files:**
- Modify: `apchat-main/src/app/subagent_tests.rs`

**Step 1: Update subagent_tests.rs**

Find line 46 and replace:

**Old:**
```rust
let sessions_dir = PathBuf::from("~/.okaychat/sessions");
```

**New:**
```rust
use apchat_common::ApChatPaths;

let sessions_dir = ApChatPaths::sessions_dir();
```

**Step 2: Build to verify**

```bash
cd apchat-main
cargo build
```

**Step 3: Commit**

```bash
git add apchat-main
git commit -m "refactor: use ApChatPaths in apchat-main subagent tests"
```

---

### Task 13: Full workspace build and test

**Step 1: Build entire workspace**

```bash
cargo build --workspace --release
```

**Step 2: Run all tests**

```bash
cargo test --workspace
```

**Step 3: Verify no `.okaychat` references remain**

```bash
grep -r "\.okaychat" --include="*.rs" .
```

Expected: No results (or only in comments/docs)

**Step 4: Final commit**

```bash
git add .
git commit -m "refactor: complete migration to XDG-compliant ApChatPaths

- Replace all ~/.okaychat references with XDG directories
- Config: ~/.config/apchat (credentials.toml, env)
- Data: ~/.local/share/apchat (histories, sessions)
- Cache: ~/.cache/apchat (logs, fastembed, candle)
- Update environment variables: OKAYCHAT_* -> APCHAT_*
- Add apchat-common crate with ApChatPaths API"
```

---

## Verification Checklist

After all tasks complete:

- [ ] `cargo build --workspace --release` succeeds
- [ ] `cargo test --workspace` passes
- [ ] No `.okaychat` references in Rust code
- [ ] `ApChatPaths::init_all()` can be called at app startup
- [ ] Environment variables updated: `OKAYCHAT_SESSIONS_DIR` → `APCHAT_SESSIONS_DIR`

---

## Notes

- No legacy migration needed - fresh XDG directories
- All `~/.okaychat` → XDG `apchat` directories
- `dirs` crate handles platform differences (Linux/macOS/Windows)
- Logs treated as cache (transient, safe to clear)