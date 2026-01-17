# Persistent Memory Feature Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a persistent memory system for APChat that stores, retrieves, and searches memories similar to the skill search functionality, with SQLite backend and configurable storage location.

**Architecture:** The persistent memory system will be implemented as a set of new tools integrated into the APChat tool registry. It will use SQLite for storage with a schema that supports memories with content, timestamps, and optional metadata. The system will include CRUD operations and search functionality with similarity scoring, reusing relevant components from the tmp/claude-memory project.

**Tech Stack:** Rust, SQLite (sqlx), chrono, uuid, serde

---

## 1. Feature Requirements Overview

### Core Requirements
1. **Memory Storage**: Store memories with content, timestamps, user/conversation IDs
2. **Memory Retrieval**: Fetch memories by ID, user, conversation, or time range
3. **Memory Search**: Full-text and semantic search capabilities
4. **Memory Management**: Update and delete existing memories
5. **Configuration**: Default storage at `~/.okaychat/memory.sqlite` with override support
6. **Integration**: Seamless integration as tools in the APChat tool registry

### Tools to Implement
1. `store_memory` - Store a new memory
2. `query_memory` - Search/retrieve memories
3. `update_memory` - Modify existing memory
4. `delete_memory` - Remove memory
5. `list_memories` - List all memories with filtering

## 2. Reusable Components from tmp/claude-memory

### Database Schema (Adapted)
```sql
CREATE TABLE memories (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    conversation_id TEXT NOT NULL,
    content TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    metadata TEXT
);
CREATE INDEX idx_memories_user ON memories(user_id);
CREATE INDEX idx_memories_conversation ON memories(conversation_id);
CREATE INDEX idx_memories_timestamp ON memories(timestamp);
```

### Key Components to Reuse
1. **Memory Model Struct**: `Memory` with id, user_id, conversation_id, content, timestamp, metadata
2. **Database Connection Management**: Connection pooling with sqlx
3. **Query Building**: Parameterized queries for search and filtering
4. **Error Handling**: Robust error handling with anyhow
5. **Configuration**: Storage path resolution using `dirs` crate

### Components to NOT Reuse
1. **Embedding/Gemma Model**: Too heavy for core APChat, will use simpler full-text search initially
2. **MCP Server**: Not needed, integrating directly as tools
3. **REST API**: Not needed, tools execute directly
4. **Sync Functionality**: Not required for initial implementation

## 3. Integration Points with APChat Architecture

### Tool Registry Integration
- **File**: `crates/apchat-tools/src/lib.rs`
- **Location**: Add new module `pub mod memory;`
- **Registration**: Add to `initialize_tool_registry()` in `apchat-main/src/config/mod.rs`

### Tool Structure
```rust
pub struct StoreMemoryTool;
pub struct QueryMemoryTool;
pub struct UpdateMemoryTool;
pub struct DeleteMemoryTool;
pub struct ListMemoriesTool;
```

### Database Layer
- **Location**: `crates/apchat-tools/src/memory/mod.rs`
- **Submodules**:
  - `memory.rs` - Memory model and core functions
  - `db.rs` - Database connection and operations
  - `search.rs` - Search functionality

### Configuration
- **Environment Variable**: `APCHAT_MEMORY_DB_PATH` to override default
- **Default Path**: `~/.okaychat/memory.sqlite`

## 4. Detailed Task Breakdown with Dependencies

### Phase 1: Database and Core Components

#### Task 1.1: Create Database Module Structure
**Files:**
- Create: `crates/apchat-tools/src/memory/mod.rs`
- Create: `crates/apchat-tools/src/memory/memory.rs`
- Create: `crates/apchat-tools/src/memory/db.rs`
- Create: `crates/apchat-tools/src/memory/search.rs`

**Steps:**
1. Write module structure with pub use statements
2. Create empty module files
3. Add memory module to `crates/apchat-tools/src/lib.rs`

**Verification:**
- `cargo check` compiles successfully
- Module can be imported in tests

#### Task 1.2: Implement Memory Model
**Files:**
- Modify: `crates/apchat-tools/src/memory/memory.rs`

**Steps:**
1. Define `Memory` struct with Serialize/Deserialize
2. Implement helper methods (new, from_db_row, to_db_row)
3. Add validation methods

**Verification:**
- `cargo test` passes for memory model tests

#### Task 1.3: Implement Database Connection
**Files:**
- Modify: `crates/apchat-tools/src/memory/db.rs`

