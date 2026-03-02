//! Parameter validation for tool calls
//!
//! This module validates that LLM-supplied tool calls have no invalid parameter names
//! and all required parameters are present before tool execution.

use serde_json::Value;
use std::collections::HashMap;
use crate::tool::{ToolParameters, ParameterDefinition};
use crate::sql_logger;
use apchat_models::{ToolCall, FunctionCall};

/// Validates a tool call's parameters against the tool schema
///
/// # Arguments
/// * `tool_call` - The tool call received from LLM
/// * `tool_schema` - The tool's parameter schema from ToolRegistry
/// * `param_definitions` - The parameter definitions from tool.parameters() as JSON values
/// * `raw_llm_output` - Optional raw LLM output for debugging
///
/// # Returns
/// * `Ok(tool_params)` - Validation passed, return parsed parameters ready for execution
/// * `Err(error_msg)` - Validation failed, return human-readable error string
///
/// # Error Format
/// Error messages follow the format:
/// "Tool '{tool_name}' has invalid parameter '{invalid_param}'. Available: {valid_params}. Missing required parameter: {missing_param}"
pub async fn validate_tool_call_with_logging(
    tool_call: &ToolCall,
    tool_schema: &ToolParameters,
    param_definitions: &HashMap<String, Value>,
    raw_llm_output: Option<String>,
) -> Result<ToolParameters, String> {
    let tool_name = &tool_call.function.name;
    let args_str = &tool_call.function.arguments;
    
    // Try to parse the arguments JSON
    let parsed_args_result: Result<HashMap<String, Value>, _> = serde_json::from_str(args_str);
    
    match parsed_args_result {
        Ok(mut parsed_args) => {
            // Continue with validation...
            let result = validate_tool_call_internal(tool_call, tool_schema, param_definitions, parsed_args);
            
            // Log the result
            let (success, error) = match &result {
                Ok(_) => (true, None),
                Err(e) => (false, Some(e.clone())),
            };
            
            sql_logger::log_tool_parse(
                None,
                Some(tool_name.clone()),
                None,
                None,
                Some(args_str.to_string()),  // Always log raw args
                error.clone(),
                success,
                None,
                None,
                raw_llm_output.clone(),
            )
            .await
            .ok();
            
            result
        }
        Err(e) => {
            // Log the parse error - this ensures we capture even if JSON parsing fails
            let error_msg = format!("Failed to parse tool arguments: {}", e);
            
            sql_logger::log_tool_parse(
                None,
                Some(tool_name.clone()),
                None,
                None,
                Some(args_str.to_string()),  // Log the raw args that failed to parse
                Some(error_msg.clone()),
                false,
                None,
                None,
                raw_llm_output.clone(),
            )
            .await
            .ok();
            
            Err(error_msg)
        }
    }
}

