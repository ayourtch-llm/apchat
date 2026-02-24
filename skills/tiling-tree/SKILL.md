---
name: tiling-tree
description: Exhaustive problem space exploration using the MIT Synthetic Neurobiology "tiling tree" method. Partitions a problem into MECE (Mutually Exclusive, Collectively Exhaustive) subsets recursively via subagents, then evaluates leaf ideas against specified criteria. Use when users say "tiling tree", "tile the solution space", "exhaustively explore approaches to", "what are all the ways to", or request a MECE breakdown of a problem.
---

# Tiling Tree

Implements the MIT Synthetic Neurobiology tiling tree method: recursively partition a problem space into non-overlapping, collectively exhaustive subsets until reaching actionable leaf ideas, then evaluate those leaves.

Adapted from [oaustegard/claude-skills tiling-tree](https://github.com/oaustegard/claude-skills/blob/main/tiling-tree/SKILL.md).

## Core Concept

The method's power comes from MECE splits forcing exploration of unfamiliar territory. A split is only valid when you can state precisely what each branch **excludes** — if you can't, the criterion is too vague and branches will overlap.

Key insight from the source method: always look for the "third option" that falls outside an obvious binary split. The bloodstream-secretion approach to neural recording only emerged because "wired vs. wireless" was defined precisely enough to reveal it covered neither case.

## When to Use

- "What are all the ways we could solve X?"
- "Apply the tiling tree method to Y"
- "Exhaustively map the solution space for Z"
- Any request for MECE decomposition of a problem domain

## Parameters

| Parameter | Default | Notes |
|-----------|---------|-------|
| Problem | required | Natural language problem statement |
| Depth | 2 | Max recursion depth. Depth 2 ≈ 16 leaves, depth 3 ≈ 64 leaves |
| Criteria | impact, novelty, feasibility | Evaluation dimensions for scoring leaves |

**Depth guidance:** Start with depth 2 to validate the problem framing. Increase to 3 only when the domain genuinely warrants it — depth 3 generates ~64 leaves.

## Process

### Phase 1: Frame the Problem

State the problem clearly and identify:
1. The **domain boundary** — what's in scope and what's not
2. The **output format** — what does a "leaf idea" look like (a technique, a design, a strategy, etc.)
3. The **evaluation criteria** — how leaves will be scored (default: impact, novelty, feasibility)

### Phase 2: Build the Tree (Level by Level)

For each level of the tree, use `launch_subagent` to split nodes into MECE branches.

**Level 1 — Root Split:**
Identify 3-5 top-level categories that partition the entire problem space. For each branch, state:
- **Branch name**: A short label
- **Split criterion**: The dimension used to divide (e.g., "energy source type")
- **Exclusion statement**: What this branch explicitly excludes

**Subsequent Levels:**
For each non-leaf node from the previous level, dispatch a subagent to split it further:

```
launch_subagent with instruction:
  "Split the following node into 3-5 MECE sub-branches.

   Parent node: [node name and description]
   Domain context: [original problem statement]

   For each branch provide:
   1. Branch name
   2. Split criterion used
   3. Exclusion statement (what this branch does NOT cover)
   4. Brief description

   Return the result as a structured list.
   If a branch is already actionable (specific enough to evaluate), mark it as a LEAF."
```

**MECE Validation at Each Split:**
Before proceeding, verify:
- **Mutually Exclusive**: No idea could reasonably belong to two branches
- **Collectively Exhaustive**: Can you think of an approach NOT covered? If yes, add a branch or redefine criteria
- **Third Option Test**: For any binary split, actively search for something that fits neither side

### Phase 3: Evaluate Leaves

Once the tree reaches the target depth or all nodes are marked as leaves, evaluate each leaf against the criteria.

Use a subagent for consistent cross-leaf evaluation:

```
launch_subagent with instruction:
  "Score each of the following leaf ideas on a 1-5 scale for each criterion.

   Criteria: [impact, novelty, feasibility] (or user-specified)
   
   Leaf ideas:
   [list all leaves with their full path in the tree]

   For each leaf provide:
   - Scores for each criterion
   - Overall score (average)
   - One-sentence rationale

   Return as a ranked table sorted by overall score."
```

### Phase 4: Synthesize Output

Produce a markdown document containing:
1. **Full tree diagram** with split criteria noted at each branch point
2. **Ranked leaf table** sorted by overall score, including:
   - Leaf name and tree path
   - Scores per criterion
   - Overall score
   - One-sentence rationale
3. **Surprising finds** — highlight any leaf that wouldn't have been discovered without the systematic MECE decomposition

## Interpreting Results

Good trees have:
- Split criteria that are **definitions**, not questions ("energy source type" not "is it renewable?")
- Leaf exclusions that **confirm non-overlap**
- A **"surprising" branch** — something you wouldn't have thought of without the tree

If all leaves feel obvious, the split criteria were too coarse. Redo the tree with more precise definitions at the branch level where it went flat.

## Example

**Problem:** "What are all the ways to reduce CI build times?"

**Level 1 split by "where time is spent":**
1. **Dependency resolution** (excludes compilation, testing, deployment)
2. **Compilation/build** (excludes dependency fetching, test execution)
3. **Test execution** (excludes build steps, deployment)
4. **Infrastructure/orchestration** (excludes application-level optimizations)

**Level 2 for "Compilation/build":**
1. **Caching strategies** (excludes code changes, parallelism)
2. **Parallelization** (excludes caching, code structure changes)
3. **Code structure optimization** (excludes infra changes)
4. **Toolchain selection** (excludes changes to project code or infra)

Each leaf is then scored on impact, novelty, and feasibility.
