// Debug test to understand the JSON parsing
fn main() {
    let test_line = r#"{"command":"command 1","session_id":null,"timestamp":"2026-01-21T18:12:39.907668Z"}{"command":"test command","session_id":null,"timestamp":"2026-01-21T18:12:39.907637Z"}"#;

    println!("Original line: {}", test_line);
    println!("Length: {}", test_line.len());
    println!();

    // Try to parse the first object
    match serde_json::from_str::<serde_json::Value>(&test_line) {
        Ok(v) => {
            println!("✅ Parsed entire line as JSON (unexpected!): {}", v);
        }
        Err(e) => {
            println!("❌ Expected error parsing entire line: {}", e);
            println!();
        }
    }

    // Try parsing from position 0
    let substring = &test_line[0..];
    println!("Substring [0..]: {}", substring);
    match serde_json::from_str::<serde_json::Value>(substring) {
        Ok(v) => {
            println!("✅ Parsed from position 0");
            let json_str = serde_json::to_string(&v).unwrap();
            println!("   Serialized: {}", json_str);
            println!("   Serialized length: {}", json_str.len());

            // Try to find the end
            let bytes = substring.as_bytes();
            let mut brace_count = 0;
            let mut end_idx = 0;
            for (i, &byte) in bytes.iter().enumerate() {
                match byte {
                    b'{' => brace_count += 1,
                    b'}' => {
                        brace_count -= 1;
                        if brace_count == 0 {
                            end_idx = i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            println!("   End index: {}", end_idx);
            println!("   Next char should be at position: {}", end_idx);
            if end_idx < substring.len() {
                println!("   Next char: '{}'", substring.chars().nth(end_idx).unwrap());
            }
        }
        Err(e) => {
            println!("❌ Failed to parse from position 0: {}", e);
        }
    }
}
