# Content Length Limiter Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a content length limiter that saves large tool outputs to files and inserts context window notes to prevent sudden context window blow-up.

**Architecture:** 
- Add a `ContentLimiter` struct to track maximum content length and handle large outputs
- Modify `ToolResult` to include a `truncated` flag and `full_path` for saved content
- Create `.apchat-large-outputs` directory for storing large outputs
- Add a `max_content_length` configuration option (default: 20,000 characters)
- Update tool execution to automatically save and truncate large outputs
- Ensure the context window note guides the model to use tools to inspect saved output

**Tech Stack:** Rust, async/await, file I/O, serde for serialization

---

## Task 1: Add Content Length Configuration

**Files:**
- Create: `crates/apchat-toolcore/src/content_limiter.rs`
- Modify: `crates/apchat-toolcore/src/tool.rs:100-120`
- Modify: `crates/apchat-toolcore/src/tool_context.rs:50-70`

**Step 1: Create the ContentLimiter module**

```rust
use std::path::PathBuf;
use std::fs;
use std::io::Write;
use chrono::Local;
use uuid::Uuid;

/// Maximum content length before truncation (default: 20,000 characters)
pub const DEFAULT_MAX_CONTENT_LENGTH: usize = 20_000;

/// Content limiter configuration
#[derive(Debug, Clone)]
pub struct ContentLimiterConfig {
    pub max_content_length: usize,
    pub large_outputs_dir: PathBuf,
}

impl ContentLimiterConfig {
    pub fn new(work_dir: &PathBuf) -> Self {
        let large_outputs_dir = work_dir.join(".apchat-large-outputs");
        Self {
            max_content_length: DEFAULT_MAX_CONTENT_LENGTH,
            large_outputs_dir,
        }
    }

    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_content_length = max_length;
        self
    }
}

/// Content limiter for handling large tool outputs
#[derive(Debug, Clone)]
pub struct ContentLimiter {
    pub config: ContentLimiterConfig,
}

impl ContentLimiter {
    pub fn new(config: ContentLimiterConfig) -> Self {
        Self { config }
    }

    /// Check if content exceeds maximum length
    pub fn is_content_too_large(&self, content: &str) -> bool {
        content.len() > self.config.max_content_length
    }

    /// Save large content to file and return truncated version with note
    pub fn save_and_truncate(&self, content: String, tool_name: &str) -> (String, Option<String>, bool) {
        if !self.is_content_too_large(&content) {
            return (content, None, false);
        }

        // Create large outputs directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(&self.config.large_outputs_dir) {
            eprintln!("Warning: Failed to create large outputs directory: {}", e);
            return (content, None, false);
        }

        // Generate unique filename
        let timestamp = Local::now().format("%Y%m%d-%H%M%S");
        let filename = format!("{}-{}-{}.txt", tool_name, timestamp, Uuid::new_v4());
        let file_path = self.config.large_outputs_dir.join(&filename);

        // Write content to file
        if let Err(e) = fs::write(&file_path, &content) {
            eprintln!("Warning: Failed to write large output to file: {}", e);
            return (content, None, false);
        }

        // Create truncated content with note
        let truncated_content = format!("[LARGE OUTPUT TRUNCATED - Full output saved to: {}]", 
                                       file_path.display());
        
        // Add note about how to inspect the output
        let note = Some(format!("\n⚠️  Note: Output exceeds {} characters. Use `open_file` tool to inspect the full output at: {}",
                               self.config.max_content_length,
                               file_path.display()));

        (truncated_content, note, true)
    }
}
```

**Step 2: Add truncated field to ToolResult**

