// Test startup to verify readline history loads without errors
use apchat_vty::history::load_and_add_to_editor;
use apchat_vty::Readline;

fn main() {
    println!("Testing readline history loading on startup...");
    println!();

    let mut rl = Readline::new().expect("Failed to create readline");

    match load_and_add_to_editor(&mut rl) {
        Ok(_) => {
            println!("✅ Successfully loaded readline history!");
            println!("   The application will now start without errors.");
        }
        Err(e) => {
            eprintln!("❌ Failed to load readline history: {}", e);
            std::process::exit(1);
        }
    }
}
