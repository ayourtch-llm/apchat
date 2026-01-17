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

    println!("Line numbers:");
    for (i, line) in test_content.lines().enumerate() {
        println!("Line {}: '{}'", i+1, line);
    }
}
