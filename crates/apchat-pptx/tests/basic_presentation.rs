use apchat_pptx::{Presentation, Theme};

#[test]
fn test_presentation_builder() {
    let ppt = Presentation::new()
        .title("Test Presentation")
        .author("Test Author")
        .theme(Theme::corporate_blue());
    
    assert_eq!(ppt.get_title(), Some("Test Presentation"));
    assert_eq!(ppt.get_author(), Some("Test Author"));
}

#[test]
fn test_add_title_slide() {
    let mut ppt = Presentation::new();
    ppt.add_title_slide("Main Title", "Subtitle");
    assert_eq!(ppt.slides_count(), 1);
}

#[test]
fn test_add_content_slide() {
    let mut ppt = Presentation::new();
    ppt.add_content_slide("Slide Title", vec!["Bullet 1", "Bullet 2"]);
    assert_eq!(ppt.slides_count(), 1);
}

#[test]
fn test_save_presentation() {
    let mut ppt = Presentation::new()
        .title("Test")
        .author("Test Author");
    
    ppt.add_title_slide("Title", "Subtitle");
    ppt.add_content_slide("Content", vec!["Bullet 1", "Bullet 2"]);
    
    let path = "/tmp/test_presentation.pptx";
    ppt.save(path).unwrap();
    
    assert!(std::fs::metadata(path).is_ok());
    
    // Cleanup
    std::fs::remove_file(path).ok();
}