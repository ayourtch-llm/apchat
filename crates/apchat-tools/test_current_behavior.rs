fn main() {
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

    // Include the analyze_file_content function
    include!("src/file_curly_glance.rs");
    
    println!("Starting from line 4:");
    let pairs = analyze_file_content(test_content, Some(4));
    for pair in &pairs {
        println!("  {}..{}: {}", pair.opening_line, pair.closing_line, pair.content);
    }
}
