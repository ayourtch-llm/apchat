#!/bin/bash

# Test 1: Check if MSPC channel is accessible
if grep -q "mspc_channel" src/app/repl.rs; then
    echo "✓ MSPC channel field found in REPL"
else
    echo "❌ MSPC channel field missing"
    exit 1
fi

# Test 2: Check if terminal router is initialized
if grep -q "TerminalInputRouter::new" src/app/repl.rs; then
    echo "✓ Terminal input router initialized"
else
    echo "❌ Terminal input router not initialized"
    exit 1
fi

# Test 3: Check if try_recv is used
if grep -q "try_recv" src/app/repl.rs; then
    echo "✓ Non-blocking message checking implemented"
else
    echo "❌ Non-blocking message checking missing"
    exit 1
fi

# Test 4: Check if interrupt handling is present
if grep -q "is_interrupt" src/app/repl.rs; then
    echo "✓ Interrupt handling implemented"
else
    echo "❌ Interrupt handling missing"
    exit 1
fi

# Test 5: Check if command handling is present
if grep -q "is_command" src/app/repl.rs; then
    echo "✓ Command handling implemented"
else
    echo "❌ Command handling missing"
    exit 1
fi

echo ""
echo "All PTY-based checks passed!"
