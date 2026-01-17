use std::fs;

// Import the analyze_file_content function
use apchat_tools::file_curly_glance::analyze_file_content;

fn main() {
    // Test with the test file
    let content = fs::read_to_string("test_curly.rs").expect("Failed to read test file");
    
    println!("=== Testing analyze_file_content ===");
    println!("\nContent of test_curly.rs:");
    println!("{}", content);
    
    println!("\n=== Analysis Result ===");
    let result = analyze_file_content(&content, None);
    println!("{}", result);
    
    println!("\n=== Testing with starting_line ===");
    let result_with_start = analyze_file_content(&content, Some(5));
    println!("{}", result_with_start);
}