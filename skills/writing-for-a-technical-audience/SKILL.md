---
name: writing-for-a-technical-audience
description: Use when writing documentation, guides, API references, or technical content - enforces clarity, conciseness, and authenticity while avoiding AI writing patterns
---

# Writing for a Technical Audience

Adapted from ed3d-plugins (https://github.com/ed3dai/ed3d-plugins) by Ed Ropple.
Licensed under CC-BY-SA 4.0.

## Overview

**Core principle:** Technical writing must be clear, concise, and authentic. Avoid AI writing patterns that make content feel robotic or inauthentic.

**Why this matters:** Developers value their time. Clear documentation builds trust. AI-like writing patterns make content feel generic and untrustworthy.

## When to Use

**Use this skill when:**
- Writing API documentation or references
- Creating guides, tutorials, or how-to content
- Documenting code, features, or architecture
- Writing technical blog posts or articles
- Reviewing technical content for clarity

## The Three Pillars

### 1. Clarity

Developers should understand on first read. No re-reading required.

- Short sentences (15-20 words average)
- Short paragraphs (2-4 sentences)
- Active voice over passive
- One concept per paragraph
- Define technical terms on first use

### 2. Conciseness

Every word serves a purpose. Remove noise and filler.

- Delete throat-clearing ("Let me explain," "It's important to note")
- Cut hedging language ("basically," "generally speaking")
- Remove marketing fluff ("powerful," "robust," "seamless")
- Use direct language ("use" not "leverage," "show" not "illuminate")

### 3. Consistency

Same terminology, structure, and voice throughout.

- Pick one term and stick to it (not "endpoint," "URL," "route" interchangeably)
- Use consistent code formatting
- Maintain same tone across all content
- Follow established patterns for similar content types

## Avoid AI Writing Patterns

### AI Phrases to Never Use

| AI Phrase | Use Instead |
|-----------|-------------|
| "delve into" | "explore," "examine," "look at" |
| "leverage" | "use," "take advantage of" |
| "robust" / "seamless" | Be specific about what you mean |
| "at its core" | "fundamentally" (use rarely) or delete |
| "cutting-edge" / "revolutionary" | Describe actual features |
| "streamline" / "optimize" | "speed up," "reduce," "improve" |
| "foster" / "cultivate" | Use direct action verbs |
| "unlock the potential" | State specific outcome |
| "in today's fast-paced world" | Delete entirely |
| "needless to say" | If needless, don't say it - delete |

### Throat-Clearing to Delete

**Never start with:**
- "Let me explain..."
- "It's important to note that..."
- "It's worth noting..."
- "In essence..."
- "Let's explore..."

**Fix:** Start with substance. Delete the preamble.

### Hedging Language to Eliminate

| Hedged | Confident |
|--------|-----------|
| "I think we should..." | "We should..." |
| "It would be great if..." | "Please do X" |
| "Should be able to..." | "Can complete..." |
| "Basically..." | Delete it |
| "Generally speaking..." | Be specific or remove |

### Transition Word Overuse

| Overused AI | Better |
|------------|--------|
| Moreover / Furthermore | Plus, also, and |
| However / Nevertheless | But, though, still |
| Additionally | And, plus |
| Consequently / As a result | So, then |
| That being said | But (or delete) |
| Indeed / Interestingly | Often delete entirely |
| In conclusion | End cleanly without announcing it |

## Technical Writing Patterns

### Explain WHY for These Cases

**ALWAYS explain why when:**

1. **Design decisions with tradeoffs**
   - Good: "We use pagination instead of cursors because it's simpler for most use cases and maintains consistent ordering"
   - Bad: "We use pagination"

2. **Non-obvious patterns**
   - Good: "Row Level Security must be enabled because it enforces security at the database level, preventing bypass through direct SQL access"
   - Bad: "Enable RLS on all tables"

3. **Breaking from conventions**
   - Good: "This API uses POST for reads because GET requests can't include request bodies in some HTTP clients"
   - Bad: "Use POST to fetch data"

### Code Examples: One Excellent Example

**Don't:**
- Implement in 5 languages
- Create fill-in-the-blank templates
- Write perfect-world examples with no error handling

**Do:**
- One complete, runnable example
- Include error handling
- Show realistic usage
- Comment WHY, not what

### Progressive Disclosure

Layer complexity. Simple first, then depth.

1. **Basic explanation** - what it does, core concept
2. **Simple example** - minimal working code
3. **Advanced section** - edge cases, configuration, tradeoffs
4. **Reference** - complete API surface

## Writing That Feels Human

### Use Contractions

- "It's important that you don't..." (not "It is important that you do not...")
- "You'll need to..." (not "You will need to...")

### Vary Sentence Length

Short sentences create emphasis. Longer sentences provide context, explanation, or explore nuance that requires more breathing room. Mix them. Create rhythm naturally.

### Add Personality

- "We tried the obvious solution first and it failed"
- "I found this approach more practical because..."
- Opinions grounded in experience

### Be Specific

- "We reduced latency from 450ms to 120ms" (not "This approach offers significant benefits")
- "Three team members raised concerns about X" (not "Companies have seen improved results")

## Error Messages

Format error messages as lowercase sentence fragments. They compose naturally when chained.

```
Good: failed to parse configuration: invalid JSON at line 42
Bad:  Failed to Parse Configuration: Invalid JSON at Line 42
```

## Red Flags - Review Checklist

Before publishing, check for these issues:

- [ ] No AI phrases ("delve," "leverage," "robust," "at its core")
- [ ] No throat-clearing openings
- [ ] No hedging language ("basically," "generally speaking")
- [ ] No marketing fluff ("powerful," "revolutionary")
- [ ] Sentence length varies
- [ ] Paragraph length varies
- [ ] Contractions used naturally
- [ ] Active voice, clear actors
- [ ] Code examples include error handling
- [ ] WHY explained for design decisions
- [ ] Technical terms defined on first use
- [ ] Specific numbers/names/details (not vague claims)
- [ ] Read aloud test - does it sound natural?

## Summary

**Technical writing in three rules:**

1. **Clear and concise** - Short sentences, short paragraphs, active voice, no filler
2. **Authentic voice** - Contractions, varied rhythm, personality, specific details
3. **Explain why** - Design decisions, tradeoffs, non-obvious patterns need justification

**Avoid AI markers:** No "delve," "leverage," "robust." No throat-clearing. No hedging. No formal transitions.

**One excellent example** beats five mediocre ones. Include error handling. Show realistic usage.

**Read aloud test:** If it sounds robotic or overly formal, rewrite it.