```rust
/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub content: String,
    pub error: Option<String>,
    pub truncated: bool,
    pub full_path: Option<String>,
}

impl ToolResult {
    pub fn success(content: String) -> Self {
        Self {
            success: true,
            content,
            error: None,
            truncated: false,
            full_path: None,
        }
    }

    pub fn success_with_truncation(content: String, full_path: String) -> Self {
        Self {
            success: true,
            content,
            error: None,
            truncated: true,
            full_path: Some(full_path),
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            content: String::new(),
            error: Some(error),
            truncated: false,
            full_path: None,
        }
    }
}
```

**Step 3: Add content limiter to ToolContext**

```rust
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub work_dir: PathBuf,
    pub session_id: String,
    pub environment: HashMap<String, String>,
    pub policy_manager: PolicyManager,
    pub terminal_manager: Option<Arc<Mutex<TerminalManager>>>,
    pub skill_registry: Option<Arc<SkillRegistry>>,
    pub todo_manager: Option<Arc<TodoManager>>,
    pub non_interactive: bool,
    pub current_model_string: Option<String>,
    pub content_limiter: Option<Arc<ContentLimiter>>, // NEW
}

impl ToolContext {
    pub fn with_content_limiter(mut self, content_limiter: Arc<ContentLimiter>) -> Self {
        self.content_limiter = Some(content_limiter);
        self
    }
    // ... existing methods ...
}
```

**Step 4: Run tests to verify changes compile**

Run: `cargo check --package apchat-toolcore -v`
Expected: Compilation succeeds

**Step 5: Commit**

```bash
git add crates/apchat-toolcore/src/content_limiter.rs crates/apchat-toolcore/src/tool.rs crates/apchat-toolcore/src/tool_context.rs
git commit -m "feat: add content limiter infrastructure and ToolResult truncation support"
```

---

## Task 2: Add Content Limiter to Tool Registry

**Files:**
- Modify: `crates/apchat-toolcore/src/tool_registry.rs:20-40`

**Step 1: Update ToolRegistry to support content limiter**

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    policy_manager: PolicyManager,
    content_limiter: Option<Arc<ContentLimiter>>, // NEW
}

impl ToolRegistry {
    pub fn new(policy_manager: PolicyManager) -> Self {
        Self {
            tools: HashMap::new(),
            policy_manager,
            content_limiter: None,
        }
    }

    pub fn with_content_limiter(mut self, content_limiter: Arc<ContentLimiter>) -> Self {
        self.content_limiter = Some(content_limiter);
        self
    }
    // ... existing methods ...
}
```

**Step 2: Update execute_tool to apply content limiting**

```rust
impl ToolRegistry {
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        params: ToolParameters,
        context: &ToolContext,
    ) -> ToolResult {
        let tool = self.get_tool(tool_name)
            .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found", tool_name));

        if let Err(e) = tool {
            return ToolResult::error(e.to_string());
        }

        let tool = tool.unwrap();
        
        // Execute the tool
        let mut result = tool.execute(params, context).await;
        
        // Apply content limiting if configured
        if result.success {
            if let Some(content_limiter) = &context.content_limiter {
                let (truncated_content, note, is_truncated) = content_limiter.save_and_truncate(
                    result.content.clone(),
                    tool_name
                );
                
                result.content = truncated_content;
                
                if is_truncated {
                    result.truncated = true;
                    result.full_path = note.map(|n| n.split("'""").collect::<Vec<_>>()[1].to_string());
                    
                    // Append note to content if note exists
                    if let Some(note_content) = note {
                        result.content.push_str(&note_content);
                    }
                }
            }
        }

        result
    }
}
```

**Step 3: Run tests to verify changes compile**

Run: `cargo check --package apchat-toolcore -v`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add crates/apchat-toolcore/src/tool_registry.rs
git commit -m "feat: integrate content limiter into tool registry"
```

---

## Task 3: Initialize Content Limiter in Main Application

**Files:**
- Modify: `apchat-main/src/main.rs:100-150`
- Modify: `apchat-main/src/app/setup.rs:50-80`

**Step 1: Update APChat struct to hold content limiter**

