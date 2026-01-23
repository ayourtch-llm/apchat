// Test to verify the corrupted history recovery works
use apchat_vty::history::load_history;

fn main() {
    println!("Testing readline history loading with corrupted data...");
    println!();
    
    match load_history(None) {
        Ok(history) => {
            println!("✅ Successfully loaded history!");
            println!("   Total entries: {}", history.len());
            println!("   Entries: {:?}", history.get_entries().iter().map(|e| &e.command).collect::<Vec<_>>());
        }
        Err(e) => {
            eprintln!("❌ Failed to load history: {}", e);
            std::process::exit(1);
        }
    }
}
