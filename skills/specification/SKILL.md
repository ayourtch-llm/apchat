---
name: specification
description: Write and maintain technical specifications. Use when the user asks to create, update, or review a SPEC.md file, or when planning a new feature or application that needs a specification document. Also use when the user mentions spec, specification, requirements document, or asks to document what a system should do.
---

# Technical Specification Writing

Specs document **what** a system does, not **how** it's implemented.

They serve as:
- Reference documentation for developers
- Requirements tracking for features
- Testing guidance for QA
- Communication tool between stakeholders

## Language Requirements

Use RFC 2119 keywords for requirement levels:
- **MUST** / **MUST NOT** — Absolute requirements
- **SHOULD** / **SHOULD NOT** — Recommended but not mandatory
- **MAY** — Optional features

Write in present tense, implementation-agnostic language. Be specific about expected behavior. No code snippets.

## Required Sections

1. **Purpose** — What the system does and why
2. **UI Layout** — ASCII diagrams of component layout
3. **Functional Requirements** — Grouped by domain (e.g., Input, Processing, Output, API Integration, Storage)
4. **Non-Functional Requirements** — Performance, styling, accessibility, security
5. **Dependencies** — Libraries and frameworks with versions
6. **Implementation Notes** — Technical details too specific for requirements (framework quirks, data formats, architectural decisions)
7. **Error Handling** — Expected error conditions and responses

### Functional Requirements

Group related requirements under clear section headers:

```markdown
### Section Name

- The application MUST do X
- The application SHOULD do Y
- The feature MAY support Z
```

Common categories:
- API Integration
- Data Input/Output
- User Interface Controls
- Status/Feedback
- Communication/Networking
- Persistence/Storage

### Non-Functional Requirements

Common categories:
- Styling/Theming
- Code Quality
- Performance
- Browser/Platform Compatibility
- Accessibility
- Security

### Implementation Notes

This section captures technical details that are too specific for requirements but important to document:

- Framework-specific behaviors
- Data format considerations
- API integration patterns
- Browser API quirks and workarounds
- Key architectural decisions that affect implementation

## Formatting Guidelines

- Use bullet points with MUST/SHOULD/MAY, not numbered requirement IDs. Bullet points are easier to maintain — adding or reordering doesn't require renumbering.
- Nest sub-items for detail
- Each MUST requirement should be testable
- Use ASCII art to show UI component layout

## Be Specific About States

Document all states: initial, transitions, success, error, edge cases.

**Vague:** "Buttons should be disabled sometimes"
**Specific:** "The submit button MUST be disabled until the form is valid" and "The submit button MUST be disabled while request is in progress"

## What to Avoid

- No code snippets or syntax examples — describe behavior, not implementation
- No SQL definitions, only tables of properties
- No implementation-specific language — "The system MUST filter invalid items" not "Use Array.prototype.filter()"
- No design patterns in specs — "The application MUST maintain a single API connection" not "Use a singleton pattern"

## File Organization

- Name the file `SPEC.md` in the project root
- Split into `SPEC_*.md` files if too large
- Keep synchronized with implementation

## Review Checklist

Before finalizing, verify:
- All MUST requirements are testable
- SHOULD vs MUST is used appropriately
- No code snippets or syntax examples
- Requirements are implementation-agnostic
- UI layout is visually documented
- Error conditions are documented
- Dependencies are listed with versions
- Language is clear and specific
