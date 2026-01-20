[cfg(test)]
mod detailed_test {
    use super::*;
    use apchat_logging::vty_output::print_heart_red;

    #[test]
    fn test_unmatched_opening_at_eof_detailed() {
        let test_content = r#"fn main() {
    let x = 5;
}

fn test() {
    let y = 10;
// Unmatched opening bracket
{"#;

        let pairs = analyze_file_content(test_content, None);
        
        print_heart_red(&format!("\n=== Testing unmatched opening bracket at EOF ==="), true);
        print_heart_red(&format!("Content:"), true);
        print_heart_red(&format!("{}", test_content), true);
        print_heart_red(&format!("\nFound {} bracket pairs:", pairs.len()), true);
        
        for pair in &pairs {
            print_heart_red(&format!("  {}..{}: {} (preceding whitespace: {:?})", 
                     pair.opening_line, pair.closing_line, 
                     pair.content, 
                     pair.preceding_whitespace_line), true);
        }
        
        // Should find the main function
        let has_main = pairs.iter().any(|p| p.content == "main");
        assert!(has_main, "Should find main function");
        
        // Should find test function (even though it's incomplete)
        let has_test = pairs.iter().any(|p| p.content == "test");
        assert!(has_test, "Should find test function (incomplete at EOF)");
        
        // Verify main function has correct range
        if let Some(main_pair) = pairs.iter().find(|p| p.content == "main") {
            assert_eq!(main_pair.opening_line, 1);
            assert_eq!(main_pair.closing_line, 2);
            print_heart_red(&format!("\nMain function: {}..{} ✓", main_pair.opening_line, main_pair.closing_line), true);
        }
        
        // Verify test function has EOF as closing line
        if let Some(test_pair) = pairs.iter().find(|p| p.content == "test") {
            assert_eq!(test_pair.opening_line, 5);
            assert_eq!(test_pair.closing_line, 8); // EOF
            print_heart_red(&format!("Test function: {}..{} (incomplete at EOF) ✓", test_pair.opening_line, test_pair.closing_line), true);
        }
    }
}