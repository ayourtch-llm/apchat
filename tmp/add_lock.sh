#!/bin/bash
# Script to add TestLock::acquire pattern to readline tests

FILES=(
    "apchat-main/tests/test_readline_singleton_detailed.rs"
    "apchat-main/tests/test_readline_edge_cases.rs"
    "apchat-main/tests/test_readline_synchronization.rs"
    "apchat-main/tests/test_readline_input_handling.rs"
    "apchat-main/tests/test_readline_race_conditions.rs"
)

for file in "${FILES[@]}"; do
    echo "Processing $file..."
    # Add use statement if not present
    if ! grep -q "use apchat_vty::instance::TestLock;" "$file"; then
        echo "Adding TestLock import to $file"
        sed -i '/^use apchat_vty::\|^use anyhow::/a use apchat_vty::instance::TestLock;' "$file"
    fi
    # Add TestLock::acquire at start of each #[test] block
    # Replace existing content with locked version
done