use apchat_models::{ToolCall, FunctionCall};

/// Parse tool calls from XML-like format used by some models (e.g., glm-4.6)
/// Format: <tool_call>TOOL_NAME\n<arg_key>KEY</arg_key>\n<arg_value>VALUE</arg_value>\n...</tool_call>
///
/// Also supports Qwen-style format (used by Qwen3.5 and similar):
/// <tool_call>\n<function=TOOL_NAME>\n<parameter=KEY>VALUE</parameter>\n</function>\n</tool_call>
pub fn parse_xml_tool_calls(content: &str) -> Option<Vec<ToolCall>> {
    if !content.contains("<tool_call>") {
        return None;
    }

    let mut tool_calls = Vec::new();
    let mut idx = 0;

    // Find all <tool_call>...</tool_call> blocks
    while let Some(start) = content[idx..].find("<tool_call>") {
        let abs_start = idx + start;
        if let Some(end) = content[abs_start..].find("</tool_call>") {
            let abs_end = abs_start + end;
            let block = &content[abs_start + 11..abs_end]; // Skip "<tool_call>"

            // Try Qwen-style format first: <function=NAME><parameter=KEY>VALUE</parameter></function>
            if let Some(tc) = parse_qwen_style_tool_call(block, tool_calls.len()) {
                tool_calls.push(tc);
                idx = abs_end + 12;
                continue;
            }

            // Fall back to original format: TOOL_NAME\n<arg_key>KEY</arg_key>\n<arg_value>VALUE</arg_value>
            // Extract tool name (first line before any tags)
            let tool_name = if let Some(first_tag) = block.find('<') {
                block[..first_tag].trim().to_string()
            } else {
                block.trim().to_string()
            };

            // Extract arguments
            let mut args = std::collections::HashMap::new();
            let mut block_idx = 0;

            while let Some(key_start) = block[block_idx..].find("<arg_key>") {
                let abs_key_start = block_idx + key_start + 9; // Skip "<arg_key>"
                if let Some(key_end) = block[abs_key_start..].find("</arg_key>") {
                    let abs_key_end = abs_key_start + key_end;
                    let key = block[abs_key_start..abs_key_end].trim();

                    // Find corresponding value
                    if let Some(val_start) = block[abs_key_end..].find("<arg_value>") {
                        let abs_val_start = abs_key_end + val_start + 11; // Skip "<arg_value>"
                        if let Some(val_end) = block[abs_val_start..].find("</arg_value>") {
                            let abs_val_end = abs_val_start + val_end;
                            let value = block[abs_val_start..abs_val_end].trim();
                            args.insert(key.to_string(), value.to_string());
                            block_idx = abs_val_end;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Build JSON arguments from extracted key-value pairs
            let json_args = build_json_args(&args);

            // Create ToolCall structure
            tool_calls.push(ToolCall {
                id: format!("call_{}", tool_calls.len()),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: tool_name,
                    arguments: json_args,
                },
            });

            idx = abs_end + 12; // Move past "</tool_call>"
        } else {
            break;
        }
    }

    if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    }
}

/// Parse Qwen-style tool call format:
/// <function=TOOL_NAME>\n<parameter=KEY>VALUE</parameter>\n</function>
fn parse_qwen_style_tool_call(block: &str, call_index: usize) -> Option<ToolCall> {
    // Look for <function=NAME>
    let func_start = block.find("<function=")?;
    let func_name_start = func_start + 10; // Skip "<function="
    let func_name_end = block[func_name_start..].find('>')? + func_name_start;
    let tool_name = block[func_name_start..func_name_end].trim().to_string();

    if tool_name.is_empty() {
        return None;
    }

    // Extract parameters: <parameter=KEY>VALUE</parameter>
    let mut args = std::collections::HashMap::new();
    let mut search_from = func_name_end;

    while let Some(param_start) = block[search_from..].find("<parameter=") {
        let abs_param_start = search_from + param_start + 11; // Skip "<parameter="
        // Find the closing > of the parameter tag
        if let Some(param_name_end) = block[abs_param_start..].find('>') {
            let abs_param_name_end = abs_param_start + param_name_end;
            let param_name = block[abs_param_start..abs_param_name_end].trim().to_string();

            // Find the value (everything until </parameter>)
            let value_start = abs_param_name_end + 1;
            if let Some(value_end) = block[value_start..].find("</parameter>") {
                let abs_value_end = value_start + value_end;
                let value = block[value_start..abs_value_end].trim().to_string();
                args.insert(param_name, value);
                search_from = abs_value_end + 12; // Skip "</parameter>"
            } else {
                break;
            }
        } else {
            break;
        }
    }

    let json_args = build_json_args(&args);

    Some(ToolCall {
        id: format!("call_{}", call_index),
        tool_type: "function".to_string(),
        function: FunctionCall {
            name: tool_name,
            arguments: json_args,
        },
    })
}

/// Build JSON argument string from a key-value map
fn build_json_args(args: &std::collections::HashMap<String, String>) -> String {
    if args.is_empty() {
        "{}".to_string()
    } else {
        let mut json_obj = serde_json::Map::new();
        for (k, v) in args {
            // Try to parse value as number if possible
            if let Ok(num) = v.parse::<i64>() {
                json_obj.insert(k.clone(), serde_json::json!(num));
            } else if v == "true" || v == "false" {
                json_obj.insert(k.clone(), serde_json::json!(v == "true"));
            } else {
                json_obj.insert(k.clone(), serde_json::json!(v));
            }
        }
        serde_json::to_string(&json_obj).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_qwen_style_tool_call() {
        let content = r#"<tool_call>
<function=read_file>
<parameter=file_path>src/main.rs</parameter>
</function>
</tool_call>"#;

        let result = parse_xml_tool_calls(content).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].function.name, "read_file");
        let args: serde_json::Value = serde_json::from_str(&result[0].function.arguments).unwrap();
        assert_eq!(args["file_path"], "src/main.rs");
    }

    #[test]
    fn test_parse_qwen_style_multiple_params() {
        let content = r#"<tool_call>
<function=edit_file>
<parameter=file_path>src/lib.rs</parameter>
<parameter=old_string>hello</parameter>
<parameter=new_string>world</parameter>
</function>
</tool_call>"#;

        let result = parse_xml_tool_calls(content).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].function.name, "edit_file");
        let args: serde_json::Value = serde_json::from_str(&result[0].function.arguments).unwrap();
        assert_eq!(args["file_path"], "src/lib.rs");
        assert_eq!(args["old_string"], "hello");
        assert_eq!(args["new_string"], "world");
    }

    #[test]
    fn test_parse_original_format_still_works() {
        let content = r#"<tool_call>read_file
<arg_key>file_path</arg_key>
<arg_value>src/main.rs</arg_value>
</tool_call>"#;

        let result = parse_xml_tool_calls(content).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].function.name, "read_file");
        let args: serde_json::Value = serde_json::from_str(&result[0].function.arguments).unwrap();
        assert_eq!(args["file_path"], "src/main.rs");
    }

    #[test]
    fn test_no_tool_calls() {
        let content = "Just some regular text without any tool calls";
        assert!(parse_xml_tool_calls(content).is_none());
    }
}
