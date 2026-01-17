use std::fs;

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

    println!("Content:");
    for (i, line) in test_content.lines().enumerate() {
        println!("{}: {}", i+1, line);
    }
    
    println!("\nStarting from line 4:");
    
    // Simulate the algorithm
    let lines: Vec<&str> = test_content.lines().collect();
    let mut current_depth = 0;
    let mut current_opening_line: Option<usize> = None;
    
    // Pre-scan to calculate depth at line 4
    let mut initial_depth = 0;
    let start_line = Some(4);
    for (line_idx, line) in lines.iter().enumerate() {
        let line_num = line_idx + 1;
        if line_num >= start_line.unwrap() {
            break;
        }
        
        for c in line.chars() {
            match c {
                '{' => initial_depth += 1,
                '}' => initial_depth -= 1,
                _ => {}
            }
        }
    }
    current_depth = initial_depth;
    
    println!("Initial depth at line 4: {}", current_depth);
    
    for (line_idx, line) in lines.iter().enumerate() {
        let line_num = line_idx + 1;
        
        // Skip lines before starting_line if specified
        if let Some(start_line) = start_line {
            if line_num < start_line {
                continue;
            }
        }
        
        println!("\nLine {}: '{}' (depth before: {})", line_num, line, current_depth);
        
        let mut char_pos = 0;
        for c in line.chars() {
            match c {
                '{' => {
                    if current_depth == 0 {
                        current_opening_line = Some(line_num);
                        println!("  Found opening at line {} (depth 0)", line_num);
                    } else {
                        println!("  Found opening at line {} (depth {}), nesting", line_num, current_depth);
                    }
                    current_depth += 1;
                    println!("  Depth after {{: {}", current_depth);
                }
                '}' => {
                    if current_depth == 0 {
                        println!("  Unmatched closing at line {}", line_num);
                        break;
                    }
                    
                    current_depth -= 1;
                    println!("  Depth after }}: {}", current_depth);
                    
                    if current_depth == 0 && current_opening_line.is_some() {
                        let opening_line = current_opening_line.unwrap();
                        println!("  Matching {}..{} (opening at line {}, now at line {})", 
                                 opening_line, line_num, opening_line, line_num);
                        current_opening_line = None;
                    }
                }
                _ => {}
            }
            char_pos += 1;
        }
    }
}
