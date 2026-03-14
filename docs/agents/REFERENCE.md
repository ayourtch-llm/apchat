# Reference Guide

## Workspace Structure

```
apchat/
├── apchat-main/           # Main binary crate (CLI, REPL, web server)
├── crates/
│   ├── apchat-common/     # Common utilities and types
│   ├── apchat-core/       # Core functionality
│   ├── apchat-designer/   # Design-related tools
│   ├── apchat-finserv/    # Financial services tools
│   ├── apchat-llm-api/    # Unified LLM client interface
│   ├── apchat-logging/    # Conversation logging (JSONL)
│   ├── apchat-mcp-pty-server/  # MCP PTY server integration
│   ├── apchat-models/     # Data structures and types
│   ├── apchat-mspc/       # Message passing channels
│   ├── apchat-ocr/        # Optical character recognition
│   ├── apchat-policy/     # Security and approval system
│   ├── apchat-pptx/       # PowerPoint processing
│   ├── apchat-progress/   # Progress tracking for streaming
│   ├── apchat-skills/     # Skill registry and loading
│   ├── apchat-terminal/   # PTY session management with VT100
│   ├── apchat-todo/       # Task tracking
│   ├── apchat-toolcore/   # Tool execution framework
│   ├── apchat-tools/      # 50+ tool implementations
│   ├── apchat-types/      # Type definitions
│   ├── apchat-vty/        # VTY/readline abstractions
│   ├── apchat-wasm/       # WebAssembly frontend
│   └── apchat-webex/      # Webex bot integration
├── agents/configs/        # Agent JSON configurations (8 agents)
├── skills/                # Skill definitions (38 skills)
├── hooks/                 # Session lifecycle hooks
└── docs/                  # Extensive documentation
```

## Complete Skills List (38 Total)

### Core Workflows
- `test-driven-development` - Red-Green-Refactor workflow
- `systematic-debugging` - 4-phase debugging process
- `writing-plans` - Implementation planning
- `executing-plans` - Plan execution with checkpoints
- `verification-before-completion` - Pre-delivery verification
- `requesting-code-review` / `receiving-code-review` - Code review workflow
- `brainstorming` - Collaborative idea development
- `clarify-before-coding` - Requirements clarification
- `condition-based-waiting` - Race condition handling
- `context-management` - Context window optimization
- `defense-in-depth` - Multi-layer validation
- `dispatching-parallel-agents` - Parallel investigation
- `finishing-a-development-branch` - Integration decisions
- `forecasting-reverso` - Reversal forecasting
- `refactoring-for-clarity` - Progressive decomposition
- `root-cause-tracing` - Error source identification
- `sharing-skills` - Skill contribution workflow
- `subagent-driven-development` - Independent task execution
- `testing-anti-patterns` - Test quality prevention
- `testing-skills-with-subagents` - Skill testing
- `using-git-worktrees` - Git isolation
- `using-superpowers` - Skill discovery and usage

### Advanced Skills
- `coding-conventions` - Code style guidelines
- `convening-experts` - Multi-expert collaboration
- `crafting-instructions` - Prompt engineering
- `learning-opportunities` - Feedback integration
- `reverse-socratic-examination` - Critical analysis
- `reviewing-ai-papers` - Academic paper analysis
- `skill-creator` - New skill development
- `socratic` - Socratic questioning
- `specification` - Specification writing
- `tiling-tree` - Hierarchical organization
- `writing-clearly-and-concisely` - Clear communication
- `writing-skills` - Skill creation workflow
- `commands` - Command management

## Agent Configuration Examples

### Planner Agent
```json
{
  "name": "planner",
  "description": "Strategic task planning and decomposition",
  "model": "grn_model",
  "tools": [],
  "system_prompt": "You are a strategic planner. Break down complex tasks into specific, actionable subtasks for specialized agents. Be specific, not generic. Don't explore files - that's for execution agents."
}
```

### File Manager Agent
```json
{
  "name": "file_manager",
  "description": "File operations specialist",
  "model": "blu_model",
  "tools": [
    "peek_file_top_10_lines",
    "write_file",
    "edit_file",
    "read_file",
    "list_files",
    "read_pdf",
    "load_skill",
    "list_skills",
    "find_relevant_skills",
    "todo_write",
    "todo_list"
  ],
  "system_prompt": "You are a file management specialist. Handle reading, writing, and organizing files efficiently. Use test-driven-development for code changes."
}
```

### Code Analyzer Agent
```json
{
  "name": "code_analyzer",
  "description": "Code analysis and architecture review",
  "model": "grn_model",
  "tools": [
    "peek_file_top_10_lines",
    "read_file",
    "list_files",
    "search_files",
    "request_more_iterations",
    "load_skill",
    "list_skills",
    "find_relevant_skills",
    "todo_write",
    "todo_list"
  ],
  "system_prompt": "You are a code analysis specialist. Understand code structure, patterns, and architecture. Read-only file access, no command execution."
}
```

### Code Reviewer Agent
```json
{
  "name": "code_reviewer",
  "description": "Senior code reviewer",
  "model": "blu_model",
  "tools": [
    "peek_file_top_10_lines",
    "read_file",
    "list_files",
    "search_files",
    "run_command",
    "request_more_iterations",
    "load_skill",
    "list_skills",
    "find_relevant_skills",
    "todo_write",
    "todo_list"
  ],
  "system_prompt": "You are a senior code reviewer. Review completed work against plans and ensure quality standards. MANDATORY: Use requesting-code-review and receiving-code-review skills. Read-only file access, limited command execution (git, cargo, npm, pytest, go)."
}
```

