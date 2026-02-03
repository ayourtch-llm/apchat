# Tool Parameter Validation Design

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add validation to verify LLM-supplied tool calls have no invalid parameter names and all required parameters are present.

**Architecture:** Create a standalone validation module in `crates/apchat-toolcore/src/parameter_validation.rs` that validates parameter names after JSON parsing. Validation only checks parameter names, not types or values. Error messages include list of valid parameters to guide LLM corrections.

**Tech Stack:** Rust, serde_json, crates/apchat-toolcore

---

## Problem Statement

Currently, the system validates that tool call JSON is well-formed but does not verify that:
1. No invalid/extra parameter names are provided
2. All required parameters are present

This can lead to runtime errors when tools attempt to use undefined parameters.

---

## Design Overview

### Validation Flow

```
LLM → ToolCall → existing JSON validation → parameter name validation → execute tool
```

The validation happens **after** existing JSON structure validation but **before** tool execution.

### Validation Rules

1. **Tool existence** - Tool name must match a registered tool (already validated)
2. **Parameter existence** - All provided parameters must be valid (in tool schema)
3. **Required parameters** - All required parameters must be present
4. **No type/value validation** - Only check parameter names, not types or values
5. **No optional parameter validation** - Optional parameters not provided don't cause errors

### Error Behavior

- **Silent fail to LLM** - Log validation failures for debugging, return error to LLM
- **Human-readable errors** - Clear messages with list of valid parameters
- **Retry mechanism** - LLM will receive error and retry with corrected parameters

---

## Implementation Specification

### Module Structure

```
crates/apchat-toolcore/
├── src/
│   ├── parameter_validation.rs  [NEW]
│   └── ...
└── tests/
    └── parameter_validation_tests.rs  [NEW]
```

### Function Signature

```rust
pub fn validate_tool_call(
    tool_call: &ToolCall,
    tool_schema: &ToolParameters
) -> Result<ToolParameters, String>
```

**Parameters:**
- `tool_call` - The tool call received from LLM
- `tool_schema` - The tool's parameter schema from ToolRegistry

**Returns:**
- `Ok(tool_params)` - Validation passed, return parsed parameters ready for execution
- `Err(error_msg)` - Validation failed, return human-readable error string

### Error Message Format

```
"Tool '{tool_name}' has invalid parameter '{invalid_param}'. Available: {valid_params}. Missing required parameter: {missing_param}"
```

**Example:**
```
"Tool 'open_file' has invalid parameter 'invalid_param'. Available: start_line, end_line, file_path, max_line_count. Missing required parameter: file_path"
```

### Validation Algorithm

```
1. Parse tool_call.arguments as JSON to create a HashMap<String, Value>
2. Extract parameter names from tool_schema (all defined parameters)
3. Extract required parameter names from tool_schema (required: true)
4. Check for extra/invalid parameters:
   - For each parsed parameter name:
     - If not in tool_schema → error: "Invalid parameter name"
5. Check for missing required parameters:
   - For each required parameter name:
     - If not in parsed parameters → error: "Missing required parameter"
6. If all checks pass → return parsed ToolParameters
```

### Integration Points

**Where to add validation:**
- Create new module: `crates/apchat-toolcore/src/parameter_validation.rs`
- Add function: `validate_tool_call(tool_call, tool_schema) -> Result<ToolParameters, String>`
- Call this function in the execution layer (after existing JSON validation)

**Execution flow:**
```
ToolCall → JSON validation (existing) → parameter name validation (new) → execute tool
```

**Scope:**
- Apply validation only in **single LLM mode**
- Skip validation in multi-agent mode (different execution path)

---

## Testing Strategy

### Test File Location
`crates/apchat-toolcore/tests/parameter_validation_tests.rs`

### Test Cases

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

### Test Tools to Use

Use real tool schemas for testing:
- `open_file` tool (parameters: file_path, start_line, end_line, max_line_count)
- `peek_file_top_10_lines` tool (parameters: file_path, max_line_count)
- `search_files` tool (parameters: pattern, max_results)

---

## Implementation Steps (High-Level)

1. Create `crates/apchat-toolcore/src/parameter_validation.rs` module
2. Implement `validate_tool_call()` function
3. Write comprehensive test suite in `parameter_validation_tests.rs`
4. Integrate validation into single LLM mode execution path
5. Verify error messages guide LLM correctly

---

## Design Rationale

### Why validate after JSON parsing?
- JSON validation already ensures structure is correct
- Parameter validation is a logical next step
- Keeps validation focused and maintainable

### Why only parameter names?
- Simpler, faster, and more maintainable
- Type and value validation can come later as needed
- LLMs already know parameter schemas, so name validation is sufficient

### Why silent error to LLM with debug logging?
- Gives LLM clear error message to correct itself
- Maintains conversation flow
- Debug logging helps diagnose issues

### Why only single LLM mode?
- Multi-agent mode has different validation needs
- Keeps initial implementation focused and simple
- Can extend to multi-agent mode later if needed

---

## Success Criteria

✓ No invalid parameter names cause tool execution errors
✓ Missing required parameters are caught before execution
✓ Error messages clearly indicate what's wrong and what's valid
✓ LLM can use error messages to correct tool calls
✓ All validation tests pass
✓ No impact on multi-agent mode execution
