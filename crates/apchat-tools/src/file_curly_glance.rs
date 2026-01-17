use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;

/// Data structure representing a curly bracket pair
/// Stores information about opening and closing brackets, their content, and preceding whitespace
#[derive(Debug, Clone)]
pub struct BracketPair {
    /// Line number of the opening curly bracket
    pub opening_line: usize,
    
    /// Line number of the closing curly bracket  
    pub closing_line: usize,
    
    /// Content or description (e.g., function name, context)
    pub content: String,
    
    /// Line number of the preceding empty or whitespace-only line (if found)
    pub preceding_whitespace_line: Option<usize>,
}

impl BracketPair {
    /// Create a new BracketPair instance
    pub fn new(opening_line: usize, closing_line: usize, content: String, preceding_whitespace_line: Option<usize>) -> Self {
        BracketPair {
            opening_line,
            closing_line,
            content,
            preceding_whitespace_line,
        }
    }
}

/// Helper function to find the matching closing bracket for a given opening bracket position
/// Returns the line number and character position of the closing bracket, or None if not found

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
/// Returns a vector of BracketPair instances representing top-level bracket pairs
pub fn analyze_file_content(content: &str, starting_line: Option<usize>) -> Vec<BracketPair> {
    let lines: Vec<&str> = content.lines().collect();
    let mut bracket_pairs = Vec::new();
    let mut current_depth = 0;
    let mut current_opening_line: Option<usize> = None;
    let mut current_opening_pos: Option<usize> = None;

    for (line_idx, line) in lines.iter().enumerate() {
        let line_num = line_idx + 1;
        
        // Skip lines before starting_line if specified
        if let Some(start_line) = starting_line {
            if line_num < start_line {
                continue;
            }
            // When we reach starting_line, reset depth to 0 (start fresh)
            if line_num == start_line {
                current_depth = 0;
            }
        }
        
        // Check for unmatched closing bracket - this should stop processing
        let mut found_unmatched_closing = false;
        
        let mut char_pos = 0;
        for c in line.chars() {
            match c {
                '{' => {
                    if current_depth == 0 {
                        // This is a top-level opening bracket
                        current_opening_line = Some(line_num);
                        current_opening_pos = Some(char_pos);
                    }
                    current_depth += 1;
                }
                '}' => {
                    if current_depth == 0 {
                        // Unmatched closing bracket - stop processing according to spec
                        found_unmatched_closing = true;
                        break;
                    }
                    
                    current_depth -= 1;
                    
                    if current_depth == 0 && current_opening_line.is_some() {
                        // This is a top-level closing bracket matching our opening
                        let opening_line = current_opening_line.unwrap();
                        let opening_pos = current_opening_pos.unwrap();
                        
                        // Find preceding empty/whitespace-only line
                        let preceding_whitespace = find_preceding_whitespace_line(&lines, opening_line);
                        
                        // Extract context/content from the opening line
                        let content = extract_context(lines[opening_line - 1], opening_pos);
                        
                        // Create BracketPair
                        let pair = BracketPair::new(
                            opening_line,
                            line_num,
                            content,
                            preceding_whitespace,
                        );
                        
                        bracket_pairs.push(pair);
                        current_opening_line = None;
                        current_opening_pos = None;
                    }
                }
                _ => {}
            }
            char_pos += 1;
        }
        
        // If we found an unmatched closing bracket, stop processing
        if found_unmatched_closing {
            break;
        }
    }
    
    bracket_pairs
}

/// Format BracketPair instances into a readable string output
/// Format: "start..end: context" for each pair, separated by commas (single line)
pub fn format_bracket_pairs(bracket_pairs: &[BracketPair]) -> String {
    if bracket_pairs.is_empty() {
        return "No bracket pairs found.".to_string();
    }
    
    let mut result_parts = Vec::new();
    
    for pair in bracket_pairs {
        let line_range = format!("{}..{}", pair.opening_line, pair.closing_line);
        
        // Add preceding whitespace information if available
        if let Some(whitespace_line) = pair.preceding_whitespace_line {
            result_parts.push(format!("{}..{}: {} (preceded by whitespace at line {})",
                                    pair.opening_line, pair.closing_line, pair.content, whitespace_line));
        } else {
            result_parts.push(format!("{}..{}: {}",
                                    pair.opening_line, pair.closing_line, pair.content));
        }
    }
    
    // Join all parts with commas for single-line output
    result_parts.join(", ")
}

