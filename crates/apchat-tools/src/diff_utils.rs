use similar::{ChangeTag, TextDiff};
use colored::Colorize;

/// Generate a unified diff string between old and new content
/// Returns a formatted diff string suitable for terminal display
pub fn generate_unified_diff(old_content: &str, new_content: &str) -> String {
    let diff = TextDiff::from_lines(old_content, new_content);
    let mut result = String::new();
    
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        
        let colored_value = match change.tag() {
            ChangeTag::Delete => change.value().red().to_string(),
            ChangeTag::Insert => change.value().green().to_string(),
            ChangeTag::Equal => change.value().bright_black().to_string(),
        };
        
        result.push_str(&format!("{}{}", colored_value, sign));
        if !change.value().ends_with('\n') && change.tag() != ChangeTag::Equal {
            result.push('\n');
        }
    }
    
    result
}