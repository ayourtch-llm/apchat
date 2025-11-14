/// More accurate rustyline SIGWINCH panic reproduction with tokio
///
/// This better simulates the actual kimichat scenario where:
/// - Tool confirmations create temporary DefaultEditor instances
/// - The main thread is in tokio's runtime parking/waiting
/// - Window resize triggers SIGWINCH while in tokio's condvar wait
///
/// To reproduce:
/// 1. Run: cargo run --bin rustyline_sigwinch_tokio_repro
/// 2. Answer 'y' to the first prompt
/// 3. Resize terminal window during the async sleep
/// 4. Program panics with "fd != -1"

use std::io::{self, Write};
use std::time::Duration;
use tokio::time::sleep;

/// Simulates a tool confirmation prompt using temporary rustyline editor
fn get_user_confirmation() -> bool {
    use rustyline::DefaultEditor;

    println!("\n🔧 Tool wants to execute a command");
    print!("Execute? (y/N): ");
    io::stdout().flush().unwrap();

    // Create temporary editor - sets up SIGWINCH handlers
    let mut rl = DefaultEditor::new().expect("Failed to create editor");

    match rl.readline(">>> ") {
        Ok(response) => {
            let response = response.trim().to_lowercase();
            response == "y" || response == "yes"
        }
        Err(_) => false,
    }
    // Editor dropped here, but signal handlers persist!
}

/// Simulates agent execution in tokio runtime
async fn simulate_agent_execution() {
    println!("\n🤖 Agent is executing...");
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║  👉 RESIZE YOUR TERMINAL WINDOW NOW                       ║");
    println!("║                                                           ║");
    println!("║  Expected: Panic from rustyline's SIGWINCH handler       ║");
    println!("║  Message: 'fd != -1' at unix.rs:1197                     ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    for i in 1..=20 {
        print!("\r⏱️  Agent working... {} seconds (resize window now!)", i);
        io::stdout().flush().unwrap();
        sleep(Duration::from_secs(1)).await;
    }

    println!("\n✅ Agent finished");
}

#[tokio::main]
async fn main() {
    println!("=== rustyline SIGWINCH + Tokio Panic Reproduction ===\n");
    println!("This simulates the exact kimichat scenario:\n");
    println!("1. Tool confirmation creates temporary rustyline editor");
    println!("2. Editor is dropped but SIGWINCH handlers persist");
    println!("3. Main thread enters tokio runtime parking");
    println!("4. Window resize triggers orphaned signal handler");
    println!("5. Handler tries to use invalid fd -> PANIC\n");

    // Step 1: Simulate tool confirmation (creates + drops rustyline)
    if !get_user_confirmation() {
        println!("Cancelled by user");
        return;
    }

    println!("\n✓ Confirmation received");
    println!("✓ rustyline editor dropped (but SIGWINCH handler still active!)");

    // Step 2: Enter tokio async work (runtime will park waiting for events)
    // This is when SIGWINCH will hit the orphaned handler
    simulate_agent_execution().await;

    println!("\nIf no panic occurred:");
    println!("- Try running again and resize during the countdown");
    println!("- The bug is timing-dependent");
    println!("- Must resize while tokio runtime is parked/waiting");
}
