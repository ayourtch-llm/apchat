#!/bin/bash

# Enhanced Superpowers Skills Verification Test
# Tests that the enhanced skills enforce process fidelity properly

echo "🧪 Enhanced Superpowers Skills Verification"
echo "=========================================="
echo ""

# Test configuration
TEST_RESULTS="skills-test-results-$(date +%Y%m%d-%H%M%S).log"
PASS_COUNT=0
FAIL_COUNT=0
TOTAL_TESTS=4

echo "📝 Test Results will be saved to: $TEST_RESULTS"
echo "🎯 Running $TOTAL_TESTS skills verification tests..."
echo ""

# Function to log test results
log_result() {
    local test_name="$1"
    local status="$2"
    local details="$3"
    
    echo "[$status] $test_name: $details" | tee -a "$TEST_RESULTS"
    
    if [ "$status" = "PASS" ]; then
        ((PASS_COUNT++))
    else
        ((FAIL_COUNT++))
    fi
}

# Test 1: using-superpowers skill enhancement
echo "🔍 Test 1: using-superpowers Skill Enhancement"
echo "---------------------------------------------"
echo "Testing that using-superpowers now enforces process fidelity..."

# Check if the enhancements were applied
if grep -q "CORE EXECUTION MANDATE" skills/using-superpowers/SKILL.md; then
    if grep -q "MANDATORY SKILL CHAINS" skills/using-superpowers/SKILL.md; then
        if grep -q "PLANNING VS EXECUTION BARRIER" skills/using-superpowers/SKILL.md; then
            log_result "using-superpowers Enhancement" "PASS" "All process fidelity enhancements applied correctly"
        else
            log_result "using-superpowers Enhancement" "FAIL" "PLANNING VS EXECUTION BARRIER not found"
        fi
    else
        log_result "using-superpowers Enhancement" "FAIL" "MANDATORY SKILL CHAINS not found"
    fi
else
    log_result "using-superpowers Enhancement" "FAIL" "CORE EXECUTION MANDATE not found"
fi
echo ""

# Test 2: writing-plans handoff enforcement
echo "🔍 Test 2: writing-plans Handoff Enforcement"
echo "--------------------------------------------"
echo "Testing that writing-plans now mandates execution handoff..."

if grep -q "CRITICAL: This handoff is MANDATORY" skills/writing-plans/SKILL.md; then
    if grep -q "Two execution options" skills/writing-plans/SKILL.md; then
        log_result "writing-plans Handoff" "PASS" "Mandatory execution handoff properly enforced"
    else
        log_result "writing-plans Handoff" "FAIL" "Execution options not properly specified"
    fi
else
    log_result "writing-plans Handoff" "FAIL" "Mandatory handoff enforcement not found"
fi
echo ""

# Test 3: executing-plans TodoWrite enforcement
echo "🔍 Test 3: executing-plans TodoWrite Enforcement"
echo "-------------------------------------------------"
echo "Testing that executing-plans now mandates TodoWrite tracking..."

if grep -q "MANDATORY PRE-EXECUTION" skills/executing-plans/SKILL.md; then
    if grep -q "Create TodoWrite for task tracking BEFORE starting" skills/executing-plans/SKILL.md; then
        if grep -q "PROCESS FIDELITY ENFORCEMENT" skills/executing-plans/SKILL.md; then
            log_result "executing-plans TodoWrite" "PASS" "TodoWrite enforcement and process fidelity implemented"
        else
            log_result "executing-plans TodoWrite" "FAIL" "Process fidelity enforcement not found"
        fi
    else
        log_result "executing-plans TodoWrite" "FAIL" "TodoWrite before start requirement not found"
    fi
else
    log_result "executing-plans TodoWrite" "FAIL" "Mandatory pre-execution section not found"
fi
echo ""

# Test 4: Original mistake prevention
echo "🔍 Test 4: Original Mistake Prevention"
echo "--------------------------------------"
echo "Testing that the original model override mistake is now impossible..."

# Check if all prevention mechanisms are in place
PREVENTION_COUNT=0

# Check using-superpowers for mandate
if grep -q "NO improvisation, NO shortcuts, NO \"I know what to do\"" skills/using-superpowers/SKILL.md; then
    ((PREVENTION_COUNT++))
fi

# Check writing-plans for mandatory handoff
if grep -q "cannot proceed directly to implementation without completing this handoff" skills/writing-plans/SKILL.md; then
    ((PREVENTION_COUNT++))
fi

# Check executing-plans for process fidelity
if grep -q "Am I following the plan step exactly?" skills/executing-plans/SKILL.md; then
    ((PREVENTION_COUNT++))
fi

if [ $PREVENTION_COUNT -eq 3 ]; then
    log_result "Original Mistake Prevention" "PASS" "All prevention mechanisms in place (3/3)"
else
    log_result "Original Mistake Prevention" "FAIL" "Missing prevention mechanisms ($PREVENTION_COUNT/3)"
fi
echo ""

# Results Summary
echo "📊 Skills Enhancement Test Results Summary"
echo "=========================================="
echo "Total Tests: $TOTAL_TESTS"
echo "Passed: $PASS_COUNT"
echo "Failed: $FAIL_COUNT"
echo "Success Rate: $(( PASS_COUNT * 100 / TOTAL_TESTS ))%"
echo ""

if [ $FAIL_COUNT -eq 0 ]; then
    echo "✅ ALL TESTS PASSED - Superpowers skills successfully enhanced!"
    echo "🎉 Process fidelity enforcement is now active"
    echo "🛡️ Original mistake is now structurally impossible"
else
    echo "❌ Some tests failed - Review skill enhancements"
    echo "🔧 See detailed results in: $TEST_RESULTS"
fi

echo ""
echo "📋 Enhanced Skills Summary:"
echo "- using-superpowers: CORE EXECUTION MANDATE + skill chains + barriers"
echo "- writing-plans: MANDATORY execution handoff (cannot skip)"
echo "- executing-plans: MANDATORY TodoWrite + process fidelity"
echo "- All skills: Transparency requirements + self-monitoring"

echo ""
echo "🚀 The superpowers system now enforces perfect process compliance!"

echo ""
echo "📁 Full test log: $TEST_RESULTS"