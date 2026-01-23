// Test to see if crossterm supports Event::Paste and how to enable bracketed paste mode
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

fn main() -> std::io::Result<()> {
    println!("Testing crossterm paste event support...");
    println!("Press any key or paste text (Ctrl-D to exit)");
    println!("Note: Bracketed paste mode may need to be enabled");
    println!();

    loop {
        if event::poll(Duration::from_millis(100))? {
            let event = event::read()?;
            match event {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('d'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                }) => {
                    println!("Received Ctrl-D, exiting...");
                    break;
                }
                Event::Key(key) => {
                    println!("Key event: {:?}", key);
                }
                Event::Paste(content) => {
                    println!("Paste event detected!");
                    println!("Content length: {} bytes", content.len());
                    println!("Content: {:?}", content);
                    println!("---");
                }
                Event::Mouse(mouse) => {
                    println!("Mouse event: {:?}", mouse);
                }
                Event::Resize(width, height) => {
                    println!("Resize: {}x{}", width, height);
                }
                Event::FocusGained => {
                    println!("Focus gained");
                }
                Event::FocusLost => {
                    println!("Focus lost");
                }
            }
        }
    }

    Ok(())
}
