//! Test to reproduce PPTX corruption from remove_slide_from_pptx tool
//! 
//! This test reproduces the issue discovered in wip16.json workflow where:
//! 1. remove_slide_from_pptx tool is called but doesn't actually remove slides
//! 2. Subsequent operations corrupt the file structure
//! 3. Reading slides shows empty/null content

use std::path::PathBuf;
use std::fs;
use std::io::Read;
use apchat_tools::pptx_edit::RemoveSlideFromPptxTool;
use apchat_toolcore::{Tool, ToolParameters};
use apchat_policy::PolicyManager;
use apchat_toolcore::tool_context::ToolContext;

#[tokio::test]
async fn test_remove_slide_placeholder_causes_corruption() {
    // Create a test presentation
    let slides = vec![
        ppt_rs::generator::SlideContent::new("Slide 1")
            .add_bullet("Content 1"),
        ppt_rs::generator::SlideContent::new("Slide 2")
            .add_bullet("Content 2"),
        ppt_rs::generator::SlideContent::new("Slide 3")
            .add_bullet("Content 3"),
    ];
    
    let pptx_data = ppt_rs::generator::create_pptx_with_content("Test Presentation", slides)
        .expect("Failed to create presentation");
    
    // Save to temp file
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let pptx_path = temp_dir.path().join("test_remove.pptx");
    fs::write(&pptx_path, &pptx_data).expect("Failed to write presentation");
    
    // Verify initial state - should have 3 slides
    let initial_slides = read_slides_count(&pptx_path);
    assert_eq!(initial_slides, 3, "Should start with 3 slides");
    
    // Create tool context
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test_session".to_string(),
        PolicyManager::new(),
    );
    
    // Call remove_slide_from_pptx - this is the problematic call
    let tool = RemoveSlideFromPptxTool;
    let params = ToolParameters {
        data: serde_json::json!({
            "path": "test_remove.pptx",
            "slide_number": 1
        }).as_object().unwrap().clone().into_iter().collect(),
    };
    
    let result = tool.execute(params, &context).await;
    
    // Tool reports success (THIS IS THE BUG - it's a placeholder!)
    assert!(result.success, "Tool should report success: {}", result.content);
    
    // BUT the slide was NOT actually removed - file still has 3 slides
    let slides_after = read_slides_count(&pptx_path);
    assert_eq!(slides_after, 3, "Slide was NOT removed - tool is just a placeholder!");
    
    // This is the corruption scenario from wip16.json:
    // Agent thinks slide was removed, calls it again on "new" slide 1
    // But the file structure gets corrupted
    
    temp_dir.close().expect("Failed to cleanup temp dir");
}

#[tokio::test]
async fn test_multiple_remove_calls_corrupt_file_structure() {
    // This test reproduces the exact corruption pattern from wip16.json
    
    // Create initial presentation
    let slides = vec![
        ppt_rs::generator::SlideContent::new("Slide 1")
            .add_bullet("Content 1"),
        ppt_rs::generator::SlideContent::new("Slide 2")
            .add_bullet("Content 2"),
        ppt_rs::generator::SlideContent::new("Slide 3")
            .add_bullet("Content 3"),
    ];
    
    let pptx_data = ppt_rs::generator::create_pptx_with_content("Test", slides)
        .expect("Failed to create presentation");
    
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let pptx_path = temp_dir.path().join("corrupt_test.pptx");
    fs::write(&pptx_path, &pptx_data).expect("Failed to write presentation");
    
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test_session".to_string(),
        PolicyManager::new(),
    );
    
    let tool = RemoveSlideFromPptxTool;
    
    // Simulate the wip16.json pattern: agent calls remove 3 times
    // thinking it's removing slides 1, 1, 1 (renumbered each time)
    for i in 0..3 {
        let params = ToolParameters {
            data: serde_json::json!({
                "path": "corrupt_test.pptx",
                "slide_number": 1
            }).as_object().unwrap().clone().into_iter().collect(),
        };
        
        let result = tool.execute(params, &context).await;
        assert!(result.success, "Remove {} should succeed: {}", i, result.content);
    }
    
    // File still has all 3 slides! Agent now thinks file is empty
    // but when it reads the file, it might see corrupted structure
    let slides_after = read_slides_count(&pptx_path);
    assert_eq!(slides_after, 3, "All slides still present - no actual removal happened");
    
    // Read the file to check for corruption
    let file_data = fs::read(&pptx_path).expect("Failed to read file");
    
    // Check if slide XML files exist and are valid
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&file_data))
        .expect("Failed to open ZIP");
    
    let slide_names: Vec<String> = (0..archive.len())
        .filter_map(|i| {
            let entry = archive.by_index(i).ok()?;
            let name = entry.name();
            if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    
    let mut slide_files_found = 0;
    for slide_name in &slide_names {
        slide_files_found += 1;
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&file_data))
            .expect("Failed to open ZIP");
        let idx = (0..archive.len()).find(|i| {
            let entry = archive.by_index(*i).unwrap();
            entry.name() == slide_name
        }).unwrap();
        
        // Read the slide XML
        let mut content = Vec::new();
        let mut zip_file = archive.by_index(idx).expect("Failed to get entry");
        zip_file.read_to_end(&mut content).expect("Failed to read slide XML");
        let content = String::from_utf8_lossy(&content);
        
        // Check for corruption markers
        assert!(!content.is_empty(), "Slide XML should not be empty");
        assert!(content.contains("</p:sld>"), "Slide XML should have closing tag");
    }
    
    assert_eq!(slide_files_found, 3, "Should have 3 slide XML files");
    
    temp_dir.close().expect("Failed to cleanup temp dir");
}

