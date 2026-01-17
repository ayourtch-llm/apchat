use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;

/// Tool for analyzing file curly bracket structures
pub struct FileCurlyGlanceTool;

/// Helper function to find the matching closing bracket for a given opening bracket position
/// Returns the line number and character position of the closing bracket, or None if not found
pub fn find_matching_closing_bracket(content: &str, opening_pos: usize) -> Option<(usize, usize)> {
    let chars: Vec<char> = content.chars().collect();
    let mut depth = 1;
    let _pos = opening_pos + 1;
    let mut line_number = 1;
    let mut char_pos_in_line = 0;
    
    for (_i, c) in chars.iter().enumerate().skip(opening_pos + 1) {
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

/// Helper function to analyze file content and find curly bracket structures
/// Returns a formatted string showing bracket pairs with line information
pub fn analyze_file_content(content: &str, starting_line: Option<usize>) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = String::new();
    let _current_line = 1;
    let mut depth = 0;
    let mut bracket_pairs = Vec::new();
    
    // Track opening bracket positions
    let mut opening_brackets = Vec::new();
    
    // First pass: find all opening brackets and their positions
    for (line_idx, line) in lines.iter().enumerate() {
        let line_num = line_idx + 1;
        
        // Skip lines before starting_line if provided
        if let Some(start_line) = starting_line {
            if line_num < start_line {
                continue;
            }
        }
        
        let mut char_pos = 0;
        for c in line.chars() {
            match c {
                '{' => {
                    opening_brackets.push((line_num, char_pos, line_idx));
                }
                '}' => {
                    if let Some((open_line, open_pos, open_idx)) = opening_brackets.pop() {
                        // Found a matching pair
                        bracket_pairs.push((open_line, open_pos, line_num, char_pos, open_idx, line_idx));
                    }
                }
                _ => {}
            }
            char_pos += 1;
        }
    }
    
    // Check if we found any bracket pairs
    if bracket_pairs.is_empty() {
        return "No bracket pairs found.".to_string();
    }
    
    // Second pass: output the bracket structures with content
    for (open_line, open_pos, close_line, close_pos, open_idx, close_idx) in &bracket_pairs {
        result.push_str(&format!("Bracket pair {} - {}: Line {} (col {}) to Line {} (col {})\n",
                                 depth, depth, open_line, open_pos, close_line, close_pos));
        
        // Show content between brackets (excluding outer brackets themselves)
        if open_idx + 1 <= close_idx - 1 {
            result.push_str("Content:\n");
            for line_num in open_idx + 1..=close_idx - 1 {
                if line_num < lines.len() {
                    result.push_str(&format!("  {}: {}\n", line_num + 1, lines[line_num]));
                }
            }
        } else {
            result.push_str("  (empty block)\n");
        }
        
        result.push_str("\n");
        depth += 1;
    }
    
    result
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

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        // Get file_path parameter
        let file_path = match params.get_required::<String>("file_path") {
            Ok(path) => path,
            Err(e) => return ToolResult::error(e.to_string()),
        };
        
        // Get starting_line parameter (optional)
        let starting_line = params.get_optional::<usize>("starting_line").unwrap_or(None);
        
        // Read the file using ToolContext's work_dir
        let full_path = context.work_dir.join(&file_path);
        
        if !full_path.exists() {
            return ToolResult::error(format!("File not found: {}", file_path));
        }
        
        // Check if it's a directory
        if full_path.is_dir() {
            return ToolResult::error(format!(
                "Path '{}' is a directory, not a file.", file_path
            ));
        }
        
        // Read file content
        let content = match fs::read_to_string(&full_path) {
            Ok(content) => content,
            Err(e) => return ToolResult::error(format!("Failed to read file: {}", e)),
        };
        
        // Analyze the file content
        let result = analyze_file_content(&content, starting_line);
        
        ToolResult::success(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_file_content() {
        let test_content = r#"// Test file
fn main() {
    let x = 5;
    
    if x > 0 {
        println!("positive");
    } else {
        println!("not positive");
    }
}

fn test() {
    let y = 10;
}
"#;

        let result = analyze_file_content(test_content, None);
        
        // Should find at least some bracket pairs
        assert!(!result.contains("No bracket pairs found."));
        
        // Should contain line information
        assert!(result.contains("Line"));
        
        println!("Test result:\n{}", result);
    }
    
    #[test]
    fn test_analyze_file_content_with_starting_line() {
        let test_content = r#"// Line 1
// Line 2
fn main() {
    let x = 5;
    
    if x > 0 {
        println!("positive");
    }
}

fn test() {
    let y = 10;
}
"#;

        // Start from line 4 (fn main)
        let result = analyze_file_content(test_content, Some(4));
        
        println!("Test result with starting line 4:\n{}", result);
    }
    
    #[test]
    fn test_analyze_empty_content() {
        let result = analyze_file_content("", None);
        assert_eq!(result, "No bracket pairs found.");
    }
    
    #[test]
    fn test_analyze_no_brackets() {
        let result = analyze_file_content("just some text", None);
        assert_eq!(result, "No bracket pairs found.");
    }
}

