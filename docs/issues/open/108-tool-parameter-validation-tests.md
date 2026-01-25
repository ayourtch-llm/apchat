# Tool Parameter Validation Tests

**Issue ID:** 108

**Status:** OPEN

**Created:** 2026-01-25

**Priority:** High

## Description

Write comprehensive tests for the parameter validation module in `crates/apchat-toolcore/tests/parameter_validation_tests.rs`.

## Requirements

- Create test file at `crates/apchat-toolcore/tests/parameter_validation_tests.rs`
- Write tests for all validation scenarios
- Use real tool schemas for testing (open_file, read_file, search_files)
- Ensure all tests pass

## Test Cases

1. **Valid tool call** - All required parameters present, no extra parameters
   - Should return `Ok(tool_params)`

2. **Missing required parameter** - Required parameter not provided
   - Should return `Err` with message about missing parameter

3. **Extra/invalid parameter** - Unknown parameter provided
   - Should return `Err` with message listing valid parameters

4. **Valid optional parameters** - Optional parameters not provided
   - Should return `Ok(tool_params)` (no errors for missing optional params)

5. **Mixed case** - Some invalid, some missing required
   - Should return `Err` with combined information

## Acceptance Criteria

- [ ] Test file created at correct location
- [ ] All 5 test cases implemented
- [ ] Tests use real tool schemas
- [ ] All tests pass
- [ ] Error messages validated

## Implementation Notes

- Test file should be in `crates/apchat-toolcore/tests/`
- Use `open_file`, `read_file`, and `search_files` tool schemas for testing
- Each test should have a descriptive name
- Include assertions for expected results

## References

- Design document: `docs/plans/2026-01-25-tool-parameter-validation-design.md`
- Test strategy section in design document
