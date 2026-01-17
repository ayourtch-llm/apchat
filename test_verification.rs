use apchat_tools::file_curly_glance::analyze_file_content;

fn main() {
    let test_content = r#"fn main() {
    let x = 5;
}

fn test() {
    let y = 10;
// Unmatched opening bracket
{"#;

    let pairs = analyze_file_content(test_content, None);
    
    println!("Found {} bracket pairs:", pairs.len());
    for pair in &pairs {
        println!("  {}..{}: {}", pair.opening_line, pair.closing_line, pair.content);
    }
}