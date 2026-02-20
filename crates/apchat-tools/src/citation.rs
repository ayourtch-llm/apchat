use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const CITATIONS_FILE: &str = "citations.txt";

/// Verify that a given citation text exists within the contents of a local file path.
/// This function is intentionally separated so it can later be extended to support
/// other path types (e.g., URLs, remote resources).
pub fn verify_citation_in_path(file_path: &Path, citation_text: &str) -> Result<bool, String> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file '{}': {}", file_path.display(), e))?;
    Ok(content.contains(citation_text))
}

/// Tool for adding a verified citation to citations.txt
pub struct AddCitationTool;

#[async_trait]
impl Tool for AddCitationTool {
    fn name(&self) -> &str {
        "add_citation"
    }

    fn description(&self) -> &str {
        "Add a verified citation to citations.txt. Verifies that the cited file exists and \
         contains the given citation text, that the citation label uses only allowed characters \
         [a-zA-Z0-9-_+/], and that the label is not already present in citations.txt."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("citation_label", "string", "Unique label for the citation (allowed characters: a-z, A-Z, 0-9, -, _, +, /)", required),
            param!("file_path", "string", "Path to the file being cited, relative to the work directory", required),
            param!("citation_text", "string", "Exact text that must appear in the cited file", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let citation_label = match params.get_required::<String>("citation_label") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };
        let file_path = match params.get_required::<String>("file_path") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };
        let citation_text = match params.get_required::<String>("citation_text") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        // Validate citation_label characters
        if !citation_label.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '+' | '/')) {
            return ToolResult::error(format!(
                "Invalid citation_label '{}': only characters [a-zA-Z0-9-_+/] are allowed",
                citation_label
            ));
        }

        // Reject newlines in file_path and citation_text to prevent malformed entries
        if file_path.contains('\n') || file_path.contains('\r') {
            return ToolResult::error("file_path must not contain newline characters".to_string());
        }
        if citation_text.contains('\n') || citation_text.contains('\r') {
            return ToolResult::error("citation_text must not contain newline characters".to_string());
        }

        // Verify that file_path exists
        let full_path = context.work_dir.join(&file_path);
        if !full_path.exists() {
            return ToolResult::error(format!("File not found: {}", file_path));
        }

        // Verify that the file contains the citation_text
        match verify_citation_in_path(&full_path, &citation_text) {
            Ok(true) => {}
            Ok(false) => {
                return ToolResult::error(format!(
                    "Citation text not found in file '{}': \"{}\"",
                    file_path, citation_text
                ));
            }
            Err(e) => return ToolResult::error(e),
        }

        // Check citations.txt for duplicate label
        let citations_path = context.work_dir.join(CITATIONS_FILE);
        let existing = if citations_path.exists() {
            match fs::read_to_string(&citations_path) {
                Ok(c) => c,
                Err(e) => return ToolResult::error(format!("Failed to read citations.txt: {}", e)),
            }
        } else {
            String::new()
        };

        let prefix = format!("{}:", citation_label);
        if existing.lines().any(|line| line.starts_with(&prefix)) {
            return ToolResult::error(format!(
                "Citation label '{}' already exists in citations.txt",
                citation_label
            ));
        }

        // Append new citation entry
        let new_entry = format!("{}:{}:{}\n", citation_label, file_path, citation_text);
        let updated = format!("{}{}", existing, new_entry);
        if let Err(e) = fs::write(&citations_path, &updated) {
            return ToolResult::error(format!("Failed to write citations.txt: {}", e));
        }

        ToolResult::success(format!(
            "Citation '{}' added successfully to citations.txt",
            citation_label
        ))
    }
}
