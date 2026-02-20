//! Parameter validation for tool calls
//!
//! This module validates that LLM-supplied tool calls have no invalid parameter names
//! and all required parameters are present before tool execution.

use serde_json::Value;
use std::collections::HashMap;
use crate::tool::{ToolParameters, ParameterDefinition};
use apchat_models::{ToolCall, FunctionCall};

/// Validates a tool call's parameters against the tool schema
///
/// # Arguments
/// * `tool_call` - The tool call received from LLM
/// * `tool_schema` - The tool's parameter schema from ToolRegistry
/// * `param_definitions` - The parameter definitions from tool.parameters() as JSON values
///
/// # Returns
/// * `Ok(tool_params)` - Validation passed, return parsed parameters ready for execution
/// * `Err(error_msg)` - Validation failed, return human-readable error string
///
/// # Error Format
/// Error messages follow the format:
/// "Tool '{tool_name}' has invalid parameter '{invalid_param}'. Available: {valid_params}. Missing required parameter: {missing_param}"
pub fn validate_tool_call(
    tool_call: &ToolCall,
    tool_schema: &ToolParameters,
    param_definitions: &HashMap<String, Value>
) -> Result<ToolParameters, String> {
    // Parse the arguments JSON
    let parsed_args: HashMap<String, Value> = serde_json::from_str(&tool_call.function.arguments)
        .map_err(|e| format!("Failed to parse tool arguments: {}", e))?;

    // Extract parameter names from parameter definitions
    let valid_params: Vec<String> = param_definitions.keys().cloned().collect();

    // Extract required parameter names from parameter definitions by deserializing
    let param_definitions: HashMap<String, ParameterDefinition> = param_definitions
        .iter()
        .map(|(name, value)| {
            let def: ParameterDefinition = serde_json::from_value(value.clone())
                .map_err(|e| format!("Failed to deserialize parameter definition for '{}': {}", name, e))
                .unwrap();
            (name.clone(), def)
        })
        .collect();

    let required_params: Vec<String> = param_definitions
        .iter()
        .filter(|(_, def)| def.required)
        .map(|(name, _)| name.clone())
        .collect();

    // Check for invalid/extra parameters first
    for param_name in parsed_args.keys() {
        if !valid_params.contains(param_name) {
            let valid_params_str = valid_params.join(", ");
            return Err(format!(
                "Tool '{}' has invalid parameter '{}'. Available: {}",
                tool_call.function.name, param_name, valid_params_str
            ));
        }
    }

    // Check for type mismatches
    for (param_name, param_value) in &parsed_args {
        if let Some(param_def) = param_definitions.get(param_name) {
            match validate_type(param_value, &param_def.param_type) {
                Ok(_) => {},
                Err(msg) => {
                    return Err(format!(
                        "Tool '{}' has parameter '{}' with wrong type: {}. Expected: {}, Received: {}",
                        tool_call.function.name, param_name, msg, param_def.param_type, param_value
                    ));
                }
            }
        }
    }

    // Check for missing required parameters
    for required_param in &required_params {
        if !parsed_args.contains_key(required_param) {
            let valid_params_str = valid_params.join(", ");
            return Err(format!(
                "Tool '{}' has missing required parameter '{}'. Available: {}",
                tool_call.function.name, required_param, valid_params_str
            ));
        }
    }

    // All checks passed, return the parsed parameters
    Ok(ToolParameters {
        data: parsed_args,
    })
}

