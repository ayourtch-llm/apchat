use apchat_pptx::{Presentation, Theme};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ppt = Presentation::new()
        .title("My Presentation")
        .author("My Name")
        .theme(Theme::corporate_blue());
    
    ppt.add_title_slide("Welcome", "An Example Presentation");
    
    ppt.add_content_slide("Features", vec![
        "Pure Rust implementation",
        "Theme support",
        "Easy to use API"
    ]);
    
    ppt.add_content_slide("Thanks!", vec![
        "Questions?",
        "Contact: example@email.com"
    ]);
    
    ppt.save("examples/output.pptx")?;
    println!("Created examples/output.pptx");
    
    Ok(())
}