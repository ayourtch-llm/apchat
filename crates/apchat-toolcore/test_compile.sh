#!/bin/bash
# Quick compilation test

echo "Testing parameter_validation.rs compilation..."

cd crates/apchat-toolcore

# Try to compile just the source file
rustc --crate-type lib src/parameter_validation.rs \
    --extern serde_json=/Users/ayourtch/.cargo/bin/serde_json \
    --extern serde=/Users/ayourtch/.cargo/bin/serde \
    --extern std=/usr/lib/rustlib/x86_64-apple-darwin/lib/libstd-*.rlib \
    --extern anyhow=/Users/ayourtch/.cargo/bin/anyhow \
    2>&1 | head -20

if [ $? -eq 0 ]; then
    echo "✓ Compilation successful!"
else
    echo "✗ Compilation failed"
fi
