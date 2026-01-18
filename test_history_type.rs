use rustyline::Editor;
use rustyline::history::FileHistory;

fn main() {
    let mut rl = Editor::<(), FileHistory>::new().unwrap();
    
    // Check what history() returns
    let h = rl.history();
    println!("Type of h: {:?}", std::any::type_name_of_val(&h));
    
    // Try to use it
    if let Some(history) = h {
        println!("Got Some(history)");
        println!("Type of history: {:?}", std::any::type_name_of_val(&history));
    } else {
        println!("Got None");
    }
}