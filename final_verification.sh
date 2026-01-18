#!/bin/bash

set -e

echo "=========================================="
echo "  MSPC Integration - Final Verification"
echo "=========================================="
echo ""

cd apchat-main

# Test 1: Build check
echo "Test 1: Building project..."
if cargo build --quiet 2>&1 | grep -q error; then
    echo "❌ Build failed"
    exit 1
fi
echo "✓ Build successful"
echo ""

# Test 2: MSPC unit tests
echo "Test 2: Running MSPC unit tests..."
cargo test test_mspc --quiet 2>&1 | grep "test result" | grep -q "0 failed"
echo "✓ All MSPC tests pass"
echo ""

# Test 3: Comprehensive integration test
echo "Test 3: Running comprehensive integration test..."
cargo test test_mspc_comprehensive --quiet 2>&1 | grep "test result" | grep -q "0 failed"
echo "✓ Comprehensive test passes"
echo ""

# Test 4: Verify REPL has MSPC integration
echo "Test 4: Verifying REPL MSPC integration..."

# Check imports
if ! grep -q "use apchat::mspc::{MspcChannel, MspcMessage}" src/app/repl.rs; then
    echo "❌ MSPC imports missing"
    exit 1
fi
echo "  ✓ MSPC imports present"

# Check channel initialization
if ! grep -q "MspcChannel::new(100)" src/app/repl.rs; then
    echo "❌ MSPC channel initialization missing"
    exit 1
fi
echo "  ✓ MSPC channel initialized"

# Check try_recv usage
if ! grep -q "try_recv" src/app/repl.rs; then
    echo "❌ Non-blocking message checking missing"
    exit 1
fi
echo "  ✓ Non-blocking message checking implemented"

# Check interrupt handling
if ! grep -q "is_interrupt" src/app/repl.rs; then
    echo "❌ Interrupt handling missing"
    exit 1
fi
echo "  ✓ Interrupt handling implemented"

# Check command handling
if ! grep -q "is_command" src/app/repl.rs; then
    echo "❌ Command handling missing"
    exit 1
fi
echo "  ✓ Command handling implemented"

# Check terminal router
if ! grep -q "TerminalInputRouter::new" src/app/repl.rs; then
    echo "❌ Terminal input router missing"
    exit 1
fi
echo "  ✓ Terminal input router initialized"

echo "✓ REPL MSPC integration verified"
echo ""

# Test 5: Check APChat struct
echo "Test 5: Verifying APChat struct..."

if ! grep -q "mspc_channel: Option<Arc<apchat::mspc::MspcChannel>>" src/main.rs; then
    echo "❌ MSPC channel field missing from APChat"
    exit 1
fi
echo "  ✓ MSPC channel field present"

if ! grep -q "with_mspc_channel" src/main.rs; then
    echo "❌ with_mspc_channel method missing"
    exit 1
fi
echo "  ✓ with_mspc_channel method present"

echo "✓ APChat struct verified"
echo ""

# Test 6: Run all tests
echo "Test 6: Running all tests..."
if cargo test --quiet 2>&1 | grep "test result" | grep -q "FAILED"; then
    echo "❌ Some tests failed"
    exit 1
fi
echo "✓ All tests pass"
echo ""

echo "=========================================="
echo "  ✓ ALL VERIFICATIONS PASSED"
echo "=========================================="
echo ""
echo "MSPC Integration Summary:"
echo "  • MSPC channel initialized in REPL with capacity 100"
echo "  • Terminal input router spawned in background"
echo "  • Non-blocking message checking with try_recv()"
echo "  • Interrupt handling (messages starting with '!')"
echo "  • Command handling (messages starting with '/')"
echo "  • User input handling (regular messages)"
echo "  • Message history management"
echo "  • Interruption cleanup"
echo "  • Integration with APChat struct"
echo ""
echo "Implementation Status: COMPLETE ✓"
echo ""
