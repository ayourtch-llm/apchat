# Persistent Memory Feature Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete the Persistent Memory feature implementation with command-line configuration support, ensuring all memory operations (store, query, update, delete, list) work correctly with SQLite backend.

**Architecture:** The memory system uses SQLite for persistent storage with a default location of `~/.okaychat/memory.sqlite`. It can be overridden via environment variable `APCHAT_MEMORY_DB_PATH` or a command-line flag. The system provides tools for CRUD operations on memories with proper filtering, pagination, and search capabilities.

**Tech Stack:** Rust, SQLx (SQLite), Serde, UUID, Chrono

---

## Task 0: Verify Current Implementation

**Files:**
- Check: `crates/apchat-tools/src/memory/` (all files)
- Check: `crates/apchat-tools/tests/` (memory tests)
- Check: `apchat-main/src/config/mod.rs` (tool registration)

**Step 1: Review existing memory module**
- Verify Memory struct definition
- Verify database operations (connect, init, CRUD)
- Verify tool implementations

**Step 2: Run existing tests**
```bash
cargo test -p apchat-tools memory_db_tests
cargo test -p apchat-tools query_memory_tool_test
cargo test -p apchat-tools store_memory_tool_test
cargo test -p apchat-tools update_memory_tool_test
cargo test -p apchat-tools delete_memory_tool_test
```

**Step 3: Verify tool registration**
- Confirm all memory tools are registered with "memory" category
- Check they appear in tool registry tests

**Step 4: Commit**
```bash
git add .
git commit -m "chore: verify existing memory implementation"
```

---

## Task 1: Add Command-Line Flag for Database Path

**Files:**
- Modify: `apchat-main/src/cli.rs`
- Modify: `apchat-main/src/app/setup.rs`
- Modify: `apchat-main/src/config/mod.rs`

**Step 1: Add CLI flag**
In `cli.rs`, add to the main CLI struct:
```rust
/// Path to memory database (overrides APCHAT_MEMORY_DB_PATH)
#[arg(long, env = "APCHAT_MEMORY_DB_PATH")]
pub memory_db_path: Option<PathBuf>,
```

**Step 2: Pass to configuration**
Update `AppConfig` to include:
```rust
pub memory_db_path: Option<PathBuf>,
```

**Step 3: Update setup**
In `app/setup.rs`, pass the memory_db_path to the tool registry initialization.

**Step 4: Update memory db.rs**
Modify `get_memory_db_path()` to accept an optional override parameter:
```rust
pub fn get_memory_db_path(override_path: Option<&PathBuf>) -> PathBuf {
    if let Some(custom_path) = override_path {
        return custom_path.clone();
    }
    // ... rest of existing logic
}
```

**Step 5: Update all tools**
Modify each memory tool to accept the override path from context or config.

**Step 6: Run tests**
```bash
cargo test -p apchat-tools memory
cargo test -p apchat-main cli
```

**Step 7: Commit**
```bash
git add .
git commit -m "feat: add command-line flag for memory database path"
```

---

## Task 2: Enhance QueryMemoryTool with Embedding Search (Optional)

**Files:**
- Modify: `crates/apchat-tools/src/memory/search.rs`
- Modify: `crates/apchat-tools/src/memory/tools.rs`

**Step 1: Check tmp/claude-memory for embedding implementation**
- Review the embedding code in tmp/claude-memory
- Assess feasibility of integrating into current system

**Step 2: Add embedding support (if feasible)**
- Add embedding table to SQLite schema
- Implement embedding generation function
- Add semantic search capability

**Step 3: Update QueryMemoryTool**
- Add parameter for semantic vs keyword search
- Implement hybrid search (keyword + semantic)

**Step 4: Update tests**
- Add tests for embedding functionality
- Test hybrid search

**Step 5: Commit**
```bash
git add .
git commit -m "feat(memory): add semantic search with embeddings"
```

---

## Task 3: Add Advanced Filtering to ListMemoriesTool

**Files:**
- Modify: `crates/apchat-tools/src/memory/db.rs` (list_memories function)
- Modify: `crates/apchat-tools/src/memory/tools.rs` (ListMemoriesTool)

**Step 1: Enhance list_memories function**
- Add timestamp range filters (after_timestamp, before_timestamp)
- Add metadata filtering
- Add sorting options

**Step 2: Update ListMemoriesTool parameters**
- Add `after_timestamp` parameter
- Add `before_timestamp` parameter
- Add `sort_by` parameter (timestamp, content, etc.)
- Add `sort_order` parameter (asc, desc)

**Step 3: Update tests**
```rust
#[tokio::test]
async fn test_list_memories_with_timestamps() {
    // Test timestamp filtering
}

#[tokio::test]
async fn test_list_memories_sorting() {
    // Test different sort orders
}
```

**Step 4: Run tests**
```bash
cargo test -p apchat-tools memory
```

**Step 5: Commit**
```bash
git add .
git commit -m "feat(memory): enhance list memories with advanced filtering"
```

---

## Task 4: Add Memory Statistics Tool

**Files:**
- Create: `crates/apchat-tools/src/memory/stats.rs`
- Modify: `crates/apchat-tools/src/memory/mod.rs`
- Modify: `crates/apchat-tools/src/memory/tools.rs`
- Modify: `apchat-main/src/config/mod.rs`

