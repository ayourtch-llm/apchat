# Tool Parameter Validation Module

**Issue ID:** 107

**Status:** RESOLVED

**Created:** 2026-01-25

**Resolved:** 2026-01-25

**Priority:** High

## Description

Create a new parameter validation module in `crates/apchat-toolcore/src/parameter_validation.rs` that validates tool call parameters to ensure:
1. No invalid/extra parameter names are provided
2. All required parameters are present

This prevents runtime errors when tools attempt to use undefined parameters.

## Resolution Summary

Successfully implemented the parameter validation module with full test coverage.

## Implementation Details

- Created `crates/apchat-toolcore/src/parameter_validation.rs` module
- Implemented `validate_tool_call()` function with proper error handling
- Added module exports to `crates/apchat-toolcore/src/lib.rs`
- Created comprehensive test suite with 10 test cases

## Key Changes Made

### Module Structure
- File: `crates/apchat-toolcore/src/parameter_validation.rs`
- Exports: `validate_tool_call()` function
- Dependencies: serde_json, std::collections::HashMap

### Function Signature
```rust
pub fn validate_tool_call(
    tool_call: &ToolCall,
    tool_schema: &ToolParameters
) -> Result<ToolParameters, String>
```

### Validation Logic
1. Parse tool_call.arguments as JSON
2. Extract parameter names from tool_schema
3. Extract required parameter names from tool_schema
4. Check for missing required parameters
5. Check for invalid/extra parameters
6. Return parsed ToolParameters or error message

### Error Messages
Format: `"Tool '{tool_name}' has invalid parameter '{invalid_param}'. Available: {valid_params}. Missing required parameter: {missing_param}"`

### Test Coverage
10 comprehensive tests covering:
- Valid tool calls
- Missing required parameters
- Extra/invalid parameters
- Valid optional parameters
- Mixed case errors
- Real tool schemas (read_file, peek_file_top_10_lines, search_files)
- Edge cases (empty arguments, null values)

## Test Results

All 10 tests passed successfully:
- ✅ test_valid_tool_call
- ✅ test_missing_required_parameter
- ✅ test_extra_invalid_parameter
- ✅ test_valid_optional_parameters
- ✅ test_mixed_case_invalid_and_missing
- ✅ test_peek_file_top_10_lines_tool_schema
- ✅ test_search_files_tool_schema
- ✅ test_empty_arguments
- ✅ test_arguments_with_null_values

## References

- Design document: `docs/plans/2026-01-25-tool-parameter-validation-design.md`
- Commit: Initial implementation of parameter validation module
