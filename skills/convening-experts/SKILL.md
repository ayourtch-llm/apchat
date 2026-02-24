---
name: convening-experts
description: Convenes expert panels for problem-solving. Use when the user mentions panel, experts, multiple perspectives, strategic decisions, root cause analysis, architecture decisions, or process improvement. Simulates 3-5 domain experts collaborating through multi-round discussion.
---

# Convening Experts

Convene domain experts and methodological specialists to solve problems through multi-round collaborative discussion. Experts build on each other's insights, challenge assumptions, and synthesize recommendations.

Adapted from [oaustegard/claude-skills convening-experts](https://github.com/oaustegard/claude-skills/blob/main/convening-experts/SKILL.md).

## Panel Format

### Single-Round Consultation
For simpler problems requiring multiple viewpoints:

1. **Assemble panel** (3-5 experts based on problem domain)
2. **Each expert provides independent perspective** (parallel, not sequential)
3. **Synthesize recommendations** with attribution

### Multi-Round Discussion
For complex problems requiring collaborative reasoning:

1. **Round 1**: Each expert analyzes problem independently
2. **Round 2**: Experts respond to each other's insights, building on or challenging points
3. **Round 3** (if needed): Converge on synthesis, resolve disagreements
4. **Final synthesis**: Integrated recommendations with decision framework

## Panel Convening Logic

Select 3-5 experts based on problem characteristics:

**Problem type → Primary expert + Supporting experts**

- **Technical troubleshooting** → Domain expert + Systems Thinker + Root Cause Analyst
- **Strategic decision** → Strategic Consultant + relevant domain experts + Risk Analyst
- **Architecture design** → Software Architect + Performance Engineer + Security Specialist
- **Process improvement** → Process Engineer + Lean Practitioner + domain expert
- **Root cause analysis** → Domain expert + Root Cause Analyst + Systems Thinker
- **Cross-functional problem** → Relevant domain experts + Integration Specialist + Systems Thinker

## Response Format

### Single-Round Format

```
## Expert Panel: [Topic]

**Panel Members:**
- [Expert 1 Role]
- [Expert 2 Role]
- [Expert 3 Role]

---

### [Expert 1 Role]
[Independent analysis and recommendations]

### [Expert 2 Role]
[Independent analysis and recommendations]

### [Expert 3 Role]
[Independent analysis and recommendations]

---

## Synthesis
[Integrated recommendations with decision framework]
```

### Multi-Round Format

```
## Expert Panel: [Topic]

**Panel Members:**
- [Expert 1 Role]
- [Expert 2 Role]
- [Expert 3 Role]

---

## Round 1: Initial Analysis

### [Expert 1 Role]
[Initial perspective]

### [Expert 2 Role]
[Initial perspective]

---

## Round 2: Cross-Examination

### [Expert 1 Role] responds to [Expert 2 Role]
[Builds on or challenges specific points]

### [Expert 2 Role] responds to [Expert 1 Role]
[Integration or disagreement]

---

## Final Synthesis
[Integrated recommendations, highlighting consensus and productive disagreements]
```

## Expert Behavior Guidelines

**Domain Experts:**
- Apply domain-specific context and constraints
- Use appropriate terminology without over-explanation
- Prioritize practical implementation over theoretical perfection
- Flag domain-specific risks and constraints

**Cross-Panel Interaction:**
- Reference other experts' points specifically ("Building on [Expert]'s observation about...")
- Challenge constructively ("I see it differently because...")
- Synthesize across disciplines
- Flag tensions between perspectives explicitly

**Disagreement Handling:**
- Make disagreements productive (what assumptions differ?)
- Present multiple valid approaches when consensus isn't required
- Identify decision criteria to resolve disagreements

## Decision Frameworks

When panel must recommend action, use one of:

**Weighted Decision Matrix**
- Criteria (importance weighted)
- Options scored on each criterion
- Total score with sensitivity analysis

**Risk-Benefit Analysis**
- Upside potential (probability × impact)
- Downside risk (probability × impact)
- Mitigation strategies

## Activation Decision Tree

```
Is problem complex with multiple valid approaches?
├─ Yes → Expert panel
│   ├─ Spans multiple domains? → Multi-round discussion
│   └─ Needs diverse perspectives? → Single-round consultation
└─ No → Direct answer (don't force panel format)
```

## Constraints

- Use role titles only (e.g., "Software Architect"), not fictional names
- Select experts genuinely relevant to problem
- Each expert must contribute unique insight
- Don't create artificial consensus when legitimate disagreements exist
- Don't force panel format when a direct answer would suffice
- Provide decision-ready synthesis, not just "here are perspectives"
