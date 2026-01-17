[cfg(test)]
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
