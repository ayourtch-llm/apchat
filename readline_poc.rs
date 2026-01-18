// Proof of Concept: Readline Cleanup Issue
//
// This demonstrates the current issue and proposed fix

use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

fn demonstrate_issue() {
    // Current implementation - history not saved
    let mut rl = DefaultEditor::new().unwrap();
    
    // Simulate user input
    let line = "ls -la".to_string();
    rl.add_history_entry(&line).unwrap();
    
    println!("Added '{}' to history", line);
    println!("History size: {}", rl.history().len());
    
    // Exit without saving - CURRENT BEHAVIOR
    // rl.save_history("history.txt").unwrap(); // <-- MISSING!
    
    println!("Exited without saving history - data lost!");
}

fn demonstrate_fix() {
    // Proposed fix - with cleanup
    let mut rl = DefaultEditor::new().unwrap();
    
    // Simulate user input
    let line = "ls -la".to_string();
    rl.add_history_entry(&line).unwrap();
    
    println!("Added '{}' to history", line);
    println!("History size: {}", rl.history().len());
    
    // Exit WITH saving - PROPOSED FIX
    rl.save_history("history.txt").unwrap();
    
    println!("Exited with history saved successfully!");
    println!("History file created at: history.txt");
}

fn main() {
    println!("=== Demonstrating Readline Cleanup Issue ===\n");
    
    println!("--- Current Behavior (ISSUE) ---");
    demonstrate_issue();
    
    println!("\n--- Proposed Fix ---");
    demonstrate_fix();
    
    println!("\n=== Analysis ===");
    println!("The issue: Readline editor's in-memory history is lost on exit");
    println!("The fix: Call rl.save_history() before the function returns");
    println!("Impact: User commands will persist across sessions");
}
