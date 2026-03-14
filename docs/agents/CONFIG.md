# Configuration & CLI Reference

## Code Style and Conventions

### Rust Code Style
- Use standard `cargo fmt` formatting
- Clippy lints: `cargo clippy`
- No comments unless explicitly requested
- Async/await throughout the codebase (tokio runtime)
- Proper error handling with `Result<T, E>` types
- Trait-based abstractions for extensibility

### Error Handling
- Use `anyhow::Error` for application errors
- Use `thiserror` for library errors with proper error variants
- Provide context with `.context()` or `.with_context()` from anyhow

### Testing Patterns
- Unit tests alongside implementation in `src/`
- Integration tests in `tests/` directories
- E2E tests via shell scripts in project root
- Test utilities: `tempfile`, `mockall`, `wiremock`, `pretty_assertions`
- Use `#[tokio::test]` for async tests

### Dependencies Management
- Workspace dependencies managed in root `Cargo.toml` `[workspace.dependencies]`
- Always check if dependencies already exist before adding new ones
- Prefer using workspace versions: `tokio = { workspace = true }`

## Architecture Patterns

### Tool System

**Central Registry Pattern** (`apchat-toolcore/src/tool_registry.rs`):
- All tools implement the `Tool` trait
- Registered at startup in a central registry
- Filtered by agent permissions in multi-agent mode
- Categories: file_ops, search, system, model_management, agent_control, skills

**Tool Execution Flow**:
1. LLM requests tool call (JSON format with tool name and arguments)
2. Registry looks up tool by name
3. Tool validates arguments against schema
4. Policy system checks permissions (Allow/Deny/Ask)
5. Tool executes with access to `ToolContext`
6. Results returned to LLM for next iteration

### Terminal/PTY Architecture

**Background Reader Pattern** (`apchat-terminal/src/session.rs`):
- Each PTY session spawns a background thread that continuously reads output
- Thread updates a shared screen buffer (protected by `Arc<Mutex<ScreenBuffer>>`)
- Tools read from the buffer without manual polling
- Graceful shutdown prevents thread hangs

**VT100 Emulation** (`apchat-terminal/src/screen_buffer.rs`):
- Interprets ANSI escape sequences
- Maintains cursor position, scrollback buffer
- Tools can capture screen content, cursor position, scrollback

## Configuration and Environment

### Required Environment Variables

```bash
# For Groq models
export GROQ_API_KEY=your_groq_api_key

# For Anthropic Claude (supports multiple model slots)
export ANTHROPIC_AUTH_TOKEN_BLU=your_claude_key_for_blu_model
export ANTHROPIC_AUTH_TOKEN_GRN=your_claude_key_for_grn_model
export ANTHROPIC_AUTH_TOKEN_RED=your_claude_key_for_red_model

# For OpenAI
export OPENAI_API_KEY=your_openai_api_key
```

### Model Configuration

Three model slots available:
- **BluModel** - Primary model for most agents
- **GrnModel** - Model for planning and code analysis
- **RedModel** - Additional model slot

## CLI Options

### Model Configuration
```bash
--api-url-blu-model <URL>        # Custom API URL for blu_model
--api-url-grn-model <URL>        # Custom API URL for grn_model
--api-url-red-model <URL>        # Custom API URL for red_model
--model-blu-model <NAME>          # Override model name for blu_model
--model-grn-model <NAME>          # Override model name for grn_model
--model-red-model <NAME>          # Override model name for red_model
--model <NAME>                    # Base model for all models
--llama-cpp-url <URL>             # Quick llama.cpp setup for all models
--blu-backend <BACKEND>           # Backend for blu_model (groq, anthropic, llama, openai)
--grn-backend <BACKEND>           # Backend for grn_model
--red-backend <BACKEND>           # Backend for red_model
--blu-key <KEY>                   # API key for blu_model
--grn-key <KEY>                   # API key for grn_model
--red-key <KEY>                   # API key for red_model
```

### Mode Selection
```bash
--agents                          # Enable multi-agent system
--task "Your task"                # One-shot task mode
--web                             # Web server mode
--interactive / -i                # Force interactive REPL
```

### Behavior
```bash
--stream                          # Streaming responses (default)
--auto-confirm                    # Skip confirmations
--early-superpowers               # Load all skills at startup
--policy-file <PATH>              # Custom policy file
--learn-policies                  # Learn from user decisions
```