**Step 1: Create stats module**
```rust
pub async fn get_memory_stats(
    pool: &SqlitePool,
    user_id: Option<&str>,
    conversation_id: Option<&str>,
) -> Result<MemoryStats> {
    // Implement statistics gathering
}

#[derive(Debug, Serialize)]
pub struct MemoryStats {
    pub total_memories: i64,
    pub total_size: i64,
    pub oldest_timestamp: Option<i64>,
    pub newest_timestamp: Option<i64>,
    pub user_counts: Vec<UserCount>,
    pub conversation_counts: Vec<ConversationCount>,
}
```

**Step 2: Create GetMemoryStatsTool**
```rust
pub struct GetMemoryStatsTool;

#[async_trait]
impl Tool for GetMemoryStatsTool {
    // ... implementation
}
```

**Step 3: Export and register**
- Add to mod.rs exports
- Register in apchat-main/src/config/mod.rs

**Step 4: Add tests**
```rust
#[tokio::test]
async fn test_memory_stats_tool() {
    // Test statistics gathering
}
```

**Step 5: Commit**
```bash
git add .
git commit -m "feat(memory): add memory statistics tool"
```

---

## Task 5: Add Batch Operations Tools

**Files:**
- Modify: `crates/apchat-tools/src/memory/tools.rs`
- Modify: `apchat-main/src/config/mod.rs`

**Step 1: Create BatchStoreMemoryTool**
```rust
pub struct BatchStoreMemoryTool;

#[async_trait]
impl Tool for BatchStoreMemoryTool {
    // Accepts array of memory objects
    // Validates and stores in transaction
}
```

**Step 2: Create BatchDeleteMemoryTool**
```rust
pub struct BatchDeleteMemoryTool;

#[async_trait]
impl Tool for BatchDeleteMemoryTool {
    // Accepts array of memory IDs
    // Deletes in transaction
}
```

**Step 3: Update db.rs**
- Add batch insert function
- Add batch delete function

**Step 4: Add tests**
```rust
#[tokio::test]
async fn test_batch_store_memories() {
    // Test batch operations
}
```

**Step 5: Commit**
```bash
git add .
git commit -m "feat(memory): add batch operations tools"
```

---

## Task 6: Integration Testing

**Files:**
- Create: `crates/apchat-tools/tests/memory_integration_tests.rs`

**Step 1: Create integration tests**
```rust
#[tokio::test]
async fn test_memory_lifecycle() {
    // Create, read, update, delete
}

#[tokio::test]
async fn test_memory_concurrency() {
    // Test concurrent operations
}

#[tokio::test]
async fn test_memory_migration() {
    // Test schema migration
}
```

**Step 2: Test with different database paths**
```rust
#[tokio::test]
async fn test_custom_db_path() {
    // Test with temporary directory
}
```

**Step 3: Run all tests**
```bash
cargo test -p apchat-tools memory
cargo test -p apchat-main --features test-utils
```

**Step 4: Commit**
```bash
git add .
git commit -m "test(memory): add comprehensive integration tests"
```

---

## Task 7: Documentation

**Files:**
- Modify: `docs/high-level/memory.md`
- Create: `docs/usage/memory.md`

**Step 1: Update high-level documentation**
- Add architecture diagram
- Document data model
- Document API

**Step 2: Create usage documentation**
- Examples of each tool
- Best practices
- Performance considerations

**Step 3: Add CLI documentation**
- Document `--memory-db-path` flag
- Document environment variable

**Step 4: Commit**
```bash
git add docs/
git commit -m "docs(memory): add comprehensive documentation"
```

---

## Task 8: Final Verification

**Files:**
- All modified files

**Step 1: Run full test suite**
```bash
cargo test -p apchat-tools
cargo test -p apchat-main
```

**Step 2: Verify tool registry**
```bash
cargo test -p apchat-toolcore tool_registry
```

**Step 3: Build release**
```bash
cargo build --release
```

**Step 4: Test with real data**
```bash
# Test storing and querying memories
# Verify database path override works
```

**Step 5: Clean up**
- Remove any debug code
- Ensure all tests pass

**Step 6: Commit**
```bash
git add .
git commit -m "chore(memory): final verification and cleanup"
```

---

## Implementation Notes

**Priorities:**
1. Task 0 - Verify current state (MUST PASS)
2. Task 1 - Add CLI flag (HIGH PRIORITY)
3. Task 2 - Embedding search (LOW PRIORITY - optional)
4. Tasks 3-5 - Enhancements (MEDIUM PRIORITY)
5. Task 6 - Integration testing (HIGH PRIORITY)
6. Task 7 - Documentation (MEDIUM PRIORITY)
7. Task 8 - Final verification (MUST PASS)

**Testing Strategy:**
- Unit tests for each function
- Integration tests for tool workflows
- End-to-end tests with real database
- Test with different database paths
- Test concurrent operations

**Performance Considerations:**
- Use transactions for batch operations
- Ensure proper indexing
- Limit result sets by default
- Implement efficient search queries

**Error Handling:**
- Validate all inputs
- Handle database errors gracefully
- Provide meaningful error messages
- Implement retry logic for transient failures
