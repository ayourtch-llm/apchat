# Subagent Implementation Pattern for Plan Execution

## Overview

This document describes the successful pattern for using subagents to implement plans in this codebase. The pattern was developed and tested during the crossterm-readline migration and has proven to be highly effective for coordinated, multi-file implementations.

## When to Use This Pattern

Use the subagent implementation pattern when you have:
- **Well-defined plan files** with clear, structured tasks
- **Multiple related issues** that need coordinated implementation
- **Complex refactoring** requiring consistent changes across multiple files
- **Need for independent execution** where tasks can be completed autonomously

## Pattern Structure

### 1. Prerequisites

You need:
- One or more plan files in `docs/plans/` directory
- Plan files should contain structured tasks with clear objectives
- Tasks should be numbered or clearly delineated
- Related issues in `docs/issues/open/` (optional but recommended)

### 2. Preparation Phase

**Gather the plan files:**

```
List the relevant plan files:
- docs/plans/2025-01-23-crossterm-readline-implementation.md
- docs/plans/2025-01-23-crossterm-readline-migration.md
- docs/plans/2025-01-23-crossterm-readline-implementation-SUMMARY.md
```

**Create a comprehensive task prompt:**

```
You are implementing a crossterm-based readline system to replace rustyline.

You have these plan files that document the work:
- docs/plans/2025-01-23-crossterm-readline-implementation.md
- docs/plans/2025-01-23-crossterm-readline-migration.md
- docs/plans/2025-01-23-crossterm-readline-implementation-SUMMARY.md

Your task is to implement the complete system according to these plans:

1. Create the readline module with the specified structure
2. Implement all required components (history, screen, key handlers, etc.)
3. Integrate with the existing mpsc-based input system
4. Update the REPL to use the new readline instance
5. Add ctrl-r reverse search functionality
6. Ensure all features from rustyline are replicated
7. Test the implementation thoroughly

Work through the plans systematically, implementing each component.
```

### 3. Subagent Launch Phase

**Use the launch_subagent_pretty tool:**

The key is to provide a comprehensive, self-contained task that includes:
- Clear context about what you're building
- References to all relevant plan files
- Specific objectives and success criteria
- Independence (no need for user interaction)

**Example prompt structure:**

```
You are implementing [FEATURE NAME] according to these plans:
- [path to plan file 1]
- [path to plan file 2]
- [path to plan file 3]

Context: [Brief description of what you're doing and why]

Your tasks:
1. [Specific task 1 with clear objective]
2. [Specific task 2 with clear objective]
3. [Specific task 3 with clear objective]
...

Requirements:
- Follow the plans exactly
- Ensure all changes are consistent
- Test your implementation
- Report any issues found

You are working independently. Complete all tasks without asking for clarification.
```

### 4. Monitoring and Iteration

**Monitor the subagent output:**

The `launch_subagent_pretty` tool provides:
- Real-time feedback on progress
- Formatted JSON output for readability
- Clear indication of files modified
- Task completion status

**Handle issues iteratively:**

If the subagent encounters issues:
1. Launch another subagent with specific instructions to fix the issues
2. Reference the original plan files
3. Provide specific error messages or areas needing attention
4. Continue until all tasks are complete

**Example follow-up prompt:**

```
The previous implementation has these issues:
- [Issue 1 with specific details]
- [Issue 2 with specific details]

Fix these issues while maintaining compatibility with:
- docs/plans/2025-01-23-crossterm-readline-implementation.md

Make minimal changes to fix only the reported issues.
```

### 5. Verification Phase

**After subagent completion:**

1. Review the modified files
2. Run tests if available
3. Check that all plan requirements are met
4. Verify integration with existing code
5. Create any necessary documentation

## Key Success Factors

### 1. Comprehensive Planning

- Plans should be detailed and specific
- Include file structures, APIs, and interfaces
- Define clear success criteria
- Consider dependencies and ordering

### 2. Clear Task Definition

- Each task should have a clear objective
- Tasks should be independent where possible
- Provide enough context for autonomous execution
- Include constraints and requirements

### 3. Self-Contained Prompts

- Include all relevant context in the prompt
- Reference plan files explicitly
- Define the scope clearly
- Specify any constraints or preferences

### 4. Iterative Refinement

- Don't expect perfection in one pass
- Use multiple subagent launches if needed
- Each iteration should build on the previous
- Keep focus narrow for follow-up tasks

## Example: Successful Implementation

The crossterm-readline migration followed this pattern:

**Initial launch:**
- 3 plan files provided
- 7 main tasks defined
- Comprehensive prompt with full context

**Result:**
- Complete readline module implemented
- All features from rustyline replicated
- Proper mpsc integration
- 14 files created/modified
- Working in 3 subagent launches

**Follow-up launches:**
- Launch 1: Fix module organization
- Launch 2: Add ctrl-r search
- Launch 3: Final testing and cleanup

## Template for Future Use

When you need to implement a plan using this pattern, use this template:

```
Implement [FEATURE NAME] according to these plans:
- docs/plans/[PLAN-FILE-1].md
- docs/plans/[PLAN-FILE-2].md
- docs/plans/[PLAN-FILE-3].md

Context:
We are implementing [FEATURE] to [PURPOSE]. This involves [BRIEF DESCRIPTION].

Your tasks:
1. [TASK 1]: [Clear objective]
2. [TASK 2]: [Clear objective]
3. [TASK 3]: [Clear objective]
...

Requirements:
- Follow the plans exactly as specified
- Ensure code quality and consistency
- Integrate properly with existing code
- Test your implementation thoroughly
- Report any issues or deviations from the plans

You are working independently. Complete all tasks without asking for clarification.
Work through the plans systematically, implementing each component in order.
```

## Best Practices

1. **Start with comprehensive plans** - Good plans lead to good implementations
2. **Be specific in tasks** - Vague tasks lead to vague results
3. **Provide full context** - Don't assume the subagent knows background
4. **Reference files explicitly** - Use full paths to plan files
5. **Allow for iteration** - Multiple focused passes are better than one broad one
6. **Monitor progress** - Use the pretty output to track what's happening
7. **Verify thoroughly** - Check results against the original plans

## Common Pitfalls to Avoid

1. **Too vague in prompt** - "Fix the code" vs "Fix the panic in pty_write caused by..."
2. **Missing context** - Not explaining what you're building or why
3. **Too much at once** - Try to break complex work into logical phases
4. **Not following plans** - Deviating from documented approach
5. **No verification** - Not checking if implementation matches plans

## Related Documentation

- Plan template: `docs/plans/[PLAN-FILE].md`
- Issue tracking: `docs/issues/`
- Subagent specification: `docs/project/subagent.md`

## Conclusion

The subagent implementation pattern is a powerful approach for executing well-planned, complex implementations. By providing comprehensive plans, clear tasks, and allowing for iterative refinement, you can achieve consistent, high-quality results while maintaining the autonomy and focus of subagent execution.

When you have a set of plans to implement, follow this pattern and adapt the template to your specific needs. The pattern has been proven to work effectively in this codebase.
