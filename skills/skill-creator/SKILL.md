---
name: skill-creator
description: Guide for creating effective skills that extend agent capabilities. Use when users want to create a new skill or update an existing skill with specialized knowledge, workflows, or tool integrations.
---

# Skill Creator

This skill provides guidance for creating effective skills.

## About Skills

Skills are modular, self-contained packages that extend agent capabilities by providing specialized knowledge, workflows, and tools. They transform a general-purpose agent into a specialized agent equipped with procedural knowledge.

### What Skills Provide

1. Specialized workflows - Multi-step procedures for specific domains
2. Tool integrations - Instructions for working with specific file formats or APIs
3. Domain expertise - Project-specific knowledge, schemas, business logic
4. Bundled resources - Scripts, references, and assets for complex and repetitive tasks

## Core Principles

### Concise is Key

The context window is a shared resource. Only add context the agent doesn't already have. Challenge each piece of information: "Does the agent really need this explanation?" and "Does this paragraph justify its token cost?"

Prefer concise examples over verbose explanations.

### Set Appropriate Degrees of Freedom

Match the level of specificity to the task's fragility and variability:

**High freedom (text-based instructions)**: Use when multiple approaches are valid, decisions depend on context, or heuristics guide the approach.

**Medium freedom (pseudocode or scripts with parameters)**: Use when a preferred pattern exists, some variation is acceptable, or configuration affects behavior.

**Low freedom (specific scripts, few parameters)**: Use when operations are fragile and error-prone, consistency is critical, or a specific sequence must be followed.

### Anatomy of a Skill

Every skill consists of a required SKILL.md file and optional bundled resources:

```
skill-name/
├── SKILL.md (required)
│   ├── YAML frontmatter metadata (required)
│   │   ├── name: (required)
│   │   └── description: (required)
│   └── Markdown instructions (required)
└── Bundled Resources (optional)
    ├── scripts/          - Executable code
    ├── references/       - Documentation loaded into context as needed
    └── assets/           - Files used in output (templates, icons, fonts, etc.)
```

#### SKILL.md (required)

- **Frontmatter** (YAML): Contains `name` and `description` fields (required). Only `name` and `description` are read to determine when the skill triggers, so be clear and comprehensive about what the skill is and when it should be used.
- **Body** (Markdown): Instructions and guidance for using the skill. Only loaded AFTER the skill triggers.

#### Bundled Resources (optional)

- **Scripts** (`scripts/`): Executable code for tasks that require deterministic reliability or are repeatedly rewritten.
- **References** (`references/`): Documentation loaded as needed into context.
- **Assets** (`assets/`): Files not loaded into context but used in output.

### Progressive Disclosure

Skills use a three-level loading system to manage context efficiently:

1. **Metadata (name + description)** - Always in context (~100 words)
2. **SKILL.md body** - When skill triggers (<5k words)
3. **Bundled resources** - As needed (unlimited)

Keep SKILL.md body to the essentials and under 500 lines. Split content into separate files when approaching this limit.

## Skill Creation Process

1. **Understand the skill** with concrete examples
2. **Plan reusable contents** (scripts, references, assets)
3. **Initialize the skill** directory with SKILL.md
4. **Edit the skill** (implement resources and write SKILL.md)
5. **Test the skill** on real tasks
6. **Iterate** based on real usage

### Writing the Frontmatter

- `name`: The skill name in kebab-case
- `description`: Primary triggering mechanism. Include both what the skill does AND specific triggers/contexts for when to use it. All "when to use" information belongs here — the body is only loaded after triggering.

### Writing the Body

- Use imperative/infinitive form
- Include information that would be beneficial and non-obvious
- Consider what procedural knowledge, domain-specific details, or reusable assets would help execute tasks more effectively
- Keep under 500 lines

### What to Not Include

Do NOT create extraneous documentation or auxiliary files (README.md, CHANGELOG.md, etc.). The skill should only contain the information needed to do the job at hand.
