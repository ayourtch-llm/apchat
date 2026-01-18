# Issue Tracking Process

This directory contains issue tracking for the project. Issues are organized into two subdirectories based on their status.

## Directory Structure

```
docs/issues/
├── README.md          # This file - process documentation
├── open/              # Issues that are identified but not yet resolved
└── resolved/          # Issues that have been fixed
```

## Issue File Naming Convention

Issue files follow the format: `NNN-short-description.md`

- `NNN`: Three-digit issue number (e.g., `001`, `042`, `101`)
- `short-description`: Kebab-case summary of the issue (e.g., `routers-lack-ip-addresses`)

Examples:
- `101-routers-lack-ip-addresses.md`
- `038-ipv6-real-tun-support.md`

## Issue File Format

Each issue file should contain:

```markdown
# Issue NNN: Short Title

## Summary
Brief description of the issue.

## Location
- File: `path/to/file.rs`
- Function/Line: `function_name` or line numbers

## Current Behavior
What currently happens (the bug or missing feature).

## Expected Behavior
What should happen instead.

## Impact
How this affects users or the system.

## Suggested Implementation
Steps or code snippets to fix the issue.

## Resolution
(Added when moving to resolved/)
Description of how the issue was fixed, including:
- What changes were made
- Which files were modified
- Any relevant commit references

---
*Created: YYYY-MM-DD*
*Resolved: YYYY-MM-DD* (if applicable)
```

## Workflow

### Creating a New Issue

1. Determine the next issue number by checking the highest number in both `open/` and `resolved/`
2. Create a new file in `docs/issues/open/` with the appropriate name
3. Fill in all sections except "Resolution"

### Resolving an Issue

1. Fix the issue in the codebase
2. Move the file from `open/` to `resolved/`:
   ```bash
   mv docs/issues/open/NNN-description.md docs/issues/resolved/
   ```
3. Edit the file to add the "Resolution" section describing what was done
4. Update the "Resolved" date at the bottom

### Issue Priority

While not encoded in the filename, issues can note priority in their content:
- **Critical**: Breaks core functionality, must be fixed before deployment
- **High**: Significant bug or missing feature affecting correctness
- **Medium**: Moderate impact, should be fixed soon
- **Low**: Minor issue or enhancement

## Current Issue Number

The highest allocated issue number is tracked by the files present. As of this writing, issue numbers up to **106** have been allocated.

To find the next available number:
```bash
ls docs/issues/open/ docs/issues/resolved/ | grep -oE '^[0-9]+' | sort -n | tail -1
```
