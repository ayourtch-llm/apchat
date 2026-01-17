fn main() {
    // Test 1: Verify single-line output format
    let test_content1 = r#"fn first() {
    let x = 1;
}

fn second() {
    let y = 2;
}
"#;

    // Include the function (simplified for testing)
    let lines: Vec<&str> = test_content1.lines().collect();
    let mut bracket_pairs = Vec::new();
    let mut current_depth = 0;
    let mut current_opening_line: Option<usize> = None;
    let mut current_opening_pos: Option<usize> = None;

    for (line_idx, line) in lines.iter().enumerate() {
        let line_num = line_idx + 1;
        
        let mut char_pos = 0;
        for c in line.chars() {
            match c {
                '{' => {
                    if current_depth == 0 {
                        current_opening_line = Some(line_num);
                        current_opening_pos = Some(char_pos);
                    }
                    current_depth += 1;
                }
                '}' => {
                    if current_depth == 0 {
                        break;
                    }
                    current_depth -= 1;
                    if current_depth == 0 && current_opening_line.is_some() {
                        let opening_line = current_opening_line.unwrap();
                        let opening_pos = current_opening_pos.unwrap();
                        let content = extract_context(lines[opening_line - 1], opening_pos);
                        bracket_pairs.push((opening_line, line_num, content));
                        current_opening_line = None;
                        current_opening_pos = None;
                    }
                }
                _ => {}
            }
            char_pos += 1;
        }
    }
    
    // Format as single line
    let result: Vec<String> = bracket_pairs.iter().map(|(o, c, content)| format!("{}..{}: {}", o, c, content)).collect();
    let output = result.join(", ");
    
    println!("Test 1 - Single line output:");
    println!("Output: {}", output);
    println!("Expected format: start..end: context, start..end: context");
    assert!(output.contains(", "), "Output should be single line with commas");
    println!("✓ Passed\n");
}

fn extract_context(line: &str, bracket_pos: usize) -> String {
    let trimmed = line.trim();
    if let Some(fn_pos) = trimmed.find("fn ") {
        let rest = &trimmed[fn_pos + 3..];
        if let Some(paren_pos) = rest.find('(') {
            let func_name = &rest[..paren_pos];
            return func_name.to_string();
        }
    }
    line.trim().to_string()
}
