use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;

/// Tool for making one-shot LLM calls
pub struct LlmCallTool;

#[async_trait]
impl Tool for LlmCallTool {
    fn name(&self) -> &str {
        "llm_oneshot"
    }

    fn description(&self) -> &str {
        "Make a one-shot call to an LLM model. Use this for simple, focused queries without conversation history."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("model_color", "string", "Color of the model to use (grn, blu, red)", required),
            param!("instruction", "string", "The instruction or prompt to send to the LLM", required),
            param!("file_path", "string", "Optional file path for context", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        // Implementation will be added later
        ToolResult::error("llm_oneshot: execute() method not implemented yet".to_string())
    }
}
