# Tool Parameter Validation Module

**Issue ID:** 107

**Status:** OPEN

**Created:** 2026-01-25

**Priority:** High

## Description

Create a new parameter validation module in `crates/apchat-toolcore/src/parameter_validation.rs` that validates tool call parameters to ensure:
1. No invalid/extra parameter names are provided
2. All required parameters are present

This prevents runtime errors when tools attempt to use undefined parameters.

## Requirements

- Create standalone module at `crates/apchat-toolcore/src/parameter_validation.rs`
- Implement `validate_tool_call()` function with proper error handling
- Follow the implementation specification in the design document
- Support validation of parameter names only (no type/value validation)
- Provide clear error messages with list of valid parameters

## Acceptance Criteria

- [ ] Module file created at correct location
- [ ] `validate_tool_call()` function implemented
- [ ] Function validates parameter existence correctly
- [ ] Function validates required parameters correctly
- [ ] Error messages include list of valid parameters
- [ ] Function returns `Result<ToolParameters, String>`
- [ ] No type or value validation performed
- [ ] Code follows Rust best practices

## Implementation Notes

- Function signature: `pub fn validate_tool_call(tool_call: &ToolCall, tool_schema: &ToolParameters) -> Result<ToolParameters, String>`
- Validation happens after JSON parsing but before tool execution
- Error messages should follow format: `"Tool '{tool_name}' has invalid parameter '{invalid_param}'. Available: {valid_params}. Missing required parameter: {missing_param}"`

## References

- Design document: `docs/plans/2026-01-25-tool-parameter-validation-design.md`