```rust
pub struct APChat {
    // ... existing fields ...
    content_limiter: Option<Arc<ContentLimiter>>, // NEW
}

impl APChat {
    pub fn with_content_limiter(mut self, content_limiter: Arc<ContentLimiter>) -> Self {
        self.content_limiter = Some(content_limiter);
        self
    }
    // ... existing methods ...
}
```

**Step 2: Initialize content limiter in setup**

```rust
pub async fn setup_chat(
    client_config: ClientConfig,
    work_dir: PathBuf,
    policy_manager: PolicyManager,
    stream: bool,
    verbose: bool,
    backend_type: TerminalBackendType,
    early_superpowers: bool,
) -> Result<APChat> {
    // ... existing setup code ...

    // Initialize content limiter
    let content_limiter_config = ContentLimiterConfig::new(&work_dir)
        .with_max_length(DEFAULT_MAX_CONTENT_LENGTH);
    let content_limiter = Arc::new(ContentLimiter::new(content_limiter_config));

    let mut chat = APChat::new_with_config(
        client_config,
        work_dir,
        policy_manager.clone(),
        stream,
        verbose,
        backend_type,
        early_superpowers,
    );

    chat = chat.with_content_limiter(content_limiter.clone());

    // Update tool registry with content limiter
    if let Some(tool_registry) = &mut chat.tool_registry {
        tool_registry.set_content_limiter(content_limiter);
    }

    Ok(chat)
}
```

**Step 3: Run tests to verify changes compile**

Run: `cargo check --package apchat-main -v`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add apchat-main/src/main.rs apchat-main/src/app/setup.rs
git commit -m "feat: initialize content limiter in main application"
```

---

## Task 4: Update Tool Registry Initialization

**Files:**
- Modify: `crates/apchat-toolcore/src/tool_registry.rs:150-200`

**Step 1: Add set_content_limiter method**

```rust
impl ToolRegistry {
    // ... existing methods ...

    pub fn set_content_limiter(&mut self, content_limiter: Arc<ContentLimiter>) {
        self.content_limiter = Some(content_limiter);
    }
    // ... existing methods ...
}
```

**Step 2: Update get_tools_for_agent to include content limiter**

```rust
impl ToolRegistry {
    pub fn get_tools_for_agent(&self, allowed_tools: &[String]) -> Vec<Arc<dyn Tool>> {
        let mut filtered_tools = Vec::new();
        
        for tool_name in allowed_tools {
            if let Some(tool) = self.tools.get(tool_name) {
                filtered_tools.push(tool.clone());
            }
        }
        
        filtered_tools
    }
    
    pub fn to_context(&self, work_dir: PathBuf, session_id: String, policy_manager: PolicyManager) -> ToolContext {
        let mut context = ToolContext::new(work_dir, session_id, policy_manager);
        
        if let Some(content_limiter) = &self.content_limiter {
            context = context.with_content_limiter(content_limiter.clone());
        }
        
        context
    }
}
```

**Step 3: Run tests to verify changes compile**

Run: `cargo check --package apchat-toolcore -v`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add crates/apchat-toolcore/src/tool_registry.rs
git commit -m "feat: update tool registry to propagate content limiter to context"
```

---

## Task 5: Update Agent Tool Execution

**Files:**
- Modify: `crates/apchat-agents/src/agent.rs:80-120`
- Modify: `crates/apchat-agents/src/agent_factory.rs:100-150`

**Step 1: Update agent tool context creation**

```rust
impl ConfigurableAgent {
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        params: ToolParameters,
        context: &ToolContext,
    ) -> ToolResult {
        let result = self.tool_registry.execute_tool(tool_name, params, context).await;
        
        // If content was truncated, log the original file path
        if result.truncated {
            if let Some(full_path) = &result.full_path {
                self.log_message(&format!(
                    "Content truncated for tool '{}'. Full output saved to: {}",
                    tool_name,
                    full_path
                ));
            }
        }
        
        result
    }
    // ... existing methods ...
}
```

