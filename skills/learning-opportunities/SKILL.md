---
name: learning-opportunities
description: Use after completing significant architectural work (new files, schema changes, refactors, unfamiliar patterns) to offer the user optional 10-15 minute interactive learning exercises grounded in evidence-based learning science - helps build genuine expertise rather than just shipping code
---

# Learning Opportunities

Adapted from [DrCatHicks/learning-opportunities](https://github.com/DrCatHicks/learning-opportunities) by Dr. Cat Hicks. Based on evidence-based learning science research.

## Purpose

Help the user build genuine expertise while using AI coding tools, not just ship code. AI-assisted development creates specific risks for learning: accepting generated code skips active processing, clean output creates fluency illusions, and machine velocity eliminates spacing and reflection. These exercises counteract those risks.

## When to Offer Exercises

Offer an optional 10-15 minute exercise after:
- Creating new files or modules
- Database schema changes
- Architectural decisions or refactors
- Implementing unfamiliar patterns
- Any work where the user asked "why" questions during development

**Always ask before starting**: "Would you like to do a quick learning exercise on [topic]? About 10-15 minutes."

## When NOT to Offer

- User declined an exercise this session
- User has already completed 2 exercises this session
- User is in a rush or explicitly focused on shipping

Keep offers brief and non-repetitive. One short sentence is enough.

## Core Principle: Pause for Input

**Do not provide answers until the user responds.** This creates commitment that strengthens encoding and surfaces mental model gaps.

Use explicit markers:

> **Your turn:** What do you think happens when [specific scenario]?
>
> (Take your best guess—wrong predictions are useful data.)

Wait for their response before continuing.

## Exercise Types

### Prediction → Observation → Reflection

1. **Pause:** "What do you predict will happen when [specific scenario]?"
2. Wait for response
3. Walk through actual behavior together
4. **Pause:** "What surprised you? What matched your expectations?"

*Why it works:* Active retrieval strengthens memory traces. Even wrong predictions produce better learning than showing the answer first.

### Generation → Comparison

1. **Pause:** "Before I show you how we handle [X], sketch out how you'd approach it"
2. Wait for response
3. Show the actual implementation
4. **Pause:** "What's similar? What's different, and why do you think we went this direction?"

*Why it works:* Generating your own solution before seeing the answer builds deeper understanding than passive reading.

### Trace the Path

1. Set up a concrete scenario with specific values
2. **Pause at each decision point:** "The request hits [component] now. What happens next?"
3. Wait before revealing each step
4. Continue through the full path

*Why it works:* Step-by-step tracing builds accurate mental models of system behavior.

### Debug This

1. Present a plausible bug or edge case
2. **Pause:** "What would go wrong here, and why?"
3. Wait for response
4. **Pause:** "How would you fix it?"
5. Discuss their approach

*Why it works:* Errors during learning, when followed by corrective feedback, enhance retention compared to error-free learning.

### Teach It Back

1. **Pause:** "Explain how [component] works as if I'm a new developer joining the project"
2. Wait for their explanation
3. Offer targeted feedback: what they nailed, what to refine

*Why it works:* Explaining forces retrieval and reveals gaps that passive familiarity hides.

### Retrieval Check-in (for Returning Sessions)

At the start of a new session on an ongoing project:

1. **Pause:** "Quick check—what do you remember about how [previous component] handles [scenario]?"
2. Wait for response
3. Fill gaps or confirm, then proceed

*Why it works:* Spaced retrieval requires the brain to reconstruct knowledge, strengthening long-term memory. This is more effective than re-reading previous work.

## Techniques to Weave In

**Elaborative interrogation**: Ask "why," "how," and "when else" questions
- "Why did we structure it this way rather than [alternative]?"
- "How would this behave differently if [condition changed]?"
- "In what context might [alternative] be a better choice?"

**Interleaving**: Mix concepts rather than drilling one
- "Which of these three recent changes would be affected if we modified [X]?"

**Varied practice contexts**: Apply the same concept in different scenarios
- "We used this pattern for user auth—how would you apply it to API key validation?"

**Concrete-to-abstract bridging**: After hands-on work, transfer to broader contexts
- "This is an example of [pattern]. Where else might you use this approach?"
- "What's the general principle here that you could apply to other projects?"

## Hands-on Code Exploration

**Prefer directing users to files over showing code snippets.** Having learners locate code themselves builds codebase familiarity and creates stronger memory traces.

### Fading Scaffolding

Adjust guidance based on demonstrated familiarity:

- **Early:** "Open `[file]`, scroll to around line `[N]`, and find the `[function]`"
- **Later:** "Find where we handle `[feature]`"
- **Eventually:** "Where would you look to change how `[feature]` works?"

### Pair Finding with Explaining

After they locate code, prompt self-explanation:

> You found it. Before I say anything—what do you think this line does?

### Example-Problem Pairs

After exploring one instance, have them find a parallel:

> We just looked at how `[function A]` handles `[task]`. Can you find another function that does something similar?

### When to Show Code Directly

- The snippet is very short (1-3 lines) and full context isn't needed
- Introducing new syntax they haven't encountered
- The file is large and searching would be frustrating rather than educational
- They're stuck and need to move forward

## Facilitation Guidelines

- **Ask if they want to engage** before starting any exercise
- **Honor their response time**—don't rush or fill silence
- **Adjust difficulty dynamically**: if they're nailing predictions, increase complexity; if they're struggling, narrow scope
- **Embrace desirable difficulty**: exercises should require effort without being frustrating
- **Offer escape hatches**: "Want to keep going or pause here?"
- **Keep exercises to 10-15 minutes** unless they want to go deeper
- **Be direct about errors**: When they're wrong, say so clearly, then explore why without judgment

## Learning Science Principles

These exercises draw from well-established findings:

| Principle | Risk in AI-Assisted Work | Countermeasure |
|-----------|-------------------------|----------------|
| **Generation effect** | Accepting generated code skips active processing | Prediction and generation exercises |
| **Fluency illusion** | Clean generated code feels understood when it isn't | Retrieval testing and self-explanation |
| **Spacing effect** | Machine velocity pushes toward constant cramming | Session check-ins and deliberate pauses |
| **Metacognition** | Fast workflows suppress learning self-monitoring | Reflection moments and self-assessment |
| **Testing & retrieval** | Complete answers reduce self-testing opportunities | Teach-it-back and retrieval practice |

For detailed scientific rationale, see [PRINCIPLES.md in the original repository](https://github.com/DrCatHicks/learning-opportunities/blob/main/agent-learning%20opportunities/resources/PRINCIPLES.md).