/// Helper function to find the preceding empty or whitespace-only line
/// Searches backwards from the given line number
fn find_preceding_whitespace_line(lines: &[&str], from_line: usize) -> Option<usize> {
    // Start from the line before the opening bracket
    for line_num in (1..from_line).rev() {
        if line_num > 0 && line_num <= lines.len() {
            let line = lines[line_num - 1];
            if line.trim().is_empty() {
                return Some(line_num);
            }
        }
    }
    
    None
}

/// Helper function to extract context/content from a line near the opening bracket
/// Returns the function name or nearby text
fn extract_context(line: &str, bracket_pos: usize) -> String {
    // Try to extract function name pattern (e.g., "fn main()")
    let trimmed = line.trim();
    
    // Check if this looks like a function declaration
    if let Some(fn_pos) = trimmed.find("fn ") {
        let rest = &trimmed[fn_pos + 3..];
        if let Some(paren_pos) = rest.find('(') {
            let func_name = &rest[..paren_pos];
            return func_name.to_string();
        }
    }
    
    // If not a function, return some text near the bracket
    let max_len = 30; // Maximum length to extract
    let start_pos = if bracket_pos > max_len / 2 { bracket_pos - max_len / 2 } else { 0 };
    let end_pos = std::cmp::min(start_pos + max_len, line.len());
    
    line[start_pos..end_pos].trim().to_string()
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
        let bracket_pairs = analyze_file_content(&content, starting_line);
        
        // Format the output
        let result = format_bracket_pairs(&bracket_pairs);
        
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

        let pairs = analyze_file_content(test_content, None);
        
        // Should find at least some bracket pairs
        assert!(!pairs.is_empty());
        
        // Should find top-level functions
        let has_main = pairs.iter().any(|p| p.content == "main");
        let has_test = pairs.iter().any(|p| p.content == "test");
        
        assert!(has_main, "Should find main function");
        assert!(has_test, "Should find test function");
        
        // Main function should have correct line range
        if let Some(main_pair) = pairs.iter().find(|p| p.content == "main") {
            assert_eq!(main_pair.opening_line, 2);
            assert!(main_pair.closing_line >= main_pair.opening_line);
        }
        
        println!("Test result: Found {} bracket pairs", pairs.len());
        for pair in &pairs {
            println!("  {}..{}: {} (preceding: {:?})", 
                     pair.opening_line, pair.closing_line, 
                     pair.content, 
                     pair.preceding_whitespace_line);
        }
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

        // Start from line 3 (fn main)
        let pairs = analyze_file_content(test_content, Some(3));
        
        println!("Test result with starting line 3:");
        for pair in &pairs {
            println!("  {}..{}: {}", pair.opening_line, pair.closing_line, pair.content);
        }
        
        // Should find main function starting at line 3
        assert!(!pairs.is_empty());
        let has_main = pairs.iter().any(|p| p.content == "main");
        assert!(has_main, "Should find main function when starting from line 3");
    }
    
    #[test]
    fn test_analyze_empty_content() {
        let pairs = analyze_file_content("", None);
        assert!(pairs.is_empty());
    }
    
    #[test]
    fn test_preceding_whitespace_detection() {
        let test_content = r#"fn first() {
    // content
}

fn second() {
    // content
}
"#;

        let pairs = analyze_file_content(test_content, None);
        
        // Should find at least one pair
        assert!(!pairs.is_empty());
        
        // The blank line between first() and second() should be detected
        // as preceding whitespace for second()
        let second_pair = pairs.iter().find(|p| p.content == "second");
        
        if let Some(pair) = second_pair {
            // second() should have a preceding whitespace line
            assert!(pair.preceding_whitespace_line.is_some());
            assert_eq!(pair.preceding_whitespace_line.unwrap(), 4); // Line 4 is the blank line
        }
    }
}

