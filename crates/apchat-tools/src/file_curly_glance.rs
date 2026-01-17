use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;

/// Tool for analyzing file curly bracket structures
pub struct FileCurlyGlanceTool;

/// Helper function to find the matching closing bracket for a given opening bracket position
/// Returns the line number and character position of the closing bracket, or None if not found
pub fn find_matching_closing_bracket(content: &str, opening_pos: usize) -> Option<(usize, usize)> {
    let chars: Vec<char> = content.chars().collect();
    let mut depth = 1;
    let mut pos = opening_pos + 1;
    let mut line_number = 1;
    let mut char_pos_in_line = 0;
    
    for (i, c) in chars.iter().enumerate().skip(opening_pos + 1) {
        if *c == '\n' {
            line_number += 1;
            char_pos_in_line = 0;
        } else {
            char_pos_in_line += 1;
        }
        
        match *c {
            '{' => {
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((line_number, char_pos_in_line));
                }
            }
            _ => {}
        }
        
        pos = i + 1;
    }
    
    None
}

/// Helper function to check if a line is empty or contains only whitespace
pub fn is_empty_or_whitespace(line: &str) -> bool {
    line.trim().is_empty()
}

/// Helper function to find the starting line of a bracket structure
/// Returns the line number where the first non-whitespace character is found after the opening bracket
pub fn find_starting_line(content: &str, opening_pos: usize) -> usize {
    let chars: Vec<char> = content.chars().collect();
    let mut line_number = 1;
    
    // Find the line containing the opening bracket
    for (_i, c) in chars.iter().enumerate().take(opening_pos) {
        if *c == '\n' {
            line_number += 1;
        }
    }
    
    line_number
}

#[async_trait]
impl Tool for FileCurlyGlanceTool {
    fn name(&self) -> &str {
        "file_curly_glance"
    }

    fn description(&self) -> &str {
        "Analyze file curly bracket structures. Finds top-level curly brackets, matches pairs, and outputs content between them with line information."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("file_path", "string", "Path to the file relative to the work directory", required),
            param!("starting_line", "integer", "Optional starting line number (1-based). If supplied, scan starts from this line and stops at extraneous closing bracket.", optional),
        ])
    }

    async fn execute(&self, _params: ToolParameters, _context: &ToolContext) -> ToolResult {
        // Implementation will be added in subsequent steps
        ToolResult::success("Not yet implemented".to_string())
    }
}
