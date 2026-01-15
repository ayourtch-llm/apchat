use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use std::collections::HashMap;
use apchat_models::types::ModelColor;
use async_trait::async_trait;

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
        // Implementation will be added in next steps
        ToolResult::error("Not implemented yet".to_string())
    }
}
