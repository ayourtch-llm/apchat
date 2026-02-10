---
name: clarify-before-coding
description: Use when requirements are ambiguous, trade-offs exist, or scope is unclear - asks targeted clarifying questions and scopes the work before writing code, preventing wasted effort from misunderstood requirements
---

# Clarify Before Coding

## Overview

The most expensive bugs are requirements bugs — building the wrong thing. Code that works perfectly but solves the wrong problem is worse than no code at all, because it creates false confidence and costs time to undo.

**Core principle:** Invest 2 minutes clarifying to save 20 minutes rebuilding.

## When to Use

**Use when:**
- Requirements have multiple valid interpretations
- Trade-offs exist between approaches (performance vs. simplicity, etc.)
- Scope is unclear (how much to change, which files to touch)
- The task involves user-facing behavior
- You're unsure what "done" looks like

**Don't use when:**
- Requirements are completely unambiguous ("fix this typo in line 42")
- You're following an existing plan that already resolved ambiguities
- The task is mechanical (formatting, renaming, moving files)

**This is NOT brainstorming.** Brainstorming is for refining rough ideas into designs. This skill is a lightweight pre-step: quick clarification before any implementation.

## The Three-Step Protocol

### Step 1: Identify Ambiguities

Before writing any code, scan the request for:

```
AMBIGUITY CHECKLIST:
- [ ] Multiple valid interpretations?
- [ ] Unstated assumptions about behavior?
- [ ] Missing error handling requirements?
- [ ] Unclear scope boundaries?
- [ ] Trade-offs the user may not have considered?
```

If ALL boxes are clear → proceed to coding.
If ANY box is checked → proceed to Step 2.

### Step 2: Ask Targeted Questions

Ask **1-2 specific questions** (not open-ended). Make them easy to answer:

```
✅ GOOD: "Should the search be case-sensitive or case-insensitive?"
✅ GOOD: "Two approaches: (A) add a flag to the existing function, or 
         (B) create a separate function. A is simpler but changes the API. 
         Which do you prefer?"

❌ BAD:  "What do you want the search to do?"
❌ BAD:  "Can you clarify the requirements?"
❌ BAD:  Asking 5+ questions at once
```

**Rules for questions:**
- Maximum 2 questions per round
- Offer concrete options (A/B/C) when possible
- Include your recommendation with reasoning
- Make the default choice clear: "I'd recommend A because [reason], unless you need [specific thing]"

### Step 3: Scope the Work

Before coding, state your plan in 2-3 sentences:

```
"I'll [approach] by modifying [files]. This will [what changes].
I'll verify by [how to test]. This does NOT include [explicit exclusions]."
```

**Why explicit exclusions matter:** Prevents scope creep and sets expectations. "This does NOT change the API" or "This does NOT add new dependencies" prevents misunderstandings.

## Decision Flow

```
User gives task
     │
     ▼
Is the requirement unambiguous?
     │
   YES ──→ State your approach in 1-2 sentences ──→ CODE
     │
    NO
     │
     ▼
Are there trade-offs between approaches?
     │
   YES ──→ Present 2-3 options with recommendation ──→ WAIT FOR ANSWER
     │
    NO
     │
     ▼
Is scope unclear?
     │
   YES ──→ State assumed scope, ask if correct ──→ WAIT FOR ANSWER
     │
    NO
     │
     ▼
Ask 1-2 targeted questions ──→ WAIT FOR ANSWER
```

## Anti-Patterns

### Anti-Pattern 1: Analysis Paralysis

```
❌ BAD: Asking 10 clarifying questions before writing any code
        "What about edge case X? And Y? And Z?"

✅ FIX: Ask about the TOP 1-2 ambiguities. Handle edge cases
        during implementation and flag them as you go.
```

### Anti-Pattern 2: Clarifying the Obvious

```
❌ BAD: "You said 'fix the bug.' Do you mean fix it or not fix it?"

✅ FIX: Only clarify genuine ambiguities. If the intent is clear
        even if details aren't, start coding and ask when you hit
        a specific decision point.
```

### Anti-Pattern 3: Skipping Straight to Code

```
❌ BAD: User says "add authentication" → immediately starts coding JWT
        (Maybe they wanted OAuth? Session-based? API keys?)

✅ FIX: "Authentication can be done with (A) JWT tokens — stateless,
        good for APIs, (B) session cookies — simpler, good for web apps,
        or (C) API keys — simplest, good for service-to-service.
        Given your [context], I'd recommend A. Which approach?"
```

### Anti-Pattern 4: The Premature Apology

```
❌ BAD: "I'm not sure what you want, could you clarify everything?"

✅ FIX: State what you DO understand, then ask about the gap.
        "I understand you want to add caching to the API layer.
         Should the cache be per-request or shared across requests?"
```

## Integration with Other Skills

- **After clarification** → Use **writing-plans** if the task is complex (3+ files)
- **After clarification** → Use **test-driven-development** to implement
- **If rough idea, not a task** → Use **brainstorming** instead (different skill)
- **If debugging, not building** → Use **systematic-debugging** instead

## Quick Reference

| Situation | Action |
|-----------|--------|
| Clear requirement | State approach, start coding |
| Multiple interpretations | Ask 1-2 targeted questions |
| Trade-offs exist | Present options with recommendation |
| Scope unclear | State assumed scope, confirm |
| Vague idea, not a task | Switch to brainstorming skill |

## The Bottom Line

**Don't guess. Don't assume. Clarify.**

But do it quickly. One or two focused questions, not an interrogation. The goal is confidence in the direction, not a complete specification.

If you find yourself asking more than 2 questions before any work, you should be using the brainstorming skill instead.
