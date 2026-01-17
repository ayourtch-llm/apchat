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

    println!("Raw string analysis:");
    println!("Length: {}", test_content.len());
    println!("\nLines:");
    for (i, line) in test_content.lines().enumerate() {
        let is_empty = line.trim().is_empty();
        println!("Line {}: '{}' (len={}, empty={})", i+1, line, line.len(), is_empty);
    }
}
