use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use std::collections::HashMap;
use apchat_models::types::ModelColor;
use apchat_llm_api::client::{LlmClient, ChatMessage};
use async_trait::async_trait;
use std::fs;
use std::sync::Arc;

/// Tool for making one-shot calls to LLM models
pub struct LlmCallTool;

#[async_trait]
impl Tool for LlmCallTool {
    fn name(&self) -> &str {
        "llm_oneshot"
    }

    fn description(&self) -> &str {
        "Make a one-shot call to an LLM model. Accepts model color (red/grn/blu), instruction, and optionally a file path to append to the instruction."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("model_color", "string", "Model color to call (red, grn, blu)", required),
            param!("instruction", "string", "Instruction/prompt for the LLM", required),
            param!("file_path", "string", "Optional file path, from which to append contents to instruction", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        // Parse model_color parameter
        let model_color_str = match params.get_required::<String>("model_color") {
            Ok(s) => s,
            Err(e) => return ToolResult::error(e.to_string()),
        };
        
        // Parse instruction parameter
        let instruction = match params.get_required::<String>("instruction") {
            Ok(s) => s,
            Err(e) => return ToolResult::error(e.to_string()),
        };
        
        // Parse optional file_path parameter
        let file_path = match params.get_optional::<String>("file_path") {
            Ok(Some(path)) => path,
            Ok(None) => String::new(),
            Err(e) => return ToolResult::error(e.to_string()),
        };
        
        // Append file contents if file_path is provided
        let mut full_prompt = instruction;
        if !file_path.is_empty() {
            match fs::read_to_string(&file_path) {
                Ok(content) => {
                    full_prompt.push_str("\n\nFile contents:\n");
                    full_prompt.push_str(&content);
                }
                Err(e) => {
                    return ToolResult::error(format!(
                        "Failed to read file '{}': {}",
                        file_path, e
                    ));
                }
            }
        }
        
        // Get the appropriate LLM client based on model color
        let model_color = match model_color_str.to_lowercase().as_str() {
            "red" => ModelColor::RedModel,
            "grn" => ModelColor::GrnModel,
            "blu" => ModelColor::BluModel,
            _ => return ToolResult::error(format!(
                "Invalid model color: '{}'. Use 'red', 'grn', or 'blu'",
                model_color_str
            )),
        };
        
        // Create chat message
        let message = ChatMessage {
            role: "user".to_string(),
            content: full_prompt,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning: None,
        };
        
        // Try to get the LLM client from context
        match context.get_llm_client(&model_color) {
            Some(client) => {
                match client.chat_completion(&[message]).await {
                    Ok(response) => {
                        ToolResult::success(response)
                    }
                    Err(e) => {
                        ToolResult::error(format!("LLM call failed: {}", e))
                    }
                }
            }
            None => {
                ToolResult::error(format!(
                    "No LLM client configured for model color: {:?}",
                    model_color
                ))
            }
        }
    }
}
