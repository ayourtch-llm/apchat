// Minimal test to verify parameter_validation.rs syntax

use serde_json::Value;
use std::collections::HashMap;
use apchat_models::ToolCall;
use apchat_toolcore::tool::{ToolParameters, ParameterDefinition};

fn validate_tool_call(
    tool_call: &ToolCall,
    tool_schema: &ToolParameters,
    param_definitions: &HashMap<String, ParameterDefinition>
) -> Result<ToolParameters, String> {
    // Parse the arguments JSON
    let parsed_args: HashMap<String, Value> = serde_json::from_str(&tool_call.function.arguments)
        .unwrap();

    // Extract parameter names from parameter definitions
    let valid_params: Vec<String> = param_definitions.keys().cloned().collect();

    // Extract required parameter names from parameter definitions
    let required_params: Vec<String> = param_definitions
        .iter()
        .filter(|(_, def)| def.required)
        .map(|(name, _)| name.clone())
        .collect();

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

    // Check for invalid/extra parameters
    for param_name in parsed_args.keys() {
        if !valid_params.contains(param_name) {
            let valid_params_str = valid_params.join(", ");
            return Err(format!(
                "Tool '{}' has invalid parameter '{}'. Available: {}",
                tool_call.function.name, param_name, valid_params_str
            ));
        }
    }

    // All checks passed, return the parsed parameters
    Ok(ToolParameters {
        data: parsed_args,
    })
}

fn main() {
    println!("Test passed!");
}