### Debug
```bash
--verbose / -v                    # Verbose output
--debug <LEVEL>                   # Debug level 0-5
--pretty                          # Pretty-print JSON
```

### Web Server
```bash
--web                             # Enable web server
--web-port <PORT>                 # Web server port (default: 8080)
--web-bind <ADDRESS>              # Web server bind address (default: 127.0.0.1)
--web-attachable                  # Allow TUI session attachment from web
--sessions-dir <PATH>             # Directory for persistent web session storage
```

### Memory & Logging
```bash
--memory-db-path <PATH>           # Path to SQLite memory database
--sql-log-path <PATH>             # Path to SQL log database for debugging
```

### Terminal & Idle
```bash
--terminal-backend <BACKEND>      # Terminal backend (pty, tmux)
--idle-timeout <SECONDS>          # Idle timeout (1-86400)
--idle-input <TEXT>               # String to inject on idle timeout
```

### Delayed Instructions
```bash
--delayed-instructions            # Enable scheduled instructions feature
```

### Webex Bot
```bash
--webex-bot <USER_EMAIL>          # Enable Webex bot for user email
--webex-websocket                 # Use WebSocket for Webex bot
--webex-reconnect-hours <HOURS>   # Proactive reconnection interval (default: 24)
```

## Adding New Components

### Adding a New Tool

1. Create tool in appropriate crate (`apchat-tools/src/your_tool.rs`)
2. Implement the `Tool` trait:
   ```rust
   #[async_trait]
   impl Tool for YourTool {
       fn name(&self) -> &str { "your_tool" }
       fn description(&self) -> &str { "Does something useful" }
       fn parameters(&self) -> serde_json::Value { /* JSON schema */ }
       async fn execute(&self, args: &Value, context: &ToolContext) -> Result<ToolResult> {
           // Implementation
       }
   }
   ```
3. Register in `apchat-main/src/main.rs` tool registry initialization
4. Add to relevant agent configs in `agents/configs/*.json`

See `docs/dev/how_to_new_tool.md` for detailed guide.

### Adding a New Agent

1. Create JSON config in `agents/configs/your_agent.json`
2. Define name, description, model, tools, and system_prompt
3. Agent is automatically discovered and available to planner
4. No code changes required

### Adding a New Skill

1. Create directory `skills/your-skill/`
2. Create `SKILL.md` with YAML frontmatter:
   ```markdown
   ---
   name: your-skill
   description: Brief description
   ---

   # Detailed workflow instructions
   ```
3. Skill is automatically discovered at startup
4. Optionally create slash command in `.claude/commands/` for user access

## Git Operations

### Rebasing and Cherry-Picking

When using `git rebase --continue` or `git cherry-pick --continue`, set `GIT_EDITOR=true` to avoid interactive editor prompts:

```bash
GIT_EDITOR=true git rebase --continue
GIT_EDITOR=true git cherry-pick --continue
```

This accepts the default commit message without opening vim/nano.

### Pushing After Rebase

After rebasing, always try a normal push first:

```bash
git push origin main
```

If it fails with "non-fast-forward", **ask the user** before using `--force-with-lease`. Force pushing rewrites history and should only be done with explicit confirmation.

### Commit Best Practices

- **Double-check directories** before committing - verify `git status` shows only intended files
- **Group logically related files** together in single commits for cleaner history
- **Avoid committing large directories/files** without explicit user confirmation
- **Check file sizes** with `git add --dry-run` before staging
- Use environment variables for sensitive configuration

## Linting and Type Checking

```bash
# Format code
cargo fmt

# Run Clippy linter
cargo clippy

# Fix Clippy warnings automatically
cargo clippy --fix

# Check for unused dependencies
cargo +nightly udeps
```

## Documentation

**For comprehensive architecture details**: See `docs/project/CLAUDE.md`
**Tool documentation**: See `docs/tools/` directory
**Development guides**: See `docs/dev/` directory
**Architecture decisions**: See `docs/architecture/` directory

Key documentation files:
- `docs/architecture/TERMINAL_SESSIONS_DESIGN.md` - PTY architecture
- `docs/architecture/REFACTORING_SUMMARY.md` - Main.rs refactoring
- `docs/dev/how_to_new_tool.md` - Tool addition guide
- `docs/dev/CUSTOMIZING_AGENTS_AND_SKILLS.md` - Agent/skill configuration