**Steps:**
1. Create `get_db_path()` function with config override support
2. Implement `connect_pool()` function for connection pooling
3. Add `init_db()` to create tables if not exist
4. Implement migration handling

**Verification:**
- Database creates successfully
- Connection pool works
- Tables created correctly

#### Task 1.4: Implement CRUD Operations
**Files:**
- Modify: `crates/apchat-tools/src/memory/db.rs`

**Steps:**
1. `create_memory()` - Insert new memory
2. `get_memory()` - Fetch by ID
3. `get_memories_by_user()` - Fetch by user_id
4. `get_memories_by_conversation()` - Fetch by conversation_id
5. `get_memories_by_time_range()` - Fetch by timestamp range
6. `update_memory()` - Update existing memory
7. `delete_memory()` - Delete memory
8. `list_memories()` - List with filtering and pagination

**Verification:**
- All CRUD operations tested with mock data

### Phase 2: Tool Implementations

#### Task 2.1: Implement StoreMemoryTool
**Files:**
- Create: `crates/apchat-tools/src/memory.rs` (tool implementations)

**Steps:**
1. Create `StoreMemoryTool` struct
2. Implement `Tool` trait:
   - `name()` - "store_memory"
   - `description()` - Clear description
   - `parameters()` - user_id, conversation_id, content, metadata (optional)
   - `execute()` - Validate, create memory, store in DB
3. Add error handling

**Verification:**
- Tool can be registered
- Parameters validated correctly
- Memory stored successfully

#### Task 2.2: Implement QueryMemoryTool
**Files:**
- Modify: `crates/apchat-tools/src/memory.rs`

**Steps:**
1. Create `QueryMemoryTool` struct
2. Implement `Tool` trait:
   - `name()` - "query_memory"
   - `description()` - Search description
   - `parameters()` - user_id, query (optional), limit (optional), conversation_id (optional), after_timestamp (optional), before_timestamp (optional)
   - `execute()` - Search memories using full-text search
3. Add result formatting

**Verification:**
- Search returns correct results
- Filtering works (user, conversation, time)
- Pagination works

#### Task 2.3: Implement UpdateMemoryTool
**Files:**
- Modify: `crates/apchat-tools/src/memory.rs`

**Steps:**
1. Create `UpdateMemoryTool` struct
2. Implement `Tool` trait:
   - `name()` - "update_memory"
   - `description()` - Update description
   - `parameters()` - memory_id, content (optional), metadata (optional)
   - `execute()` - Update memory in DB
3. Add validation (user can only update their own memories)

**Verification:**
- Memory updated successfully
- Partial updates work
- Validation prevents unauthorized updates

#### Task 2.4: Implement DeleteMemoryTool
**Files:**
- Modify: `crates/apchat-tools/src/memory.rs`

**Steps:**
1. Create `DeleteMemoryTool` struct
2. Implement `Tool` trait:
   - `name()` - "delete_memory"
   - `description()` - Delete description
   - `parameters()` - memory_id
   - `execute()` - Delete memory from DB
3. Add confirmation prompt in interactive mode

**Verification:**
- Memory deleted successfully
- Confirmation works in interactive mode
- Auto-confirms in non-interactive mode

#### Task 2.5: Implement ListMemoriesTool
**Files:**
- Modify: `crates/apchat-tools/src/memory.rs`

**Steps:**
1. Create `ListMemoriesTool` struct
2. Implement `Tool` trait:
   - `name()` - "list_memories"
   - `description()` - List description
   - `parameters()` - user_id (optional), conversation_id (optional), limit (optional), offset (optional)
   - `execute()` - List memories with formatting
3. Add human-readable output formatting

**Verification:**
- Memories listed correctly
- Filtering works
- Pagination works

### Phase 3: Integration and Registration

#### Task 3.1: Register Tools in Tool Registry
**Files:**
- Modify: `apchat-main/src/config/mod.rs`

**Steps:**
1. Import memory tools
2. Add to `initialize_tool_registry()`:
   ```rust
   registry.register_with_categories(StoreMemoryTool, vec!["memory".to_string()]);
   registry.register_with_categories(QueryMemoryTool, vec!["memory".to_string()]);
   registry.register_with_categories(UpdateMemoryTool, vec!["memory".to_string()]);
   registry.register_with_categories(DeleteMemoryTool, vec!["memory".to_string()]);
   registry.register_with_categories(ListMemoriesTool, vec!["memory".to_string()]);
   ```

**Verification:**
- Tools appear in tool list
- Tools can be called by name

#### Task 3.2: Add sqlx to Cargo.toml
**Files:**
- Modify: `crates/apchat-tools/Cargo.toml`

