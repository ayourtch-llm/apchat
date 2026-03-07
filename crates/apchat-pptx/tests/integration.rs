use apchat_pptx::{Presentation, Theme};
use std::fs;

#[test]
fn test_full_presentation_workflow() {
    let mut ppt = Presentation::new()
        .title("Q3 Executive Review")
        .author("APChat AI")
        .theme(Theme::corporate_blue());
    
    ppt.add_title_slide("Strategic Insights", "Q3 2024");
    ppt.add_content_slide("Key Metrics", vec![
        "Revenue: +23% YoY",
        "User growth: 1.2M new users",
        "NPS: 72 (up 8 points)"
    ]);
    ppt.add_content_slide("Market Analysis", vec![
        "Market share: 34%",
        "Growth rate: 15% CAGR",
        "Competitive advantage maintained"
    ]);
    
    let path = "/tmp/integration_test.pptx";
    ppt.save(path).unwrap();
    
    let metadata = fs::metadata(path).unwrap();
    assert!(metadata.len() > 0);
    
    // Verify it's a valid PPTX (ZIP)
    let file_type = std::process::Command::new("file")
        .arg(path)
        .output()
        .unwrap();
    let output = String::from_utf8_lossy(&file_type.stdout);
    assert!(output.contains("PowerPoint") || output.contains("ZIP"));
    
    fs::remove_file(path).ok();
}