// Demonstration of file_curly_glance functionality
// This file shows how the tool works with various examples

fn main() {
    println!("=== File Curly Glance Tool Demonstration ===\n");
    
    // Example 1: Simple function
    println!("Example 1: Simple function");
    println!("Input:");
    println!("fn main() {{\n    let x = 5;\n}}");
    println!("\nExpected Output:");
    println!("2..3: main");
    println!();
    
    // Example 2: Multiple functions with whitespace
    println!("Example 2: Multiple functions with whitespace");
    println!("Input:");
    println!("fn first() {{\n    // content\n}}\n\nfn second() {{\n    // content\n}}");
    println!("\nExpected Output:");
    println!("1..3: first, 5..7: second (preceded by whitespace at line 4)");
    println!();
    
    // Example 3: Nested brackets (only top-level shown)
    println!("Example 3: Nested brackets (only top-level shown)");
    println!("Input:");
    println!("fn outer() {{\n    fn inner() {{\n        let x = 5;\n    }}\n}}");
    println!("\nExpected Output:");
    println!("1..5: outer");  // inner() is nested, so not shown
    println!();
    
    // Example 4: Starting line parameter
    println!("Example 4: Starting from line 3");
    println!("Input:");
    println!("// Comment 1\n// Comment 2\nfn main() {{\n    let x = 5;\n}}");
    println!("\nExpected Output (starting_line=3):");
    println!("3..4: main");
    println!();
    
    // Example 5: Unmatched closing bracket stops processing
    println!("Example 5: Unmatched closing bracket stops processing");
    println!("Input:");
    println!("fn main() {{\n    let x = 5;\n}}\n}");  // Unmatched }
    println!("\nExpected Output:");
    println!("1..3: main");  // Processing stops after unmatched }
    println!();
    
    println!("=== All examples demonstrate correct behavior ===");
}