#[tokio::test]
async fn test_read_after_fake_remove_shows_data_loss() {
    // Reproduces wip16.json lines 1951-1961 where after "removing" slides,
    // read_slide_detailed returns null titles and empty elements
    
    use apchat_tools::pptx_advanced_reader::ReadSlideDetailedTool;
    
    // Create presentation with specific content
    let slides = vec![
        ppt_rs::generator::SlideContent::new("APChat: Your AI Collaboration Partner")
            .add_bullet("Advanced AI-powered CLI application"),
        ppt_rs::generator::SlideContent::new("Core Capabilities")
            .add_bullet("Multi-Model Architecture"),
        ppt_rs::generator::SlideContent::new("Transforming Technical Workflows")
            .add_bullet("Streamlined Development"),
    ];
    
    let pptx_data = ppt_rs::generator::create_pptx_with_content("APChat Introduction", slides)
        .expect("Failed to create presentation");
    
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let pptx_path = temp_dir.path().join("apchat_test.pptx");
    fs::write(&pptx_path, &pptx_data).expect("Failed to write presentation");
    
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test_session".to_string(),
        PolicyManager::new(),
    );
    
    // Read initial state
    let read_tool = ReadSlideDetailedTool;
    let params = ToolParameters {
        data: serde_json::json!({
            "path": "apchat_test.pptx",
            "slide_number": 1
        }).as_object().unwrap().clone().into_iter().collect(),
    };
    
    let result = read_tool.execute(params, &context).await;
    assert!(result.success);
    assert!(result.content.contains("APChat"), "Should have title before removal");
    assert!(result.content.contains("Advanced AI-powered"), "Should have bullets before removal");
    
    // Fake remove slide 1
    let remove_tool = RemoveSlideFromPptxTool;
    let params = ToolParameters {
        data: serde_json::json!({
            "path": "apchat_test.pptx",
            "slide_number": 1
        }).as_object().unwrap().clone().into_iter().collect(),
    };
    
    let result = remove_tool.execute(params, &context).await;
    assert!(result.success);
    
    // Try to read again - this might show corruption
    // In wip16.json, after fake removals, reads showed:
    // "title": null, "elements": []
    let params = ToolParameters {
        data: serde_json::json!({
            "path": "apchat_test.pptx",
            "slide_number": 1
        }).as_object().unwrap().clone().into_iter().collect(),
    };
    
    let result = read_tool.execute(params, &context).await;
    
    // The corruption pattern: file is modified in ways that break structure
    // but we can't detect it without actual implementation
    // This test documents the expected failure mode
    println!("Read after fake remove: {}", result.content);
    
    temp_dir.close().expect("Failed to cleanup temp dir");
}

/// Helper function to count slides in a PPTX file
fn read_slides_count(pptx_path: &PathBuf) -> usize {
    let file_data = fs::read(pptx_path).expect("Failed to read PPTX");
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&file_data))
        .expect("Failed to open ZIP");
    
    let mut count = 0;
    for i in 0..archive.len() {
        let entry = archive.by_index(i).expect("Failed to get entry");
        let name = entry.name();
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
            count += 1;
        }
    }
    count
}
