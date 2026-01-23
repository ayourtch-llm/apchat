#!/bin/bash
# Quick test to verify apchat starts without readline history errors

cd apchat-main

echo "Testing apchat startup..."
echo "Press Ctrl+C to exit the application"
echo ""

(timeout 3 cargo run --release -- --stream --interactive 2>&1 || true) | grep -E "(readline|Recovered|Loaded|error|Error)" || true

echo ""
echo "If you see 'Loaded X readline history entries' above, the fix worked!"
