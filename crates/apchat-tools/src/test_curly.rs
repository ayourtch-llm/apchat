// Test file for curly bracket analysis
// This is a comment

fn main() {
    // Outer function block
    let x = 5;
    let y = 10;
    
    if x < y {
        // Inner if block
        println!("x is less than y");
    } else {
        // Else block
        println!("x is not less than y");
    }
    
    // Loop block
    for i in 0..5 {
        println!("Loop iteration: {}", i);
    }
}

fn another_function() {
    let data = vec![1, 2, 3, 4, 5];
    
    // Nested blocks
    {
        let inner_var = "hello";
        println!("{}", inner_var);
    }
    
    println!("Outer scope");
}
