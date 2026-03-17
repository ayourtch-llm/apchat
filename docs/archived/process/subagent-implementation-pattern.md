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

**Issue Tracking Pattern:**

Before implementation, create individual issue files for each major task in `docs/issues/open/`:
- Each issue gets its own file: `docs/issues/open/XXX-issue-title.md`
- Number issues sequentially (e.g., 101, 102, 103...)
- Include issue description, requirements, and acceptance criteria
- Reference these issues in your subagent prompt

See `docs/issues/README.md` for the complete issue tracking process.

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
- References to open issue files that need to be resolved
- Specific objectives and success criteria
- Independence (no need for user interaction)

**Issue Resolution Instructions:**

When instructing the subagent, explicitly tell it to:
1. Implement the task according to the plan
2. Move the issue file from `docs/issues/open/` to `docs/issues/resolved/`
3. Update the issue file with implementation details
4. Mark issues as resolved with completion notes

**Example prompt structure:**

```
You are implementing [FEATURE NAME] according to these plans:
- [path to plan file 1]
- [path to plan file 2]
- [path to plan file 3]

Issues to resolve:
- docs/issues/open/101-issue-one.md
- docs/issues/open/102-issue-two.md
- docs/issues/open/103-issue-three.md

Context: [Brief description of what you're doing and why]

Your tasks:
1. [Issue 101] [Specific task 1 with clear objective]
2. [Issue 102] [Specific task 2 with clear objective]
3. [Issue 103] [Specific task 3 with clear objective]
...

Requirements:
- Follow the plans exactly
- Ensure all changes are consistent
- Test your implementation
- Report any issues found

Issue Resolution:
For each issue you resolve:
1. Read the issue file from docs/issues/open/
2. Implement the solution described
3. Move the issue file to docs/issues/resolved/
4. Add "Status: RESOLVED" and implementation notes
5. Reference the commit in the issue file

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
- docs/issues/resolved/101-related-issue.md

Make minimal changes to fix only the reported issues.

When complete, update any open issue files to resolved status as appropriate.
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

**Preparation:**
- Created 3 comprehensive plan files
- Created 24 issue files in docs/issues/open/ (101-124)
- Each issue represented a specific task or component

**Initial launch:**
- 3 plan files provided
- 24 issue files referenced
- 7 main task categories defined
- Comprehensive prompt with full context

**Issue Resolution Process:**
The subagent systematically:
1. Read each issue from docs/issues/open/
2. Implemented the solution
3. Moved issue to docs/issues/resolved/
4. Added status and implementation notes
5. Continued through all 24 issues

**Result:**
- Complete readline module implemented
- All features from rustyline replicated
- Proper mpsc integration
- 14 files created/modified
- All 24 issues resolved and moved to docs/issues/resolved/
- Working in 3 subagent launches

**Follow-up launches:**
- Launch 1: Fix module organization (resolved issues 101-108)
- Launch 2: Add ctrl-r search (resolved issues 109-118)
- Launch 3: Final testing and cleanup (resolved issues 119-124)

## Template for Future Use

When you need to implement a plan using this pattern, use this template:

```
Implement [FEATURE NAME] according to these plans:
- docs/plans/[PLAN-FILE-1].md
- docs/plans/[PLAN-FILE-2].md
- docs/plans/[PLAN-FILE-3].md

Issues to resolve:
- docs/issues/open/XXX-[issue-name].md
- docs/issues/open/YYY-[issue-name].md
- docs/issues/open/ZZZ-[issue-name].md

Context:
We are implementing [FEATURE] to [PURPOSE]. This involves [BRIEF DESCRIPTION].

Your tasks:
1. [Issue XXX] [TASK 1]: [Clear objective]
2. [Issue YYY] [TASK 2]: [Clear objective]
3. [Issue ZZZ] [TASK 3]: [Clear objective]
...

Requirements:
- Follow the plans exactly as specified
- Ensure code quality and consistency
- Integrate properly with existing code
- Test your implementation thoroughly
- Report any issues or deviations from the plans

Issue Resolution Process:
For each issue:
1. Read the issue file from docs/issues/open/
2. Implement according to the issue requirements
3. Move the file to docs/issues/resolved/
4. Update the file with:
   - Status: RESOLVED
   - Implementation date
   - Key changes made
   - Test results
5. Reference commit hashes if applicable

See docs/issues/README.md for complete issue tracking guidelines.

You are working independently. Complete all tasks without asking for clarification.
Work through the plans systematically, implementing each component in order.
```

## Best Practices

1. **Start with comprehensive plans** - Good plans lead to good implementations
2. **Create detailed issue files** - Each issue should be clear and actionable
3. **Be specific in tasks** - Vague tasks lead to vague results
4. **Provide full context** - Don't assume the subagent knows background
5. **Reference files explicitly** - Use full paths to plan and issue files
6. **Allow for iteration** - Multiple focused passes are better than one broad one
7. **Monitor progress** - Use the pretty output to track what's happening
8. **Verify thoroughly** - Check results against the original plans
9. **Track issue resolution** - Ensure all issues are moved to resolved/
10. **Document outcomes** - Update issue files with implementation details

## Common Pitfalls to Avoid

1. **Too vague in prompt** - "Fix the code" vs "Fix the panic in pty_write caused by..."
2. **Missing context** - Not explaining what you're building or why
3. **Too much at once** - Try to break complex work into logical phases
4. **Not following plans** - Deviating from documented approach
5. **No verification** - Not checking if implementation matches plans

## Related Documentation

- This document: `docs/process/subagent-implementation-pattern.md`
- Plan template: `docs/plans/[PLAN-FILE].md`
- Issue tracking process: `docs/issues/README.md` ⚠️ **IMPORTANT**
- Subagent specification: `docs/project/subagent.md`

**Issue Tracking Workflow:**

See `docs/issues/README.md` for:
- How to create issue files
- Issue file format and structure
- The open/ vs resolved/ workflow
- Status tracking and metadata
- Best practices for issue management

## Conclusion

The subagent implementation pattern is a powerful approach for executing well-planned, complex implementations. By providing comprehensive plans, clear tasks, and allowing for iterative refinement, you can achieve consistent, high-quality results while maintaining the autonomy and focus of subagent execution.

When you have a set of plans to implement, follow this pattern and adapt the template to your specific needs. The pattern has been proven to work effectively in this codebase.
