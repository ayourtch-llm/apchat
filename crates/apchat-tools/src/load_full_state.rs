use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;

/// Tool for loading full conversation state from a file
/// Only available when --load flag is enabled
pub struct LoadFullStateTool;

#[async_trait]
impl Tool for LoadFullStateTool {
    fn name(&self) -> &str {
        "load_full_state"
    }

    fn description(&self) -> &str {
        "Load the full conversation state from a file. Requires --load flag to be enabled. Use this to resume a previous session that was saved with save_full_state."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("file_path", "string", "Path to the state file to load (e.g., ~/.apchat_state.json)", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let file_path = match params.get_required::<String>("file_path") {
            Ok(path) => path,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        // This tool is a placeholder - actual load happens via APChat::load_state()
        // The tool registry will call back into APChat to perform the load
        ToolResult::success(format!("State load requested for path: {}", file_path))
    }
}