**Step 2: Update agent factory to create tool context with content limiter**

```rust
impl AgentFactory {
    pub fn create_tool_context(&self, agent: &ConfigurableAgent, work_dir: PathBuf) -> ToolContext {
        let mut context = ToolContext::new(
            work_dir.clone(),
            agent.session_id.clone(),
            agent.policy_manager.clone(),
        );
        
        context = context.with_non_interactive(agent.config.non_interactive);
        
        // Propagate content limiter from tool registry to context
        if let Some(content_limiter) = &self.tool_registry.content_limiter {
            context = context.with_content_limiter(content_limiter.clone());
        }
        
        context
    }
    // ... existing methods ...
}
```

**Step 3: Run tests to verify changes compile**

Run: `cargo check --package apchat-agents -v`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add crates/apchat-agents/src/agent.rs crates/apchat-agents/src/agent_factory.rs
git commit -m "feat: update agent tool execution with content limiter support"
```

---

## Task 6: Add Content Limiter Configuration Option

**Files:**
- Modify: `apchat-main/src/cli.rs:50-80`
- Modify: `apchat-main/src/config/mod.rs:100-130`

**Step 1: Add CLI option for max content length**

```rust
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    // ... existing fields ...
    
    /// Maximum content length before truncation (default: 20000)
    #[arg(long = "max-content-length")]
    pub max_content_length: Option<usize>,
}
```

**Step 2: Update ClientConfig to support max content length**

```rust
#[derive(Debug, Clone)]
pub struct ClientConfig {
    // ... existing fields ...
    pub max_content_length: usize,
}

impl ClientConfig {
    pub fn new() -> Self {
        Self {
            // ... existing fields ...
            max_content_length: DEFAULT_MAX_CONTENT_LENGTH,
        }
    }
    
    pub fn from_cli(cli: &Cli) -> Self {
        let mut config = Self::new();
        // ... existing field assignments ...
        
        config.max_content_length = cli.max_content_length
            .unwrap_or(DEFAULT_MAX_CONTENT_LENGTH);
        
        config
    }
}
```

**Step 3: Update content limiter initialization to use config value**

```rust
// In setup.rs or main.rs
let content_limiter_config = ContentLimiterConfig::new(&work_dir)
    .with_max_length(client_config.max_content_length);
```

**Step 4: Run tests to verify changes compile**

Run: `cargo check --package apchat-main -v`
Expected: Compilation succeeds

**Step 5: Commit**

```bash
git add apchat-main/src/cli.rs apchat-main/src/config/mod.rs
git commit -m "feat: add CLI configuration option for max content length"
```

---

## Task 7: Update Tool Descriptions with Truncation Information

**Files:**
- Modify: `crates/apchat-tools/src/file_ops.rs:50-80`
- Modify: `crates/apchat-tools/src/search.rs:30-60`

**Step 1: Update file operations tool description**

```rust
impl FileOperationsTool {
    fn description(&self) -> &str {
        "Perform file operations: read, write, edit, delete, search, and batch edits.

IMPORTANT: Large file contents (>20,000 characters) will be automatically truncated and saved to the .apchat-large-outputs directory. Use the open_file tool to inspect the full content if needed."
    }
    // ... rest of implementation ...
}
```

**Step 2: Update search tool description**

```rust
impl SearchTool {
    fn description(&self) -> &str {
        "Search for text across files using glob patterns.

IMPORTANT: Large search results (>20,000 characters) will be automatically truncated and saved to the .apchat-large-outputs directory. Use the open_file tool to inspect the full results if needed."
    }
    // ... rest of implementation ...
}
```

**Step 3: Update other tools with similar descriptions**

Apply similar updates to:
- `SystemTool` in `crates/apchat-tools/src/system.rs`
- `ListFilesTool` in `crates/apchat-tools/src/file_ops.rs`
- `SearchTool` variations

**Step 4: Run tests to verify changes compile**

Run: `cargo check -v`
Expected: Compilation succeeds

**Step 5: Commit**

```bash
git add crates/apchat-tools/src/file_ops.rs crates/apchat-tools/src/search.rs crates/apchat-tools/src/system.rs
git commit -m "docs: update tool descriptions with truncation information"
```

---

## Task 8: Write Tests for Content Limiter

**Files:**
- Create: `crates/apchat-toolcore/tests/content_limiter_tests.rs`

**Step 1: Create comprehensive tests**

```rust
use std::fs;
use std::path::PathBuf;
use apchat_toolcore::{ContentLimiter, ContentLimiterConfig, ToolResult};

