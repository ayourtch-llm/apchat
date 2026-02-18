use async_trait::async_trait;
use std::collections::HashMap;

use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;

pub struct ScratchpadReadTool;

#[async_trait]
impl Tool for ScratchpadReadTool {
    fn name(&self) -> &str {
        "scratchpad_read"
    }

    fn description(&self) -> &str {
        "Read the current contents of the scratchpad. The scratchpad is a persistent text buffer you can use to store notes, plans, intermediate results, or any information you want to keep across tool calls."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::new()
    }

    async fn execute(&self, _params: ToolParameters, context: &ToolContext) -> ToolResult {
        let scratchpad = match context.scratchpad.as_ref() {
            Some(sp) => sp,
            None => return ToolResult::error("Scratchpad not available".to_string()),
        };

        let content = match scratchpad.lock() {
            Ok(guard) => guard.clone(),
            Err(e) => return ToolResult::error(format!("Failed to read scratchpad: {}", e)),
        };

        if content.is_empty() {
            ToolResult::success("(scratchpad is empty)".to_string())
        } else {
            ToolResult::success(content)
        }
    }
}

pub struct ScratchpadWriteTool;

#[async_trait]
impl Tool for ScratchpadWriteTool {
    fn name(&self) -> &str {
        "scratchpad_write"
    }

    fn description(&self) -> &str {
        "Write content to the scratchpad, replacing its current contents. The scratchpad is a persistent text buffer you can use to store notes, plans, intermediate results, or any information you want to keep across tool calls."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("content", "string", "The text content to write to the scratchpad. This replaces the entire scratchpad contents.", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let content = match params.get_required::<String>("content") {
            Ok(c) => c,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let scratchpad = match context.scratchpad.as_ref() {
            Some(sp) => sp,
            None => return ToolResult::error("Scratchpad not available".to_string()),
        };

        match scratchpad.lock() {
            Ok(mut guard) => {
                let len = content.len();
                *guard = content;
                ToolResult::success(format!("Scratchpad updated ({} bytes written)", len))
            }
            Err(e) => ToolResult::error(format!("Failed to write scratchpad: {}", e)),
        }
    }
}
