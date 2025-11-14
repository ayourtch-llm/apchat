#!/usr/bin/env bash
# SessionStart hook for kimichat skills system
# Injects foundational skills at the start of each REPL session

set -euo pipefail

# Determine work directory (current directory or passed as argument)
WORK_DIR="${1:-.}"
SKILLS_DIR="${WORK_DIR}/skills"

# Check if skills directory exists
if [ ! -d "$SKILLS_DIR" ]; then
    # No skills available
    exit 0
fi

# Read the using-superpowers skill if it exists
USING_SKILLS_FILE="${SKILLS_DIR}/using-superpowers/SKILL.md"
if [ -f "$USING_SKILLS_FILE" ]; then
    SKILL_CONTENT=$(cat "$USING_SKILLS_FILE" 2>&1 || echo "Error reading using-superpowers skill")

    # Escape the content for JSON
    SKILL_ESCAPED=$(echo "$SKILL_CONTENT" | sed 's/\\/\\\\/g' | sed 's/"/\\"/g' | awk '{printf "%s\\n", $0}')

    # Output context injection as plain text (kimichat will add it to system prompt)
    cat <<EOF
<EXTREMELY_IMPORTANT>
You have access to skills - proven workflows and techniques that you MUST follow.

**Below is the full content of the 'using-superpowers' skill - your introduction to using skills:**

${SKILL_CONTENT}

</EXTREMELY_IMPORTANT>

Remember: If a skill exists for your task, using it is MANDATORY, not optional.
EOF
else
    # using-superpowers skill not found, provide basic instructions
    cat <<EOF
<IMPORTANT>
You have access to skills - proven workflows and techniques.

Use these tools to work with skills:
- list_skills: See all available skills
- find_relevant_skills: Find skills for your current task
- load_skill: Load and read a specific skill

MANDATORY: Before starting any task, check for relevant skills and follow them exactly.
</IMPORTANT>
EOF
fi

exit 0
