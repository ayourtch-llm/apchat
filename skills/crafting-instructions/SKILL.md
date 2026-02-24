---
name: crafting-instructions
description: Generate optimized instructions and prompts for LLMs. Use when the user requests help writing effective prompts, creating system instructions, building agent prompts, or needs guidance on prompt engineering techniques.
---

# Crafting Instructions

Generate optimized instructions for LLMs, focusing on effective prompt engineering principles that produce better results.

Adapted from [oaustegard/claude-skills crafting-instructions](https://github.com/oaustegard/claude-skills/blob/main/crafting-instructions/SKILL.md).

## Core Optimization Principles

### 1. Imperative Construction
Frame as direct action commands, not suggestions:
- ❌ "Consider creating X" → ✅ "Create X when conditions Y"
- ❌ "You might want to" → ✅ "Execute" / "Generate"
- ❌ "Try to optimize" → ✅ "Optimize by"

### 2. Positive Directive Framing
State WHAT to do, not what NOT to do:
- ❌ "Don't use bullet points" → ✅ "Write in flowing paragraph form"
- ❌ "Avoid technical jargon" → ✅ "Use accessible language for beginners"

Negative instructions force inference. Positive instructions state desired behavior directly.

### 3. Context and Motivation
Explain WHY requirements exist:
- ❌ "Use paragraph form"
- ✅ "Use paragraph form because flowing prose is more conversational for casual learning"

Context helps the LLM make better autonomous decisions in edge cases.

### 4. Strategic Over Procedural
Provide goals and decision frameworks, not step-by-step procedures:
- Specify: Success criteria, boundaries, decision frameworks
- Minimize: Sequential steps, detailed execution, obvious operations
- Rule: If the LLM can infer procedure from goal, specify only the goal

### 5. Trust Base Behavior
LLMs already have built-in capabilities. ONLY specify domain-specific deviations from default behavior.

## Instruction Categories

### System Prompts
- Additive to base capabilities (no duplication of default behavior)
- Focus on domain-specific behavior modifications
- Simple structure (headings/paragraphs) unless complexity demands more

### Agent Instructions
- Define role, capabilities, and constraints
- Use progressive disclosure (overview → details → edge cases)
- Include trigger patterns for when the agent should activate

### Task Prompts
- Clear and explicit about desired output format
- Provide context and examples when helpful
- Scale complexity to task needs
- Give permission to express uncertainty

## Quality Checklist

Before delivering instructions:

**Strategic:**
- [ ] Clear goals stated without micromanagement
- [ ] Context explains WHY requirements exist
- [ ] Decision frameworks for ambiguous cases
- [ ] Constraints use positive framing when possible

**Technical:**
- [ ] Imperative language throughout
- [ ] Positive directives over negative restrictions
- [ ] Appropriate structure (simple by default)
- [ ] No duplication of default LLM behavior
- [ ] Examples (if any) perfectly aligned with desired output

**Execution:**
- [ ] Immediately actionable
- [ ] Success criteria clear
- [ ] Format matches complexity needs

## Common Mistakes to Avoid

- **Duplicating defaults** - Don't tell the LLM to do things it already does
- **Negative framing** - "Don't use lists" → "Present in natural prose paragraphs"
- **Procedural micromanagement** - "Step 1: X, Step 2: Y" → "Goal: X. Quality standard: Y."
- **Contextless requirements** - "Always use formal tone" → "Use formal tone because recipients expect authoritative voice"
- **Imperfect examples** - Examples teach ALL patterns, including unintended ones. Every detail matters.

## Complexity Scaling

Match instruction complexity to task needs:

**Simple task** → Simple prompt or brief instructions
**Medium task** → Structured guidance with decision frameworks
**Complex task** → Comprehensive instructions with examples and edge cases