### Search Specialist Agent
```json
{
  "name": "search_specialist",
  "description": "Code search and discovery",
  "model": "blu_model",
  "tools": [
    "peek_file_top_10_lines",
    "read_file",
    "list_files",
    "search_files",
    "load_skill",
    "list_skills",
    "find_relevant_skills",
    "todo_write",
    "todo_list"
  ],
  "system_prompt": "You are a code search specialist. Find specific code patterns, functions, or files efficiently."
}
```

### System Operator Agent
```json
{
  "name": "system_operator",
  "description": "System operations specialist",
  "model": "blu_model",
  "tools": [
    "run_command",
    "peek_file_top_10_lines",
    "write_file",
    "edit_file",
    "read_file",
    "list_files",
    "plan_edits",
    "apply_edit_plan",
    "load_skill",
    "list_skills",
    "find_relevant_skills",
    "todo_write",
    "todo_list"
  ],
  "system_prompt": "You are a system operations specialist. Run shell commands, manage development workflows, and handle system administration. Use test-driven-development, systematic-debugging, and verification-before-completion skills. Read-write file access, all command execution."
}
```

### Terminal Specialist Agent
```json
{
  "name": "terminal_specialist",
  "description": "PTY session management specialist",
  "model": "blu_model",
  "tools": [
    "pty_launch",
    "pty_send_keys",
    "pty_get_screen",
    "pty_list",
    "pty_kill",
    "pty_get_cursor",
    "pty_resize",
    "pty_set_scrollback",
    "pty_start_capture",
    "pty_stop_capture",
    "pty_request_user_input",
    "peek_file_top_10_lines",
    "read_file",
    "list_files",
    "load_skill",
    "list_skills",
    "find_relevant_skills",
    "todo_write",
    "todo_list"
  ],
  "system_prompt": "You are a terminal specialist. Manage interactive terminal sessions and execute commands that require persistent shell environments. Read-only file access, all command execution."
}
```

### Financial Analyst Agent
```json
{
  "name": "financial_analyst",
  "description": "Financial services specialist",
  "model": "blu_model",
  "tools": [
    "peek_file_top_10_lines",
    "read_file",
    "write_file",
    "edit_file",
    "list_files",
    "search_files",
    "run_command",
    "fetch_url",
    "load_skill",
    "list_skills",
    "find_relevant_skills",
    "todo_write",
    "todo_list",
    "request_more_iterations"
  ],
  "system_prompt": "You are a financial analyst. Build financial models, conduct equity research, perform investment banking analysis, and manage wealth management workflows. Build institutional-quality financial deliverables including comps analysis, DCF models, LBO models, CIMs, earnings analysis, and portfolio management. Read-write file access, python/python3/pip command execution, network access."
}
```

## Tool Categories

### File Operations
- `read_file`, `write_file`, `edit_file`, `peek_file_top_10_lines`, `list_files`
- `plan_edits`, `apply_edit_plan`, `read_pdf`

### Search & Discovery
- `search_files`, `find_relevant_skills`, `list_skills`, `load_skill`

### System & Commands
- `run_command`, `fetch_url`

### Task Management
- `todo_write`, `todo_list`, `request_more_iterations`

### PTY/Terminal
- `pty_launch`, `pty_send_keys`, `pty_get_screen`, `pty_list`, `pty_kill`
- `pty_get_cursor`, `pty_resize`, `pty_set_scrollback`, `pty_start_capture`
- `pty_stop_capture`, `pty_request_user_input`

### Model Management
- `llm_oneshot`, `switch_model`

### Agent Control
- `launch_subagent`, `launch_subagent_pretty`

### Memory & Storage
- `store_memory`, `query_memory`, `update_memory`, `delete_memory`
- `list_memories`

## Git Operations Best Practices

### Safe Rebasing
```bash
# Always use GIT_EDITOR=true to avoid interactive prompts
GIT_EDITOR=true git rebase --continue
GIT_EDITOR=true git cherry-pick --continue
```

### Force Push Safety
```bash
# Always try normal push first
git push origin main

# Only use force-with-lease after user confirmation
git push origin main --force-with-lease
```

### Commit Guidelines
- Verify `git status` before committing
- Group logically related files together
- Check file sizes with `git add --dry-run`
- Use environment variables for sensitive config

## Common Development Workflows

### Adding a New Feature
1. Use `test-driven-development` skill
2. Write failing test first
3. Implement minimal code to pass
4. Refactor with `refactoring-for-clarity`
5. Run `cargo test` and `cargo clippy`
6. Request code review via `code_reviewer` agent

### Debugging an Issue
1. Use `systematic-debugging` skill
2. Complete all 4 phases: root cause, pattern analysis, hypothesis testing, implementation
3. Use `root-cause-tracing` for deep errors
4. Verify fix with `verification-before-completion`

### Code Review Process
1. Complete implementation
2. Use `requesting-code-review` skill
3. Dispatch `code_reviewer` agent
4. Receive feedback
5. Use `receiving-code-review` skill
6. Implement improvements
7. Verify with `verification-before-completion`

## Documentation Structure

### Core Documentation
- `AGENTS.md` - Agent system overview (core)
- `docs/project/CLAUDE.md` - Comprehensive architecture
- `docs/architecture/` - Architecture decision records

### Tool Documentation
- `docs/tools/README.md` - Tool overview
- `docs/tools/QUICK_REFERENCE.md` - Quick reference
- `docs/tools/curly_glance_usage.md` - Curly glance tool guide

### Development Guides
- `docs/dev/how_to_new_tool.md` - Adding tools
- `docs/dev/CUSTOMIZING_AGENTS_AND_SKILLS.md` - Customization guide

### Agent Documentation
- `AGENTS.md` - Agent system overview (core)
- `docs/agents/QUICKSTART.md` - Build/test commands
- `docs/agents/CONFIG.md` - Configuration reference
- `docs/agents/REFERENCE.md` - Complete skills list