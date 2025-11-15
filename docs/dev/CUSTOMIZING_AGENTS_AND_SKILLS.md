# Customizing Agents and Skills

This guide explains how to add custom agent configurations and skills for repository-specific use in KimiChat.

## Table of Contents

- [Overview: Embedded + Filesystem Architecture](#overview-embedded--filesystem-architecture)
- [Adding Custom Skills](#adding-custom-skills)
- [Adding Custom Agent Configurations](#adding-custom-agent-configurations)
- [Overriding Built-in Configs](#overriding-built-in-configs)
- [Best Practices](#best-practices)
- [Examples](#examples)

---

## Overview: Embedded + Filesystem Architecture

KimiChat uses a **hybrid loading system** that combines embedded and filesystem configs:

```
┌─────────────────────────────────────────┐
│  1. Load Embedded (Built into Binary)  │
│     • 20 skills                         │
│     • 7 agent configs                   │
└─────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────┐
│  2. Scan Filesystem (Optional)          │
│     • skills/ directory                 │
│     • agents/configs/ directory         │
└─────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────┐
│  3. Merge & Override                    │
│     • Filesystem overrides embedded     │
│     • Add new skills/agents             │
└─────────────────────────────────────────┘
```

**Key Benefits:**
- ✅ Binary works standalone (embedded configs)
- ✅ Add repo-specific customizations (filesystem)
- ✅ Override built-in behavior for testing
- ✅ Share custom configs in git repos

---

## Adding Custom Skills

### Skill Directory Structure

```
skills/
└── your-custom-skill/
    └── SKILL.md
```

### Step 1: Create the Skill Directory

```bash
mkdir -p skills/my-custom-skill
```

### Step 2: Create SKILL.md

Every skill needs **YAML frontmatter** followed by markdown content:

```markdown
---
name: my-custom-skill
description: Brief description of what this skill does and when to use it
---

# My Custom Skill

## Overview

Explain what this skill does and why it exists.

## When to Use

**Always:**
- Specific scenario 1
- Specific scenario 2

**Never:**
- When not applicable

## The Process

### Phase 1: Preparation

1. First step with clear instructions
2. Second step with examples
3. Third step with decision criteria

### Phase 2: Execution

**Example workflow:**
```
tool_call: some_tool
parameters:
  param1: value1
```

### Phase 3: Verification

Before marking complete:
- [ ] Checklist item 1
- [ ] Checklist item 2

## Common Pitfalls

⚠️ **Warning about common mistake:**
How to avoid it.

## Examples

### Good Example
```
Show correct usage
```

### Bad Example
```
Show what NOT to do
```
```

### Step 3: Test Your Skill

```bash
# Run KimiChat with agents
cargo run -- --agents -i

# In the REPL:
> /skills
# Should show your new skill

# Test find_relevant_skills
# Your skill should appear for relevant tasks
```

### Step 4: Make It Discoverable

**Skill Naming Tips:**
- Use hyphen-separated lowercase: `repository-analysis`, `api-integration-testing`
- Include action words: `debugging`, `reviewing`, `deploying`
- Be specific: `react-component-testing` not just `testing`

**Description Tips:**
- Start with "Use when..." to clarify applicability
- Mention key tools/concepts: "Uses git bisect for..."
- Explain outcome: "...ensures zero downtime deployments"

---

## Adding Custom Agent Configurations

### Agent Config Directory Structure

```
agents/
└── configs/
    └── your_agent.json
```

### Step 1: Create Agent Config File

```bash
mkdir -p agents/configs
touch agents/configs/my_specialist.json
```

### Step 2: Define Agent Configuration

```json
{
  "name": "my_specialist",
  "description": "Specialist for domain-specific tasks",
  "version": "1.0.0",
  "model": "grn_model",
  "tools": [
    "read_file",
    "write_file",
    "search_files",
    "load_skill",
    "list_skills",
    "find_relevant_skills",
    "todo_write",
    "todo_list"
  ],
  "capabilities": [
    "domain_specific_capability"
  ],
  "system_prompt": "You are a Domain Specialist...",
  "permissions": {
    "file_access": "readwrite",
    "command_execution": [],
    "network_access": false,
    "system_modification": false
  },
  "metadata": {
    "category": "specialized",
    "priority": "medium",
    "scope": "workspace"
  }
}
```

### Step 3: Configure Agent Fields

#### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Unique agent identifier (lowercase, underscores) |
| `description` | string | What this agent does (for planner selection) |
| `version` | string | Semantic version (e.g., "1.0.0") |
| `model` | string | LLM to use: `"blu_model"`, `"grn_model"`, or `"red_model"` |
| `tools` | array | List of tool names this agent can use |
| `system_prompt` | string | Instructions for how the agent should behave |

#### Available Tools

**File Operations:**
- `read_file` - Read file contents
- `write_file` - Create or overwrite files
- `edit_file` - Make targeted edits
- `open_file` - Read with line ranges
- `list_files` - List directory contents
- `search_files` - Search for files by pattern

**Search & Analysis:**
- `search_code` - Search code with regex
- `search_files` - Find files by pattern

**Skills (Mandatory for all agents):**
- `load_skill` - Load a specific skill
- `list_skills` - List all available skills
- `find_relevant_skills` - Find skills for a task

**Task Tracking:**
- `todo_write` - Create/update task lists
- `todo_list` - View current tasks

**System Operations:**
- `execute_command` - Run shell commands
- `batch_execute_commands` - Run multiple commands

**Terminal/PTY (for interactive sessions):**
- `pty_launch` - Start terminal session
- `pty_send_keys` - Send input to terminal
- `pty_get_screen` - Read terminal output
- `pty_list_sessions` - List active sessions
- `pty_kill_session` - Stop terminal session
- (See `terminal_specialist.json` for full PTY toolset)

**Agent Control:**
- `request_more_iterations` - Request more tool call rounds

#### System Prompt Structure

**Recommended Template:**

```
You are a [Agent Role]. Your expertise is in [domain/capability].

═══════════════════════════════════════════════════════════════
🎯 MANDATORY SKILL USAGE
═══════════════════════════════════════════════════════════════

BEFORE starting ANY task, you MUST:
1. Use find_relevant_skills to check for applicable skills
2. If relevant skills found, use load_skill to read them
3. Follow the skill exactly as written - NO exceptions
4. Announce: "I'm using the [skill-name] skill to [what you're doing]"

IF A SKILL EXISTS FOR YOUR TASK, USING IT IS MANDATORY. Not optional.

Common skills you should use:
- test-driven-development: For ANY code changes (write tests first)
- systematic-debugging: For ANY bugs or unexpected behavior
- verification-before-completion: Before marking work complete

═══════════════════════════════════════════════════════════════
📋 TASK TRACKING WITH TODO_WRITE
═══════════════════════════════════════════════════════════════

For complex multi-step tasks (3+ steps), use todo_write to track progress:

**When to use:**
- Complex tasks requiring 3 or more distinct steps
- Multi-file operations or batch processing
- Tasks with dependencies or sequential operations

**When NOT to use:**
- Single straightforward operations
- Trivial tasks completable in 1-2 steps

**Critical Rules:**
1. Exactly ONE task should be in_progress at a time (not zero, not multiple)
2. Mark tasks completed IMMEDIATELY after finishing
3. Only mark completed when FULLY accomplished (not if blocked/errored)
4. Each task needs: content (imperative), status, activeForm (present continuous)

**Example:**
{
  "todos": [
    {"content": "Analyze requirements", "status": "completed", "activeForm": "Analyzing requirements"},
    {"content": "Implement feature", "status": "in_progress", "activeForm": "Implementing feature"},
    {"content": "Write tests", "status": "pending", "activeForm": "Writing tests"}
  ]
}

═══════════════════════════════════════════════════════════════
🎯 YOUR SPECIFIC RESPONSIBILITIES
═══════════════════════════════════════════════════════════════

[Describe what this agent does, how it approaches tasks, best practices, etc.]

When working on tasks:
1. CHECK FOR RELEVANT SKILLS FIRST (mandatory)
2. [Agent-specific step 2]
3. [Agent-specific step 3]
4. Provide clear feedback about actions taken

Focus on [agent's specialty]. [Additional guidance...]
```

### Step 4: Test Your Agent

```bash
# Start with agents mode
cargo run -- --agents -i

# Your agent will be automatically discovered and available to the planner
```

**Verification:**
- Check startup logs for: `✅ Loaded embedded agent configurations`
- Your agent should appear in the planner's available agents
- Test by asking for a task your agent specializes in

---

## Overriding Built-in Configs

### Why Override?

- **Testing**: Modify agent behavior during development
- **Tuning**: Adjust system prompts for your use case
- **Tool access**: Add/remove tools for specific workflows
- **Model selection**: Use different LLMs for specific agents

### How to Override

**Option 1: Override Embedded Skill**

```bash
# Copy embedded skill structure
mkdir -p skills/systematic-debugging
nano skills/systematic-debugging/SKILL.md
# Modify the skill content
```

On startup you'll see:
```
Loaded 20 embedded skills
  ↳ Overriding embedded skill with filesystem version: systematic-debugging
Total skills available: 20
```

**Option 2: Override Embedded Agent**

```bash
# Create filesystem version
cp agents/configs/code_analyzer.json.example agents/configs/code_analyzer.json
nano agents/configs/code_analyzer.json
# Modify configuration
```

On startup you'll see:
```
Loaded 7 embedded agent configurations
↳ Overriding embedded agent with filesystem version: code_analyzer
Total agent configurations available: 7
```

### Common Override Scenarios

#### 1. Add Tools to Existing Agent

```json
{
  "name": "file_manager",
  "tools": [
    "read_file",
    "write_file",
    "edit_file",
    "list_files",
    "execute_command"  // ← Added command execution
    // ... rest of tools
  ]
}
```

#### 2. Change Agent's LLM Model

```json
{
  "name": "code_analyzer",
  "model": "red_model",  // ← Changed from grn_model
  // ... rest of config
}
```

#### 3. Modify System Prompt

```json
{
  "name": "planner",
  "system_prompt": "You are a Planning Agent with focus on [your domain]...\n\n[Add custom instructions]"
}
```

---

## Best Practices

### Skills

✅ **DO:**
- Make skills atomic (one clear purpose)
- Include specific examples and counter-examples
- Use checklists for verification steps
- Explain the "why" behind each rule
- Reference other skills when relevant (`See also: test-driven-development`)

❌ **DON'T:**
- Create vague, generic skills ("be good at coding")
- Skip the YAML frontmatter
- Make skills too long (>500 lines = split it)
- Use ambiguous language ("maybe", "could", "probably")

### Agent Configurations

✅ **DO:**
- Give agents focused, specific responsibilities
- Include skill tools in every agent: `load_skill`, `list_skills`, `find_relevant_skills`
- Use descriptive names: `database_migration_specialist` not `db_agent`
- Add comprehensive system prompts with examples
- Include todo tools for complex agents: `todo_write`, `todo_list`
- Test with representative tasks before deploying

❌ **DON'T:**
- Create "god agents" that do everything
- Give agents conflicting tool access (e.g., read-only agent with `write_file`)
- Skip the mandatory skill usage section in system prompts
- Use production credentials in config files
- Hardcode paths or assumptions specific to your machine

### Organization

**For Team Repos:**
```
skills/
├── company-deployment-process/    # Company-specific workflow
├── security-review-checklist/     # Internal security requirements
└── onboarding-new-developers/     # Team processes

agents/configs/
├── deployment_specialist.json     # Knows company infrastructure
└── security_auditor.json         # Enforces company policies
```

**For Project Repos:**
```
skills/
├── api-integration-testing/       # Project-specific API patterns
└── database-migration-workflow/   # Project DB practices

agents/configs/
├── api_integration_specialist.json
└── migration_coordinator.json
```

---

## Examples

### Example 1: Custom API Testing Agent

**File:** `agents/configs/api_testing_specialist.json`

```json
{
  "name": "api_testing_specialist",
  "description": "Specialist for API testing, integration tests, and contract validation",
  "version": "1.0.0",
  "model": "grn_model",
  "tools": [
    "read_file",
    "write_file",
    "edit_file",
    "list_files",
    "search_code",
    "execute_command",
    "load_skill",
    "list_skills",
    "find_relevant_skills",
    "todo_write",
    "todo_list"
  ],
  "capabilities": [
    "api_testing",
    "integration_testing",
    "contract_validation"
  ],
  "system_prompt": "You are an API Testing Specialist. Your expertise is in testing REST APIs, GraphQL endpoints, and service integrations.\n\n[Include skill usage section from template]\n\nWhen testing APIs:\n1. CHECK FOR RELEVANT SKILLS FIRST (mandatory)\n2. Start by reading existing test patterns in the codebase\n3. Follow test-driven-development skill for all tests\n4. Use systematic-debugging skill for failing tests\n5. Verify response schemas and status codes\n6. Test error cases and edge conditions\n7. Document API behavior and gotchas\n\nFocus on thorough test coverage and clear assertions.",
  "permissions": {
    "file_access": "readwrite",
    "command_execution": ["npm", "yarn", "pnpm", "pytest", "curl"],
    "network_access": false,
    "system_modification": false
  },
  "metadata": {
    "category": "testing",
    "priority": "high",
    "scope": "workspace"
  }
}
```

### Example 2: Custom Deployment Skill

**File:** `skills/zero-downtime-deployment/SKILL.md`

```markdown
---
name: zero-downtime-deployment
description: Use when deploying services to production - ensures zero downtime with health checks, canary releases, and automatic rollback
---

# Zero-Downtime Deployment

## Overview

This skill ensures production deployments have zero customer impact through gradual rollout and automated health verification.

**Core principle:** Deploy incrementally, verify continuously, rollback instantly.

## When to Use

**Always:**
- Production deployments
- Customer-facing services
- Database migrations with schema changes
- Critical infrastructure updates

**Never:**
- Development/staging environments (use simpler process)
- One-off scripts or jobs
- Services without health endpoints

## The Process

### Phase 1: Pre-Deployment Checks

1. **Verify health endpoints exist and work:**
   ```bash
   curl https://api.example.com/health
   # Must return 200 OK with {"status": "healthy"}
   ```

2. **Run full test suite:**
   - All unit tests passing
   - Integration tests passing
   - Contract tests passing (if applicable)

3. **Check monitoring dashboards:**
   - No active incidents
   - Error rates < 0.1%
   - Latency p99 < 500ms

4. **Create deployment checklist:**
   - [ ] Health endpoints verified
   - [ ] Tests passing
   - [ ] Monitoring green
   - [ ] Rollback plan documented
   - [ ] Team notified in #deployments

### Phase 2: Canary Deployment

Deploy to 5% of traffic first:

```bash
# Example with Kubernetes
kubectl set image deployment/api api=api:v2.0.0 --record
kubectl rollout pause deployment/api
# Scale to 5% using traffic splitting
```

**Monitoring window:** 15 minutes minimum

Watch for:
- Error rate increase > 0.5%
- Latency increase > 20%
- Memory/CPU anomalies
- Customer reports

If any issues → **ROLLBACK IMMEDIATELY**

### Phase 3: Gradual Rollout

If canary successful, proceed:
- 5% → 25% (wait 15 min)
- 25% → 50% (wait 15 min)
- 50% → 100% (wait 30 min)

Monitor same metrics at each stage.

### Phase 4: Post-Deployment Verification

After 100% rollout:
- [ ] Run smoke tests against production
- [ ] Verify key user flows work
- [ ] Check error logs for new issues
- [ ] Update deployment log in wiki
- [ ] Announce completion in #deployments

## Rollback Procedure

**Trigger rollback if:**
- Error rate > 1%
- Latency p99 > 1000ms
- Any critical functionality broken
- Customer complaints

**Rollback command:**
```bash
kubectl rollout undo deployment/api
```

Verify rollback succeeded within 2 minutes.

## Common Pitfalls

⚠️ **Skipping health checks:**
Without health checks, you're deploying blind. Always verify endpoints first.

⚠️ **Rolling out too fast:**
Fast = risky. Gradual rollout catches issues before they impact all users.

⚠️ **Ignoring baseline metrics:**
Know your normal error rate before deployment so you can spot anomalies.

## Database Migrations

For schema changes:
1. Deploy backward-compatible schema first
2. Deploy code that works with both schemas
3. Migrate data (can take hours)
4. Deploy code that uses new schema
5. Remove old schema (days/weeks later)

Never deploy breaking schema changes with code simultaneously.
```

### Example 3: Repository-Specific Skill

**File:** `skills/our-pr-review-process/SKILL.md`

```markdown
---
name: our-pr-review-process
description: Company-specific pull request review checklist - mandatory for all PRs before approval
---

# Our PR Review Process

## Overview

All pull requests must pass this checklist before being approved and merged.

## Pre-Review Checklist

Before requesting review, verify:

- [ ] PR title follows format: `[TICKET-123] Brief description`
- [ ] PR description includes:
  - What changed and why
  - Testing performed
  - Screenshots (for UI changes)
  - Migration guide (for breaking changes)
- [ ] All CI checks passing (tests, lint, security scan)
- [ ] No merge conflicts with main branch
- [ ] Branch is up to date with main

## Code Quality Checks

**Functionality:**
- [ ] Code does what PR description claims
- [ ] Edge cases are handled
- [ ] Error cases have appropriate error messages

**Tests:**
- [ ] New features have unit tests
- [ ] Bug fixes have regression tests
- [ ] Tests are meaningful (not just coverage padding)
- [ ] Test names clearly describe what's being tested

**Code Style:**
- [ ] Follows team style guide (auto-formatted)
- [ ] Naming is clear and consistent
- [ ] No commented-out code
- [ ] No debug console.logs or print statements

**Security:**
- [ ] No secrets or credentials in code
- [ ] User input is validated/sanitized
- [ ] SQL queries use parameterized statements
- [ ] Authentication/authorization checked where needed

**Performance:**
- [ ] No N+1 queries
- [ ] Large lists are paginated
- [ ] Expensive operations are cached/optimized
- [ ] No memory leaks (listeners cleaned up)

## Review Process

1. **Self-review first** - Review your own PR as if you're the reviewer
2. **Request 2 reviewers** - One technical lead + one peer
3. **Address feedback** - Respond to all comments, make changes
4. **Re-request review** - After substantial changes
5. **Merge when approved** - After 2 approvals + CI green

## Approval Criteria

**Must have before approval:**
- 2 approvals from team members
- All conversations resolved
- CI passing
- No merge conflicts
- Documentation updated (if needed)
- CHANGELOG.md updated (if user-facing)

## After Merge

- [ ] Delete feature branch
- [ ] Verify deployment to staging
- [ ] Update Jira ticket status
- [ ] Notify stakeholders if needed
```

---

## Troubleshooting

### My skill isn't being found

**Check:**
1. YAML frontmatter is present and valid
2. Directory structure: `skills/skill-name/SKILL.md`
3. Skill name in frontmatter matches directory name
4. No typos in the `name:` field
5. Restart KimiChat to reload skills

**Debug:**
```bash
# Look for your skill in startup logs
cargo run -- --agents -i 2>&1 | grep -i "loaded.*skill"
```

### My agent isn't being used

**Check:**
1. JSON is valid (use `jq` to validate)
2. Agent name is unique (no conflicts with built-in agents)
3. All required fields are present
4. Tools list includes only valid tool names
5. Model field is one of: `blu_model`, `grn_model`, `red_model`

**Debug:**
```bash
# Check agent loading
cargo run -- --agents -i 2>&1 | grep -i "agent configuration"
```

### Filesystem configs aren't overriding embedded ones

**Check:**
1. File is in correct location: `agents/configs/` or `skills/`
2. Name in config matches embedded config exactly
3. File has correct extension (`.json` for agents, `.md` for skills)
4. Look for override message in logs: `↳ Overriding embedded...`

---

## Reference: Built-in Agents

KimiChat includes 7 embedded agents:

| Agent | Model | Purpose |
|-------|-------|---------|
| `planner` | blu_model | Task decomposition and agent coordination |
| `code_analyzer` | grn_model | Code structure, patterns, architecture analysis |
| `code_reviewer` | red_model | Code review with quality standards enforcement |
| `file_manager` | blu_model | General-purpose file operations |
| `search_specialist` | grn_model | Code search and discovery |
| `system_operator` | blu_model | Command execution and batch operations |
| `terminal_specialist` | grn_model | Interactive terminal sessions and PTY management |

See `agents/configs/*.json` for full configurations.

## Reference: Built-in Skills

KimiChat includes 20 embedded skills covering:
- Development workflows (TDD, subagent-driven development)
- Debugging (systematic debugging, root cause tracing)
- Planning (writing plans, executing plans, brainstorming)
- Quality (code review, verification, testing anti-patterns)
- Process (git workflows, branch finishing, waiting strategies)

See `skills/*/SKILL.md` for full content.

---

## Additional Resources

- **Main docs:** `CLAUDE.md` - Overall architecture
- **Agent configs:** `agents/configs/*.json` - Examples of all built-in agents
- **Skills:** `skills/*/SKILL.md` - Examples of all built-in skills
- **Refactoring history:** `REFACTORING_SUMMARY.md` - Implementation details

## Questions?

Create an issue or discussion in the repository for:
- Clarification on agent configuration options
- Suggestions for new built-in agents/skills
- Help with custom agent/skill development
