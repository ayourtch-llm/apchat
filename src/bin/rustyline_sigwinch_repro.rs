/// Minimal reproduction of rustyline SIGWINCH panic
///
/// This demonstrates a bug where rustyline's global SIGWINCH signal handler
/// can panic when the terminal is resized after a temporary DefaultEditor
/// has been created and dropped.
///
/// To reproduce:
/// 1. Run this program: cargo run --bin rustyline_sigwinch_repro
/// 2. When prompted, resize your terminal window
/// 3. The program will panic with "fd != -1" from rustyline's signal handler
///
/// Root cause: rustyline sets up global signal handlers that outlive the
/// editor instance, and these handlers expect valid file descriptors.

use std::io::{self, Write};
use std::time::Duration;

fn main() {
    println!("=== rustyline SIGWINCH Panic Reproduction ===\n");

    // Step 1: Create a temporary DefaultEditor
    // This sets up global SIGWINCH signal handlers
    println!("Step 1: Creating temporary rustyline DefaultEditor...");
    {
        use rustyline::DefaultEditor;
        let mut rl = DefaultEditor::new().expect("Failed to create editor");

        // Use it briefly
        println!("Step 2: Getting input with rustyline...");
        print!("Enter anything (or just press Enter): ");
        io::stdout().flush().unwrap();

        match rl.readline(">>> ") {
            Ok(line) => println!("You entered: {}", line),
            Err(e) => println!("Error: {}", e),
        }

        // Editor drops here, but SIGWINCH handler persists!
    }

    println!("\nStep 3: Editor dropped, but signal handlers still active!");
    println!("Step 4: Now sleeping while waiting for input...\n");

    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║  👉 RESIZE YOUR TERMINAL WINDOW NOW                       ║");
    println!("║                                                           ║");
    println!("║  Expected result: Panic with 'fd != -1'                  ║");
    println!("║  from rustyline's SIGWINCH handler                        ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    // Simulate being in an async runtime or waiting state
    // This is similar to tokio runtime parking
    println!("Waiting 30 seconds for you to resize the window...");
    for i in 1..=30 {
        print!("\rWaiting: {} seconds... (resize window now!)", i);
        io::stdout().flush().unwrap();
        std::thread::sleep(Duration::from_secs(1));
    }

    println!("\n\nIf no panic occurred, try running again and resize faster!");
    println!("The bug is timing-dependent - the resize must happen while");
    println!("the program is in certain waiting states.");
}
