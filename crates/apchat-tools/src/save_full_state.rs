use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;

/// Tool for saving full conversation state to a file
/// Only available when --save flag is enabled
pub struct SaveFullStateTool;

#[async_trait]
impl Tool for SaveFullStateTool {
    fn name(&self) -> &str {
        "save_full_state"
    }

    fn description(&self) -> &str {
        "Save the full conversation state to a file. Requires --save flag to be enabled. Use this to persist your work and resume later with --load."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("file_path", "string", "Path to save the state file to (e.g., ~/.apchat_state.json)", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let file_path = match params.get_required::<String>("file_path") {
            Ok(path) => path,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        // This tool is a placeholder - actual save happens via APChat::save_state()
        // The tool registry will call back into APChat to perform the save
        ToolResult::success(format!("State save requested for path: {}", file_path))
    }
}