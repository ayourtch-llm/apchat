//! Comprehensive tests for parameter validation module
//!
//! This test file validates the parameter validation functionality
//! for tool calls received from LLMs.

use apchat_toolcore::parameter_validation::validate_tool_call;
use apchat_toolcore::tool::{ToolParameters, ParameterDefinition};
use apchat_models::ToolCall;
use serde_json::Value;
use std::collections::HashMap;

/// Helper function to create a mock tool schema
fn create_tool_schema(params: Vec<(String, ParameterDefinition)>) -> ToolParameters {
    let mut data = std::collections::HashMap::new();
    for (name, def) in params {
        data.insert(name, serde_json::to_value(def).unwrap());
    }
    ToolParameters { data }
}

/// Helper function to create a mock tool call with actual parameter values
fn create_tool_call(name: &str, args: &str) -> ToolCall {
    ToolCall {
        id: "call_1".to_string(),
        tool_type: "function".to_string(),
        function: apchat_models::FunctionCall {
            name: name.to_string(),
            arguments: args.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to extract parameter definitions from a ToolParameters object
    fn get_param_definitions(tool_params: &ToolParameters) -> HashMap<String, Value> {
        tool_params.data.clone()
    }

    // ============================================================================
    // VALID SCENARIOS
    // ============================================================================

    #[test]
    fn test_valid_tool_call_with_all_required_params() {
        // Test with open_file tool schema - minimal required parameters only
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"file_path": "test.txt"}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_ok(), "Valid tool call should pass validation");

        let params = result.unwrap();
        assert_eq!(
            params.get_required::<String>("file_path").unwrap(),
            "test.txt"
        );
    }

    #[test]
    fn test_valid_tool_call_with_optional_params() {
        // Test with open_file tool schema - all parameters
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
            ("start_line".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Starting line number".to_string(),
                required: false,
                default: None,
            }),
            ("end_line".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Ending line number".to_string(),
                required: false,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"file_path": "test.txt", "start_line": 10, "end_line": 20}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_ok(), "Valid tool call with optional parameters should pass validation");

        let params = result.unwrap();
        assert_eq!(params.get_required::<String>("file_path").unwrap(), "test.txt");
        assert_eq!(params.get_optional::<i64>("start_line").unwrap().unwrap(), 10);
        assert_eq!(params.get_optional::<i64>("end_line").unwrap().unwrap(), 20);
    }

    #[test]
    fn test_valid_tool_call_with_only_optional_params() {
        // Test with optional parameters when required ones are not provided
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: false,
                default: None,
            }),
            ("start_line".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Starting line number".to_string(),
                required: false,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"start_line": 10}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_ok(), "Valid tool call with only optional parameters should pass validation");

        let params = result.unwrap();
        assert_eq!(params.get_optional::<i64>("start_line").unwrap().unwrap(), 10);
    }

    #[test]
    fn test_tool_call_with_all_parameters() {
        // Test with all parameters provided
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
            ("start_line".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Starting line number".to_string(),
                required: false,
                default: None,
            }),
            ("end_line".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Ending line number".to_string(),
                required: false,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"file_path": "test.txt", "start_line": 5, "end_line": 15}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_ok(), "Valid tool call with all parameters should pass validation");

        let params = result.unwrap();
        assert_eq!(params.get_required::<String>("file_path").unwrap(), "test.txt");
        assert_eq!(params.get_optional::<i64>("start_line").unwrap().unwrap(), 5);
        assert_eq!(params.get_optional::<i64>("end_line").unwrap().unwrap(), 15);
    }

    // ============================================================================
    // MISSING REQUIRED PARAMETERS
    // ============================================================================

    #[test]
    fn test_missing_required_parameter() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
            ("start_line".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Starting line number".to_string(),
                required: false,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"start_line": 10}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Tool call should fail when missing required file_path parameter");
    }

    #[test]
    fn test_missing_all_required_parameters() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
            ("start_line".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Starting line number".to_string(),
                required: false,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Tool call should fail when missing required file_path parameter");

        let error = result.unwrap_err();
        assert!(error.contains("missing required parameter"), "Error should contain 'missing required parameter': {}", error);
    }

    #[test]
    fn test_missing_multiple_required_parameters() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
            ("start_line".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Starting line number".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"start_line": 10}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Tool call should fail when missing required file_path parameter");

        let error = result.unwrap_err();
        assert!(error.contains("missing required parameter"), "Error should contain 'missing required parameter': {}", error);
    }

    // ============================================================================
    // INVALID PARAMETERS
    // ============================================================================

    #[test]
    fn test_invalid_parameter() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"file_path": "test.txt", "invalid_param": "value"}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Tool call with invalid parameter should fail");
    }

    #[test]
    fn test_multiple_invalid_parameters() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
            ("start_line".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Starting line number".to_string(),
                required: false,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"file_path": "test.txt", "invalid1": "value1", "invalid2": "value2"}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Tool call with multiple invalid parameters should fail");
    }

    // ============================================================================
    // EXTRA PARAMETERS
    // ============================================================================

    #[test]
    fn test_extra_parameter() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"file_path": "test.txt", "extra_param": "value"}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Tool call with extra parameter should fail");
    }

    // ============================================================================
    // DATA TYPE VALIDATION
    // ============================================================================

    #[test]
    fn test_wrong_data_type_for_string() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"file_path": 12345}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Tool call with wrong data type should fail");
    }

    #[test]
    fn test_wrong_data_type_for_integer() {
        let schema = create_tool_schema(vec![
            ("start_line".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Starting line number".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"start_line": "not_a_number"}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Tool call with wrong data type should fail");
    }

    #[test]
    fn test_wrong_data_type_for_boolean() {
        let schema = create_tool_schema(vec![
            ("append".to_string(), ParameterDefinition {
                param_type: "boolean".to_string(),
                description: "Append to file".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"append": "not_boolean"}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Tool call with wrong data type should fail");
    }

    // ============================================================================
    // JSON VALIDATION
    // ============================================================================

    #[test]
    fn test_invalid_json() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"file_path": "test.txt", invalid_json: "value"}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Tool call with invalid JSON should fail");
    }

    #[test]
    fn test_empty_json() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Tool call with empty JSON should fail");
    }

    // ============================================================================
    // EDGE CASES
    // ============================================================================

    #[test]
    fn test_whitespace_in_parameter_names() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"file_path": "test.txt", "  extra  ": "value"}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Tool call with whitespace in parameter name should fail");
    }

    #[test]
    fn test_null_values() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"file_path": null}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Tool call with null value should fail validation");
    }

    #[test]
    fn test_null_string_parameter() {
        let schema = create_tool_schema(vec![
            ("name".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Name".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("test", r#"{"name": null}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Null should be rejected for string parameters");
    }

    #[test]
    fn test_null_integer_parameter() {
        let schema = create_tool_schema(vec![
            ("count".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Count".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("test", r#"{"count": null}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Null should be rejected for integer parameters");
    }

    #[test]
    fn test_null_number_parameter() {
        let schema = create_tool_schema(vec![
            ("value".to_string(), ParameterDefinition {
                param_type: "number".to_string(),
                description: "Value".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("test", r#"{"value": null}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Null should be rejected for number parameters");
    }

    #[test]
    fn test_null_boolean_parameter() {
        let schema = create_tool_schema(vec![
            ("flag".to_string(), ParameterDefinition {
                param_type: "boolean".to_string(),
                description: "Flag".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("test", r#"{"flag": null}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Null should be rejected for boolean parameters");
    }

    #[test]
    fn test_null_array_parameter() {
        let schema = create_tool_schema(vec![
            ("items".to_string(), ParameterDefinition {
                param_type: "array".to_string(),
                description: "Items".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("test", r#"{"items": null}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Null should be rejected for array parameters");
    }

    #[test]
    fn test_null_object_parameter() {
        let schema = create_tool_schema(vec![
            ("config".to_string(), ParameterDefinition {
                param_type: "object".to_string(),
                description: "Config".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("test", r#"{"config": null}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Null should be rejected for object parameters");
    }

    #[test]
    fn test_mixed_null_and_valid_parameters() {
        let schema = create_tool_schema(vec![
            ("name".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Name".to_string(),
                required: true,
                default: None,
            }),
            ("count".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Count".to_string(),
                required: true,
                default: None,
            }),
            ("optional".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Optional".to_string(),
                required: false,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("test", r#"{"name": null, "count": null, "optional": null}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "All null values should fail validation");
    }

    #[test]
    fn test_null_optional_parameter() {
        let schema = create_tool_schema(vec![
            ("required_name".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Required name".to_string(),
                required: true,
                default: None,
            }),
            ("optional_description".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Optional description".to_string(),
                required: false,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("test", r#"{"required_name": null, "optional_description": null}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "Null values should be rejected for optional parameters");
    }

    #[test]
    fn test_all_null_values_schema() {
        let schema = create_tool_schema(vec![
            ("param1".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "First param".to_string(),
                required: true,
                default: None,
            }),
            ("param2".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Second param".to_string(),
                required: true,
                default: None,
            }),
            ("param3".to_string(), ParameterDefinition {
                param_type: "boolean".to_string(),
                description: "Third param".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("test", r#"{"param1": null, "param2": null, "param3": null}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_err(), "All null values should be rejected for valid parameter types");
    }

    #[test]
    fn test_empty_string_values() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"file_path": ""}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_ok(), "Tool call with empty string value should pass validation");
    }

    #[test]
    fn test_zero_values() {
        let schema = create_tool_schema(vec![
            ("start_line".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Starting line number".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"start_line": 0}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_ok(), "Tool call with zero value should pass validation");
    }

    #[test]
    fn test_negative_integer_values() {
        let schema = create_tool_schema(vec![
            ("start_line".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Starting line number".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"start_line": -10}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_ok(), "Tool call with negative integer value should pass validation");
    }

    #[test]
    fn test_large_integer_values() {
        let schema = create_tool_schema(vec![
            ("start_line".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Starting line number".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"start_line": 999999999}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_ok(), "Tool call with large integer value should pass validation");
    }

    // ============================================================================
    // ARRAY PARAMETERS
    // ============================================================================

    #[test]
    fn test_array_parameter() {
        let schema = create_tool_schema(vec![
            ("files".to_string(), ParameterDefinition {
                param_type: "array".to_string(),
                description: "List of files".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"files": ["file1.txt", "file2.txt"]}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_ok(), "Tool call with array parameter should pass validation");
    }

    #[test]
    fn test_nested_array_parameter() {
        let schema = create_tool_schema(vec![
            ("items".to_string(), ParameterDefinition {
                param_type: "array".to_string(),
                description: "List of items".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"items": [{"name": "item1"}, {"name": "item2"}]}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_ok(), "Tool call with nested array parameter should pass validation");
    }

    // ============================================================================
    // OBJECT PARAMETERS
    // ============================================================================

    #[test]
    fn test_object_parameter() {
        let schema = create_tool_schema(vec![
            ("options".to_string(), ParameterDefinition {
                param_type: "object".to_string(),
                description: "Options object".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"options": {"key1": "value1", "key2": "value2"}}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_ok(), "Tool call with object parameter should pass validation");
    }

    #[test]
    fn test_array_of_objects_parameter() {
        let schema = create_tool_schema(vec![
            ("items".to_string(), ParameterDefinition {
                param_type: "array".to_string(),
                description: "List of items with nested objects".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"items": [{"name": "item1", "details": {"key": "value"}}, {"name": "item2"}]}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_ok(), "Tool call with array of objects parameter should pass validation");
    }

    #[test]
    fn test_empty_array_parameter() {
        let schema = create_tool_schema(vec![
            ("files".to_string(), ParameterDefinition {
                param_type: "array".to_string(),
                description: "List of files".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"files": []}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_ok(), "Tool call with empty array parameter should pass validation");
    }

    #[test]
    fn test_empty_object_parameter() {
        let schema = create_tool_schema(vec![
            ("options".to_string(), ParameterDefinition {
                param_type: "object".to_string(),
                description: "Options object".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("open_file", r#"{"options": {}}"#);

        let definitions = get_param_definitions(&schema);
        let result = validate_tool_call(&tool_call, &schema, &definitions);
        assert!(result.is_ok(), "Tool call with empty object parameter should pass validation");
    }
}