/// Validates that a JSON value matches the expected type
fn validate_type(value: &Value, expected_type: &str) -> Result<(), String> {
    match expected_type {
        "string" => match value {
            Value::String(_) => Ok(()),
            Value::Null => Err("Parameter cannot be null".to_string()),
            _ => Err(format!("Parameter has wrong type. Expected: string, Received: {}", format!("{:?}", value))),
        },
        "integer" => match value {
            Value::Number(n) if n.is_i64() => Ok(()),
            Value::Null => Err("Parameter cannot be null".to_string()),
            _ => Err(format!("Parameter has wrong type. Expected: integer, Received: {}", format!("{:?}", value))),
        },
        "number" => match value {
            Value::Number(_) => Ok(()),
            Value::Null => Err("Parameter cannot be null".to_string()),
            _ => Err(format!("Parameter has wrong type. Expected: number, Received: {}", format!("{:?}", value))),
        },
        "boolean" => match value {
            Value::Bool(_) => Ok(()),
            Value::Null => Err("Parameter cannot be null".to_string()),
            _ => Err(format!("Parameter has wrong type. Expected: boolean, Received: {}", format!("{:?}", value))),
        },
        "array" => match value {
            Value::Array(_) => Ok(()),
            Value::Null => Err("Parameter cannot be null".to_string()),
            _ => Err(format!("Parameter has wrong type. Expected: array, Received: {}", format!("{:?}", value))),
        },
        "object" => match value {
            Value::Object(_) => Ok(()),
            Value::Null => Err("Parameter cannot be null".to_string()),
            _ => Err(format!("Parameter has wrong type. Expected: object, Received: {}", format!("{:?}", value))),
        },
        _ => Ok(()), // Unknown type, accept the value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::ToolParameters;

    // Helper function to create a mock tool schema
    fn create_tool_schema(params: Vec<(String, ParameterDefinition)>) -> ToolParameters {
        let mut data = HashMap::new();
        for (name, def) in params {
            data.insert(name, serde_json::to_value(def).unwrap());
        }
        ToolParameters { data }
    }

    // Helper function to create a mock tool call
    fn create_tool_call(name: &str, args: &str) -> ToolCall {
        ToolCall {
            id: "call_1".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: name.to_string(),
                arguments: args.to_string(),
            },
        }
    }

    #[test]
    fn test_valid_tool_call() {
        // Test with read_file tool schema
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

        let tool_call = create_tool_call("read_file", r#"{"file_path": "test.txt"}"#);

        let result = validate_tool_call(&tool_call, &schema, &schema.data);
        assert!(result.is_ok());

        let params = result.unwrap();
        assert_eq!(params.get_required::<String>("file_path").unwrap(), "test.txt");
    }

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

        let tool_call = create_tool_call("read_file", r#"{"start_line": 10}"#);

        let result = validate_tool_call(&tool_call, &schema, &schema.data);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.contains("missing required parameter"));
        assert!(error.contains("file_path"));
    }

    #[test]
    fn test_extra_invalid_parameter() {
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

        let tool_call = create_tool_call("read_file", r#"{"file_path": "test.txt", "invalid_param": "value"}"#);

        let result = validate_tool_call(&tool_call, &schema, &schema.data);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.contains("invalid parameter"));
        assert!(error.contains("invalid_param"));
    }

    #[test]
    fn test_valid_optional_parameters() {
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

        // Test with no optional parameters
        let tool_call = create_tool_call("read_file", r#"{"file_path": "test.txt"}"#);
        let result = validate_tool_call(&tool_call, &schema, &schema.data);
        assert!(result.is_ok());

        // Test with optional parameters
        let tool_call = create_tool_call("read_file", r#"{"file_path": "test.txt", "start_line": 10}"#);
        let result = validate_tool_call(&tool_call, &schema, &schema.data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mixed_case_invalid_and_missing() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
            ("content".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "File content".to_string(),
                required: true,
                default: None,
            }),
        ]);

        // Missing required parameter 'content' and has invalid parameter 'wrong_param'
        let tool_call = create_tool_call("write_file", r#"{"file_path": "test.txt", "wrong_param": "value"}"#);

        let result = validate_tool_call(&tool_call, &schema, &schema.data);
        assert!(result.is_err());

        let error = result.unwrap_err();
        eprintln!("Error message: {}", error);

        // Should mention invalid parameter (checked first)
        assert!(error.contains("invalid parameter"), "Error should contain 'invalid parameter': {}", error);
        assert!(error.contains("wrong_param"), "Error should contain 'wrong_param': {}", error);
        // Note: missing required parameter 'content' is also present but not reported in this case
        // since we return the first error encountered
    }

    #[test]
    fn test_peek_file_top_10_lines_tool_schema() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
            ("limit".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Maximum number of lines to read".to_string(),
                required: false,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("peek_file_top_10_lines", r#"{"file_path": "test.txt", "limit": 100}"#);

        let result = validate_tool_call(&tool_call, &schema, &schema.data);
        assert!(result.is_ok());

        let params = result.unwrap();
        assert_eq!(params.get_required::<String>("file_path").unwrap(), "test.txt");
        assert_eq!(params.get_optional::<i64>("limit").unwrap().unwrap(), 100);
    }

    #[test]
    fn test_search_files_tool_schema() {
        let schema = create_tool_schema(vec![
            ("pattern".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Search pattern".to_string(),
                required: true,
                default: None,
            }),
            ("max_results".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Maximum number of results".to_string(),
                required: false,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("search_files", r#"{"pattern": "*.rs"}"#);

        let result = validate_tool_call(&tool_call, &schema, &schema.data);
        assert!(result.is_ok());

        let params = result.unwrap();
        assert_eq!(params.get_required::<String>("pattern").unwrap(), "*.rs");
    }

    #[test]
    fn test_empty_arguments() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("read_file", r#"{}"#);

        let result = validate_tool_call(&tool_call, &schema, &schema.data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing required parameter"));
    }

    #[test]
    fn test_arguments_with_null_values() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("read_file", r#"{"file_path": null}"#);

        let result = validate_tool_call(&tool_call, &schema, &schema.data);
        assert!(result.is_err());
    }
}