**Steps:**
1. Add sqlx dependency:
   ```toml
   sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "chrono", "uuid"] }
   ```
2. Add uuid dependency:
   ```toml
   uuid = { version = "1.0", features = ["v4", "serde"] }
   ```
3. Add chrono dependency:
   ```toml
   chrono = { version = "0.4", features = ["serde"] }
   ```
4. Add dirs dependency:
   ```toml
   dirs = "5.0"
   ```

**Verification:**
- `cargo check` compiles successfully

### Phase 4: Testing

#### Task 4.1: Unit Tests for Database Layer
**Files:**
- Create: `crates/apchat-tools/tests/memory_db_tests.rs`

**Steps:**
1. Create temporary in-memory SQLite database
2. Test all CRUD operations
3. Test query building
4. Test error handling

**Verification:**
- All unit tests pass

#### Task 4.2: Integration Tests for Tools
**Files:**
- Create: `crates/apchat-tools/tests/memory_tools_integration.rs`

**Steps:**
1. Test tool registration
2. Test store_memory workflow
3. Test query_memory workflow
4. Test update_memory workflow
5. Test delete_memory workflow
6. Test list_memories workflow

**Verification:**
- All integration tests pass

#### Task 4.3: End-to-End Tests
**Files:**
- Create: `test_memory_feature.sh`

**Steps:**
1. Build project
2. Run sequence of tool calls to simulate real usage
3. Verify data persistence across runs
4. Test configuration override

**Verification:**
- Feature works end-to-end
- Data persists correctly
- Configuration override works

### Phase 5: Documentation

#### Task 5.1: Create Tool Documentation
**Files:**
- Create: `docs/tools/memory.md`

**Steps:**
1. Document all memory tools
2. Provide usage examples
3. Document parameter details
4. Document return formats

**Verification:**
- Documentation is clear and complete

#### Task 5.2: Update README
**Files:**
- Modify: `README.md`

**Steps:**
1. Add memory tools to feature list
2. Add memory section to tool documentation
3. Add configuration section for memory path

**Verification:**
- Documentation is consistent

## 5. Database Schema Design

### Main Table: memories

```sql
CREATE TABLE memories (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    conversation_id TEXT NOT NULL,
    content TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    metadata TEXT
);
```

**Fields:**
- `id`: UUID string (primary key)
- `user_id`: String identifier for user
- `conversation_id`: String identifier for conversation
- `content`: Text content of memory
- `timestamp`: Unix timestamp (chrono::DateTime)
- `metadata`: Optional JSON metadata (serde_json::Value)

**Indexes:**
```sql
CREATE INDEX idx_memories_user ON memories(user_id);
CREATE INDEX idx_memories_conversation ON memories(conversation_id);
CREATE INDEX idx_memories_timestamp ON memories(timestamp);
```

### Rationale
1. **UUID IDs**: Ensure uniqueness across devices and installations
2. **Text Fields**: Allow flexible user/conversation identification
3. **Timestamp**: Enable time-based queries and sorting
4. **Metadata**: Support extensibility for future features
5. **Indexes**: Optimize common query patterns

## 6. Memory CRUD Operations

### Create (Store)
```rust
async fn create_memory(
    pool: &SqlitePool,
    user_id: &str,
    conversation_id: &str,
    content: &str,
    metadata: Option<Value>,
) -> Result<Memory> {
    // Validate inputs
    // Insert into database
    // Return created memory
}
```

### Read (Query)
```rust
async fn get_memory(pool: &SqlitePool, memory_id: &str) -> Result<Option<Memory>>;

async fn query_memories(
    pool: &SqlitePool,
    user_id: Option<&str>,
    conversation_id: Option<&str>,
    query: Option<&str>,
    limit: Option<usize>,
    after_timestamp: Option<i64>,
    before_timestamp: Option<i64>,
) -> Result<Vec<Memory>>;
```

### Update
```rust
async fn update_memory(
    pool: &SqlitePool,
    memory_id: &str,
    content: Option<&str>,
    metadata: Option<Value>,
) -> Result<Memory>;
```

### Delete
```rust
async fn delete_memory(pool: &SqlitePool, memory_id: &str) -> Result<bool>;
```

## 7. Search Functionality

### Full-Text Search
Implement SQLite FTS5 extension for full-text search:

```sql
-- Alternative: Use LIKE for simple search initially
SELECT * FROM memories 
WHERE content LIKE '%search%'
  AND user_id = ?
  AND timestamp > ?
ORDER BY timestamp DESC
LIMIT ?;
```