/// Internal validation logic (extracted for reuse)
fn validate_tool_call_internal(
    tool_call: &ToolCall,
    _tool_schema: &ToolParameters,
    param_definitions: &HashMap<String, Value>,
    parsed_args: HashMap<String, Value>,
) -> Result<ToolParameters, String> {
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

    // Coerce values to match expected types where possible (e.g., float 320.0 -> integer 320)
    let mut parsed_args = parsed_args;
    for (param_name, param_value) in parsed_args.iter_mut() {
        if let Some(param_def) = param_definitions.get(param_name) {
            if let Some(coerced) = coerce_value(param_value, &param_def.param_type) {
                *param_value = coerced;
            }
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

/// Original validate_tool_call function (kept for backward compatibility)
pub fn validate_tool_call(
    tool_call: &ToolCall,
    tool_schema: &ToolParameters,
    param_definitions: &HashMap<String, Value>
) -> Result<ToolParameters, String> {
    // Parse the arguments JSON
    let parsed_args: HashMap<String, Value> = serde_json::from_str(&tool_call.function.arguments)
        .map_err(|e| format!("Failed to parse tool arguments: {}", e))?;
    
    validate_tool_call_internal(tool_call, tool_schema, param_definitions, parsed_args)
}

/// Attempts to coerce a JSON value to match the expected parameter type.
/// Returns Some(coerced_value) if coercion is possible, None otherwise.
///
/// This handles common LLM quirks like supplying float values (320.0) or
/// string values ("320.0", "50") where integers are expected.
fn coerce_value(value: &Value, expected_type: &str) -> Option<Value> {
    match expected_type {
        "integer" => {
            match value {
                Value::Number(n) if n.is_f64() && !n.is_i64() => {
                    let f = n.as_f64()?;
                    f64_to_i64_value(f)
                }
                Value::String(s) => {
                    if let Ok(i) = s.parse::<i64>() {
                        return Some(Value::Number(serde_json::Number::from(i)));
                    }
                    if let Ok(f) = s.parse::<f64>() {
                        return f64_to_i64_value(f);
                    }
                    None
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Converts an f64 to a JSON integer Value if it has no fractional part and is in i64 range.
fn f64_to_i64_value(f: f64) -> Option<Value> {
    if f.fract() == 0.0 && f >= i64::MIN as f64 && f <= i64::MAX as f64 {
        Some(Value::Number(serde_json::Number::from(f as i64)))
    } else {
        None
    }
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
            ("offset".to_string(), ParameterDefinition {
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
            ("offset".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Starting line number".to_string(),
                required: false,
                default: None,
            }),
        ]);

        let tool_call = create_tool_call("read_file", r#"{"offset": 10}"#);

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
            ("offset".to_string(), ParameterDefinition {
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
            ("offset".to_string(), ParameterDefinition {
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
        let tool_call = create_tool_call("read_file", r#"{"file_path": "test.txt", "offset": 10}"#);
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

    #[test]
    fn test_float_to_integer_coercion() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
            ("offset".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Starting line number".to_string(),
                required: false,
                default: None,
            }),
            ("limit".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Max lines".to_string(),
                required: false,
                default: None,
            }),
        ]);

        // Float values like 320.0 should be coerced to integers
        let tool_call = create_tool_call("read_file", r#"{"file_path": "test.txt", "offset": 320.0, "limit": 50.0}"#);
        let result = validate_tool_call(&tool_call, &schema, &schema.data);
        assert!(result.is_ok(), "Float 320.0 should be coerced to integer: {:?}", result);

        let params = result.unwrap();
        assert_eq!(params.get_required::<i64>("offset").unwrap(), 320);
        assert_eq!(params.get_required::<i64>("limit").unwrap(), 50);
    }

    #[test]
    fn test_string_integer_to_integer_coercion() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
            ("offset".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Starting line number".to_string(),
                required: false,
                default: None,
            }),
        ]);

        // String "320" should be coerced to integer
        let tool_call = create_tool_call("read_file", r#"{"file_path": "test.txt", "offset": "320"}"#);
        let result = validate_tool_call(&tool_call, &schema, &schema.data);
        assert!(result.is_ok(), "String '320' should be coerced to integer: {:?}", result);

        let params = result.unwrap();
        assert_eq!(params.get_required::<i64>("offset").unwrap(), 320);
    }

    #[test]
    fn test_string_float_to_integer_coercion() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
            ("offset".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Starting line number".to_string(),
                required: false,
                default: None,
            }),
            ("limit".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Max lines".to_string(),
                required: false,
                default: None,
            }),
        ]);

        // String "320.0" should be coerced to integer (matches the exact error from the issue)
        let tool_call = create_tool_call("read_file", r#"{"file_path": "test.txt", "offset": "320.0", "limit": "50.0"}"#);
        let result = validate_tool_call(&tool_call, &schema, &schema.data);
        assert!(result.is_ok(), "String '320.0' should be coerced to integer: {:?}", result);

        let params = result.unwrap();
        assert_eq!(params.get_required::<i64>("offset").unwrap(), 320);
        assert_eq!(params.get_required::<i64>("limit").unwrap(), 50);
    }

    #[test]
    fn test_fractional_float_rejected() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
            ("offset".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Starting line number".to_string(),
                required: false,
                default: None,
            }),
        ]);

        // Float with fractional part should be rejected
        let tool_call = create_tool_call("read_file", r#"{"file_path": "test.txt", "offset": 320.5}"#);
        let result = validate_tool_call(&tool_call, &schema, &schema.data);
        assert!(result.is_err(), "Float 320.5 should not be coerced to integer");
    }

    #[test]
    fn test_non_numeric_string_rejected() {
        let schema = create_tool_schema(vec![
            ("file_path".to_string(), ParameterDefinition {
                param_type: "string".to_string(),
                description: "Path to the file".to_string(),
                required: true,
                default: None,
            }),
            ("offset".to_string(), ParameterDefinition {
                param_type: "integer".to_string(),
                description: "Starting line number".to_string(),
                required: false,
                default: None,
            }),
        ]);

        // Non-numeric string should be rejected
        let tool_call = create_tool_call("read_file", r#"{"file_path": "test.txt", "offset": "abc"}"#);
        let result = validate_tool_call(&tool_call, &schema, &schema.data);
        assert!(result.is_err(), "String 'abc' should not be coerced to integer");
    }

    #[test]
    fn test_coerce_value_function() {
        // Float without fraction -> integer
        let v = serde_json::json!(320.0);
        let coerced = coerce_value(&v, "integer");
        assert_eq!(coerced, Some(serde_json::json!(320)));

        // String integer -> integer
        let v = serde_json::json!("50");
        let coerced = coerce_value(&v, "integer");
        assert_eq!(coerced, Some(serde_json::json!(50)));

        // String float without fraction -> integer
        let v = serde_json::json!("320.0");
        let coerced = coerce_value(&v, "integer");
        assert_eq!(coerced, Some(serde_json::json!(320)));

        // Fractional float -> None (can't coerce)
        let v = serde_json::json!(320.5);
        let coerced = coerce_value(&v, "integer");
        assert!(coerced.is_none());

        // Non-numeric string -> None
        let v = serde_json::json!("abc");
        let coerced = coerce_value(&v, "integer");
        assert!(coerced.is_none());

        // Already an integer -> None (no coercion needed)
        let v = serde_json::json!(320);
        let coerced = coerce_value(&v, "integer");
        assert!(coerced.is_none());

        // Non-integer type -> None (no coercion for other types)
        let v = serde_json::json!(320.0);
        let coerced = coerce_value(&v, "string");
        assert!(coerced.is_none());
    }
}
