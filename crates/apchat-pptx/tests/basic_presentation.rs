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