### Search Features
1. **Keyword Search**: Simple pattern matching in content
2. **Filtering**: By user_id, conversation_id, timestamp ranges
3. **Pagination**: Limit and offset support
4. **Sorting**: By timestamp (newest first by default)

### Future Enhancements
- Add FTS5 virtual table for better full-text search
- Add semantic search using embeddings (separate optional feature)
- Add relevance scoring

## 8. Configuration

### Default Configuration
- **Path**: `~/.okaychat/memory.sqlite`
- **Automatic Creation**: Database created on first use
- **Migrations**: Handled automatically

### Override Support

#### Environment Variable
```bash
export APCHAT_MEMORY_DB_PATH=/custom/path/memory.sqlite
```

#### Command Line
```bash
apchat --memory-db-path /custom/path/memory.sqlite
```

#### Implementation
```rust
fn get_memory_db_path() -> PathBuf {
    if let Ok(custom_path) = std::env::var("APCHAT_MEMORY_DB_PATH") {
        return PathBuf::from(custom_path);
    }
    
    let mut path = if let Some(mut base) = dirs::home_dir() {
        base.push(".okaychat");
        base.push("memory.sqlite");
        base
    } else {
        // Fallback to current directory
        PathBuf::from("memory.sqlite")
    };
    
    // Ensure directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    
    path
}
```

## 9. Testing Strategy

### Test Levels

#### 1. Unit Tests
- **Location**: `crates/apchat-tools/tests/memory_db_tests.rs`
- **Coverage**:
  - Memory model serialization/deserialization
  - Database connection and initialization
  - Individual CRUD operations
  - Query building
  - Error handling

#### 2. Integration Tests
- **Location**: `crates/apchat-tools/tests/memory_tools_integration.rs`
- **Coverage**:
  - Tool registration and discovery
  - End-to-end tool execution workflows
  - Tool parameter validation
  - Tool result formatting
  - Policy integration

#### 3. End-to-End Tests
- **Location**: `test_memory_feature.sh`
- **Coverage**:
  - Feature workflows (store → query → update → delete)
  - Data persistence across sessions
  - Configuration override scenarios
  - Integration with existing tools

#### 4. Manual Testing
- **Scenarios**:
  - Store multiple memories
  - Search and filter memories
  - Update and delete memories
  - Test with different configuration paths
  - Test in interactive and non-interactive modes

### Test Data
Use consistent test data across all tests:
- User ID: `test-user-123`
- Conversation IDs: `conv-1`, `conv-2`
- Sample memories with varied content

### Continuous Integration
- Add tests to CI pipeline
- Check for database migrations compatibility
- Verify no breaking changes to existing tools

## 10. Dependencies and Compatibility

### New Dependencies
```toml
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "chrono", "uuid"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
dirs = "5.0"
```

### Existing Dependencies Used
- `anyhow` - Error handling
- `serde`/`serde_json` - Serialization
- `tokio` - Async runtime
- `apchat_toolcore` - Tool framework
- `apchat_policy` - Policy checks

### Breaking Changes
- **None**: Feature is additive only
- **Backward Compatible**: All existing tools remain unchanged

## 11. Implementation Timeline

### Estimated Duration
- **Total**: 2-3 weeks
- **Phase 1**: 3-4 days (Database and core)
- **Phase 2**: 5-7 days (Tool implementations)
- **Phase 3**: 1 day (Integration)
- **Phase 4**: 3-4 days (Testing)
- **Phase 5**: 1 day (Documentation)

### Risk Assessment

#### High Risk
1. **Database Migrations**: Need careful handling
2. **Performance**: Large memory collections may need optimization
3. **Data Corruption**: Need robust error handling

#### Mitigation Strategies
1. **Migrations**: Versioned migration system
2. **Performance**: Add indexes, implement pagination
3. **Data Integrity**: Transactions for all write operations

### Rollback Plan
- Database is separate from main code
- Can disable tools by not registering them
- Data can be backed up before upgrades

---

## Summary

This implementation plan provides a complete roadmap for implementing the Persistent Memory feature in APChat. The feature will be integrated as tools in the existing tool registry, using SQLite for persistent storage. The implementation leverages reusable components from the tmp/claude-memory project while adapting them to APChat's architecture and requirements.

The plan follows a phased approach with clear dependencies, comprehensive testing at multiple levels, and thorough documentation. The feature is designed to be backward compatible and can be incrementally enhanced in the future (e.g., adding semantic search).