#[test]
fn test_content_limiter_creation() {
    let work_dir = PathBuf::from("/tmp/test-workdir");
    let config = ContentLimiterConfig::new(&work_dir)
        .with_max_length(100); // Small limit for testing
    let limiter = ContentLimiter::new(config);
    
    assert_eq!(limiter.config.max_content_length, 100);
    assert!(limiter.config.large_outputs_dir.ends_with(".apchat-large-outputs"));
}

#[test]
fn test_content_not_truncated_when_below_limit() {
    let work_dir = PathBuf::from("/tmp/test-workdir");
    let config = ContentLimiterConfig::new(&work_dir)
        .with_max_length(100);
    let limiter = ContentLimiter::new(config);
    
    let content = "Short content".to_string();
    let (result_content, note, is_truncated) = limiter.save_and_truncate(content, "test_tool");
    
    assert_eq!(result_content, "Short content");
    assert!(note.is_none());
    assert!(!is_truncated);
}

#[test]
fn test_content_truncated_when_above_limit() {
    let work_dir = PathBuf::from("/tmp/test-workdir");
    let config = ContentLimiterConfig::new(&work_dir)
        .with_max_length(100);
    let limiter = ContentLimiter::new(config);
    
    // Create content longer than 100 characters
    let content = "A".repeat(150);
    let (result_content, note, is_truncated) = limiter.save_and_truncate(content.clone(), "test_tool");
    
    assert!(result_content.contains("[LARGE OUTPUT TRUNCATED"));
    assert!(note.is_some());
    assert!(is_truncated);
    
    // Verify file was created
    let note_text = note.unwrap();
    assert!(note_text.contains("open_file"));
    assert!(note_text.contains(".apchat-large-outputs"));
    
    // Verify content was saved to file
    let full_path_match = note_text.split("open_file tool to inspect the full output at: ").collect::<Vec<_>>();
    if full_path_match.len() > 1 {
        let file_path = full_path_match[1].trim();
        assert!(PathBuf::from(file_path).exists());
        
        // Verify content matches
        let saved_content = fs::read_to_string(file_path).unwrap();
        assert_eq!(saved_content, content);
        
        // Clean up
        fs::remove_file(file_path).ok();
    }
}

#[test]
fn test_tool_result_truncation_fields() {
    let result = ToolResult::success("test".to_string());
    assert!(!result.truncated);
    assert!(result.full_path.is_none());
    
    let result = ToolResult::success_with_truncation("truncated".to_string(), "/path/to/file.txt".to_string());
    assert!(result.truncated);
    assert_eq!(result.full_path, Some("/path/to/file.txt".to_string()));
}
```

**Step 2: Run the tests**

Run: `cargo test --package apchat-toolcore content_limiter -v`
Expected: All tests pass

**Step 3: Commit**

```bash
git add crates/apchat-toolcore/tests/content_limiter_tests.rs
git commit -m "test: add comprehensive tests for content limiter"
```

---

## Task 9: Build and Verify Integration

**Step 1: Build the entire project**

Run: `cargo build --release -v`
Expected: Build completes successfully

**Step 2: Run existing tests to ensure no regressions**

Run: `cargo test -v`
Expected: All existing tests pass

**Step 3: Test the feature manually**

Create a test script that:
1. Creates a large file (>20,000 characters)
2. Uses the peek_file_top_10_lines tool to read it
3. Verifies the output is truncated with a note
4. Verifies the full content is saved to `.apchat-large-outputs/`
5. Verifies the note contains the correct file path

**Step 4: Commit final integration**

```bash
git add .
git commit -m "feat: complete content length limiter implementation"
```

---

## Task 10: Update Documentation

**Files:**
- Create: `docs/architecture/CONTENT_LENGTH_LIMITER.md`

**Step 1: Create documentation**

```markdown
# Content Length Limiter

