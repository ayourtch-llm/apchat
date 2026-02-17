---
name: coding-conventions
description: Project-specific coding conventions and constraints. Use when writing, modifying, or refactoring any code, implementing features, or writing tests. Covers two-pass development process, simplicity principles, functional and OO patterns, guard conditions, parameter rules, and testing practices.
---

# Coding Conventions

## Two-Pass Process

Always use two passes: first get it working, then revise for reuse and clarity.

Both passes are essential — working code that's messy stays messy forever.

## Keep it Simple

Keep it simple. Focus on solving the specific problem first. Program close to the requirements.

## Functional / OO

The functional paradigm is powerful, and OO lends good organization. Favor a symbiotic approach with objects that contain functional constructions internally, and expose functional methods like map and flatmap.

## Shallow Call Chains

Use shallow call chains and pass returned artifacts from call to call.

## Guard Conditions

Use guard conditions liberally to exit logic flows early and reduce nesting. Test for bad cases before happy path.

## Granular Functions

Favor tight, granular functions over inlining and deep nesting.

## Parameter Rules

- **No default parameter values.** Default values scatter values all over the code. Better to fail fast. Do not provide default values unless explicitly called for.
- **No optional parameters.** The signature should be the signature, not 2^n variations of it.

## Code Quality

- Write clear, decoupled code with clear names and single responsibilities
- Use guard clauses to handle edge cases early and reduce nesting
- Keep functions small and granular — each does one thing well
- Favor functional internals inside OO containers; expose `map`/`flatMap`-style methods
- Use shallow call chains — pass returned artifacts call to call
- Solve the specific problem first; do not over-engineer ahead of requirements

## Testing

Unit tests are critical. Always write tests as reusable unit tests in the test directory, not throwaway scripts.
