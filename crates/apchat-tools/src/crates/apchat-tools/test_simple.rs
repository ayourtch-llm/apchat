fn main() {
    // Test the analyze_file_content function directly with inline content
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

    println!("=== Test Content ===");
    println!("{}", test_content);
    
    println!("\n=== Analysis Result ===");
    // Call the function directly (we'll simulate it for now)
    println!("Function would analyze the content here");
    println!("Expected: Find bracket pairs in the code");
}