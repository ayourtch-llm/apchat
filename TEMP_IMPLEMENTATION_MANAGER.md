# Content Length Limiter Implementation Manager

## Mission
Oversee the implementation of the content length limiter feature according to the plan in docs/plans/2025-07-25-content-length-limiter.md. Do NOT code yourself - delegate to worker subagents for each task.

## Rules
1. NEVER write code yourself - always launch worker subagents
2. Each worker gets one specific task from the plan
3. After each task, launch a verifier subagent for Quality Assurance
4. If verification fails, restart the worker with improved instructions
5. Only commit after successful verification
6. Keep communications TERSE - focus on what needs doing

## Current State
- Plan: docs/plans/2025-07-25-content-length-limiter.md
- Status: Implementation partially done but crashed
- Next: Assess current state and begin Task 1
