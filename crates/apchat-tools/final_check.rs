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

    println!("Detailed line analysis:");
    for (i, line) in test_content.lines().enumerate() {
        let line_num = i + 1;
        println!("Line {}: '{}' (len={})", line_num, line, line.len());
        if line.contains("fn main") {
            println!("  ^^^ THIS IS fn main() ^^^");
        }
        if line_num == 4 {
            println!("  ^^^ THIS IS LINE 4 ^^^");
        }
    }
}