## Overview

The content length limiter prevents sudden context window blow-up by automatically saving large tool outputs to files and inserting context window notes.

## Design

### Components

1. **ContentLimiter**: Core struct that handles content length checking and file saving
2. **ContentLimiterConfig**: Configuration with max content length and output directory
3. **ToolResult Extension**: Added `truncated` and `full_path` fields to track truncation
4. **ToolContext Integration**: Content limiter is propagated through the tool execution context

### Flow

```
Tool Execution → Check Content Length → 
  (If < limit) → Return content normally
  (If ≥ limit) → Save to file → 
  Return truncated content with note →
  Context window gets note → Model uses tools to inspect
```

### Configuration

- **Default limit**: 20,000 characters
- **CLI option**: `--max-content-length <value>`
- **Output directory**: `.apchat-large-outputs/` in the working directory
- **Filename format**: `<tool_name>-<timestamp>-<uuid>.txt`

### Truncation Message Format

```
[LARGE OUTPUT TRUNCATED - Full output saved to: /path/to/.apchat-large-outputs/tool-name-20250101-120000-unique-id.txt]

⚠️  Note: Output exceeds 20000 characters. Use `open_file` tool to inspect the full output at: /path/to/.apchat-large-outputs/tool-name-20250101-120000-unique-id.txt
```

## Usage

### For Users

When you see a truncated output:
1. Note the file path in the message
2. Use the `open_file` tool to read the full content
3. Example: `open_file --file_path ".apchat-large-outputs/tool-name-20250101-120000-unique-id.txt"`

### For Developers

To adjust the limit:
```bash
cargo run -- --max-content-length 30000
```

To disable truncation (not recommended):
```bash
cargo run -- --max-content-length 0
```

### Affected Tools

All tools that return string content are affected:
- File operations (read, list, search)
- System commands
- Search operations
- Terminal output
- Any tool returning large text content

## Benefits

1. **Context Window Protection**: Prevents sudden blow-up from large outputs
2. **Safe Inspection**: Model can use tools to inspect saved content
3. **Transparency**: Clear notes explain where full content is located
4. **Configurable**: Adjustable limit based on context window size
5. **Automatic**: No manual intervention required

## Implementation Details

### File Management

- Directory created automatically if it doesn't exist
- Files named with timestamp and UUID to prevent collisions
- Files are not automatically cleaned up (manual management)

### Performance

- Content length check is O(n) string length operation
- File writing only occurs for large outputs
- Minimal overhead for normal-sized outputs

### Error Handling

- If directory creation fails, content is returned normally
- If file writing fails, content is returned normally
- Truncation is a best-effort feature

## Testing

See `crates/apchat-toolcore/tests/content_limiter_tests.rs` for comprehensive test coverage.

## Future Enhancements

Possible improvements:
- Configurable cleanup policy for old large outputs
- Compression for very large outputs
- Metadata files tracking saved outputs
- Size-based limits instead of character-based
```

**Step 2: Commit documentation**

```bash
git add docs/architecture/CONTENT_LENGTH_LIMITER.md
git commit -m "docs: add content length limiter documentation"
```

---

Plan complete and saved to `docs/plans/2025-07-25-content-length-limiter.md`.

Two execution options:

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**
