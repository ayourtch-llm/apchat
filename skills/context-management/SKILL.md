---
name: context-management
description: Use when working on long tasks, multi-step investigations, or any session where context window efficiency matters - teaches strategic loading of information, progressive disclosure, and avoiding unnecessary context consumption to maintain effectiveness throughout extended sessions
---

# Context Management

## Overview

LLM context windows are finite. Every file read, every tool output, every message consumes tokens. As a session progresses, context fills up, and eventually the system must summarize — losing detail.

**Core principle:** Treat context like a budget. Load what you need, when you need it, and no more.

## When to Use

**Use when:**
- Working on multi-step tasks (3+ steps)
- Investigating unfamiliar codebases
- Tasks require reading many files
- Deep into a long conversation
- Debugging across multiple files

**Don't use when:**
- Simple single-file changes
- One-shot questions with known answers

## The Three Rules

```
1. PEEK before you READ — use quick scans before loading full files
2. SUMMARIZE before you EXPAND — state what you know before loading more
3. LOAD on demand — don't preload "just in case"
```

## Rule 1: Peek Before You Read

Before reading a full file, check if you actually need all of it.

```
❌ WASTEFUL: Read 500-line file to find one function signature
✅ EFFICIENT: Peek at file structure, then read specific section
```

**Decision tree:**
- Need a specific function/struct? → Search first, read targeted section
- Need to understand file structure? → Peek at top lines or use curly glance
- Need full implementation detail? → Then read the full file

**Pattern:**
1. Use `search_files` or `file_curly_glance` to locate what you need
2. Read only the relevant section with line ranges
3. If you need more context, expand incrementally

## Rule 2: Summarize Before You Expand

Before loading additional information, state what you already know. This serves two purposes:
- Confirms your understanding (catches errors early)
- Creates a compressed record (useful after summarization)

**Pattern:**
```
"Based on what I've read so far:
- File X handles [purpose]
- Function Y takes [params] and returns [result]
- The issue is likely in [area] because [reason]

I need to check [specific thing] next."
```

**Anti-pattern:**
```
❌ Read file A. Read file B. Read file C. Read file D. Read file E.
   Now what was in file A again?

✅ Read file A. "A handles routing."
   Read file B. "B handles auth. Routing calls auth via middleware."
   Now I know where to look: the middleware in A that calls B.
```

## Rule 3: Load on Demand

Don't preload information "just in case." Load it when you have a specific question to answer.

```
❌ PRELOADING: "Let me read all 15 source files to understand the codebase"
✅ ON DEMAND:  "The bug is in parsing. Let me find the parser module."
```

**Decision before each file read:**
1. What specific question am I trying to answer?
2. Do I already have enough information to answer it?
3. What is the minimum I need to load to answer it?

## Practical Patterns

### Pattern: Incremental Investigation

When investigating an unfamiliar area:

1. **Start with structure** — List files, read directory layout
2. **Identify entry points** — Find main files, public APIs
3. **Follow the thread** — Read only along the execution path you care about
4. **Stop when answered** — Don't read "one more file" for completeness

### Pattern: Search-First Discovery

When looking for specific functionality:

1. **Search by keyword** — Find files containing relevant terms
2. **Narrow by file type** — Filter to relevant extensions
3. **Read matches in context** — Read surrounding lines, not full files
4. **Follow references** — Only chase dependencies that matter

### Pattern: Progressive Reporting

When presenting findings to the user:

1. **Lead with the answer** — State what you found first
2. **Provide evidence** — Show the specific code/data that supports it
3. **Offer depth on request** — "I can investigate [X] further if needed"
4. **Don't dump raw data** — Summarize, don't paste entire files

## Common Mistakes

**❌ Reading all imports:** Loading every imported module to "understand dependencies"
**✅ Fix:** Only follow imports that are relevant to your current task

**❌ Re-reading files:** Reading the same file multiple times because you forgot what was in it
**✅ Fix:** Summarize key findings after each read (Rule 2)

**❌ Exploratory reading:** Reading files that "might be relevant"
**✅ Fix:** Have a specific question before each read (Rule 3)

**❌ Full file for one function:** Loading an entire file to understand a single function
**✅ Fix:** Search for the function, read only that section

## Context Budget Checklist

Before loading new information, ask:

- [ ] Do I have a specific question I'm trying to answer?
- [ ] Have I summarized what I already know?
- [ ] Is there a way to get this information with less context usage?
- [ ] Am I loading this because I need it NOW, or "just in case"?

## Real-World Impact

Applying context management consistently results in:
- Longer effective sessions before summarization kicks in
- Better recall of earlier findings (because summaries persist)
- Faster task completion (less time re-reading)
- More focused investigation (less wandering through irrelevant code)
