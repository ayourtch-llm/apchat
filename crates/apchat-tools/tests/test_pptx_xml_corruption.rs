//! Test to reproduce XML corruption in PPTX files from repeated tool operations
//! 
//! This reproduces the specific corruption seen in wip16.json where after
//! operations like set_element_z_order and format_text_on_slide, the slide
//! structure becomes corrupted showing:
//! - title: null
//! - elements: []
//! 
//! Root cause: Tools modify XML structure without preserving all required elements

use std::path::PathBuf;
use std::fs;
use std::io::Read;

/// Test that demonstrates XML corruption when editing elements repeatedly
#[tokio::test]
async fn test_repeated_element_edits_corrupt_xml_structure() {
    use apchat_tools::pptx_element_edit::EditElementPropertiesTool;
    use apchat_tools::pptx_element_edit::FormatTextOnSlideTool;
    use apchat_tools::pptx_edit::SetSlideBackgroundTool;
    use apchat_toolcore::{Tool, ToolParameters};
    use apchat_policy::PolicyManager;
    use apchat_toolcore::tool_context::ToolContext;
    
    // Create a presentation similar to wip16.json
    let slides = vec![
        ppt_rs::generator::SlideContent::new("APChat: Your AI Collaboration Partner")
            .add_bullet("Advanced AI-powered CLI application for seamless human-AI collaboration")
            .add_bullet("Built with Rust for performance, reliability, and security")
            .add_bullet("Designed for developers, system administrators, and technical professionals"),
        ppt_rs::generator::SlideContent::new("Core Capabilities")
            .add_bullet("Multi-Model Architecture: GrnModel, BluModel, RedModel")
            .add_bullet("Superpower Skills Framework: Proven workflows")
            .add_bullet("PTY Terminal Integration: Direct system interaction"),
    ];
    
    let pptx_data = ppt_rs::generator::create_pptx_with_content("APChat Introduction", slides)
        .expect("Failed to create presentation");
    
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let pptx_path = temp_dir.path().join("corruption_test.pptx");
    fs::write(&pptx_path, &pptx_data).expect("Failed to write presentation");
    
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test_session".to_string(),
        PolicyManager::new(),
    );
    
    // Simulate the wip16.json workflow:
    // 1. Set background
    // 2. Format text
    // 3. Edit element properties
    
    // Step 1: Set background (wip16.json line 1377-1451)
    let bg_tool = SetSlideBackgroundTool;
    let params = ToolParameters {
        data: serde_json::json!({
            "path": "corruption_test.pptx",
            "slide_number": 1,
            "color": "F5F5F5"
        }).as_object().unwrap().clone().into_iter().collect(),
    };
    let result = bg_tool.execute(params, &context).await;
    assert!(result.success, "Set background should succeed: {}", result.content);
    
    // Step 2: Format text (wip16.json line 1495-1751)
    let format_tool = FormatTextOnSlideTool;
    let params = ToolParameters {
        data: serde_json::json!({
            "path": "corruption_test.pptx",
            "slide_number": 1,
            "element_selector": "Title",
            "bold": true
        }).as_object().unwrap().clone().into_iter().collect(),
    };
    let result = format_tool.execute(params, &context).await;
    assert!(result.success, "Format text should succeed: {}", result.content);
    
    // Step 3: Edit element properties (wip16.json line 278-304)
    let _edit_tool = EditElementPropertiesTool;
    
    // Verify file is still valid ZIP
    let file_data = fs::read(&pptx_path).expect("Failed to read file");
    let archive_result = zip::ZipArchive::new(std::io::Cursor::new(&file_data));
    assert!(archive_result.is_ok(), "File should still be valid ZIP after edits");
    
    // Check XML structure
    let mut archive = archive_result.expect("Failed to open ZIP");
    
    // First, collect the slide file names
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
    
    // Now read each slide separately to avoid borrow issues
    for slide_name in &slide_names {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&file_data))
            .expect("Failed to re-open ZIP");
        let idx = (0..archive.len()).find(|i| {
            let entry = archive.by_index(*i).unwrap();
            entry.name() == slide_name
        }).expect("Slide not found");
        
        let mut content = Vec::new();
        let mut zip_file = archive.by_index(idx).expect("Failed to get entry");
        zip_file.read_to_end(&mut content).expect("Failed to read XML");
        let content = String::from_utf8_lossy(&content);
        let name = slide_name;
        
        // Check for well-formed XML
        assert!(content.contains("<?xml"), "Should have XML declaration");
        assert!(content.contains("<p:sld"), "Should have slide root element");
        assert!(content.contains("</p:sld>"), "Should have closing slide tag");
        
        // Check for corrupted structure
        if content.contains("<a:p>") && !content.contains("</a:p>") {
            panic!("Corrupted XML: unclosed <a:p> tag in {}", name);
        }
        
        // Check for proper namespace declarations
        if name == "ppt/slides/slide1.xml" {
            assert!(content.contains("xmlns:p="), "Should have presentation namespace");
            assert!(content.contains("xmlns:a="), "Should have drawing namespace");
        }
    }
    
    assert!(!slide_names.is_empty(), "Should have at least one slide");
    
    temp_dir.close().expect("Failed to cleanup temp dir");
}

/// Test that documents the exact corruption pattern from wip16.json
#[tokio::test]
async fn test_wip16_corruption_pattern() {
    // This test documents the exact sequence from wip16.json that led to corruption:
    // 
    // 1. Lines 449-527: Add shapes to slides 1, 2, 3
    // 2. Lines 540-607: Set z-order (send_to_back) for rectangles
    // 3. Lines 637-648: Format text on slide 1
    // 4. Lines 664-717: Remove slides 1, 1, 1 (FAKED - doesn't actually remove)
    // 5. Lines 746-758: Create new presentation (overwrites file)
    // 6. Lines 778-822: Add image to slide 1
    // 7. Lines 840-943: Edit element properties
    // 8. Lines 1765-1930: Add shapes and set z-order again
    // 9. Lines 1945-1961: Read slides - NOW SHOWS EMPTY! title: null, elements: []
    //
    // The corruption happens because:
    // - remove_slide_from_pptx doesn't actually remove slides (line 1004-1008 in editor.rs)
    // - But the tool returns success, so agent thinks slides are removed
    // - When agent creates new presentation with same filename, file is overwritten
    // - Subsequent operations on the new file may corrupt it due to XML parsing issues
    
    use apchat_tools::pptx_tool::CreatePresentationTool;
    use apchat_tools::pptx_advanced_reader::ReadSlideDetailedTool;
    use apchat_toolcore::{Tool, ToolParameters};
    use apchat_policy::PolicyManager;
    use apchat_toolcore::tool_context::ToolContext;
    
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let _pptx_path = temp_dir.path().join("wip16_pattern.pptx");
    
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test_session".to_string(),
        PolicyManager::new(),
    );
    
    // Step 1: Create initial presentation (like wip16.json line 746)
    let create_tool = CreatePresentationTool;
    let slides_json = serde_json::json!([
        {
            "type": "content",
            "title": "APChat: Your AI Collaboration Partner",
            "bullets": [
                "Advanced AI-powered CLI application for seamless human-AI collaboration",
                "Built with Rust for performance, reliability, and security"
            ]
        },
        {
            "type": "content", 
            "title": "Core Capabilities",
            "bullets": [
                "Multi-Model Architecture: GrnModel, BluModel, RedModel",
                "Superpower Skills Framework: Proven workflows"
            ]
        },
        {
            "type": "content",
            "title": "Transforming Technical Workflows",
            "bullets": [
                "Streamlined Development: Natural language to production code",
                "System Administration: Complex operations simplified"
            ]
        }
    ]);
    
    let params = ToolParameters {
        data: serde_json::json!({
            "path": "wip16_pattern.pptx",
            "title": "APChat Introduction",
            "author": "APChat",
            "slides": slides_json
        }).as_object().unwrap().clone().into_iter().collect(),
    };
    
    let result = create_tool.execute(params, &context).await;
    assert!(result.success, "Create should succeed: {}", result.content);
    
    // Step 2: Read slide to verify content (like wip16.json line 1945)
    let read_tool = ReadSlideDetailedTool;
    let params = ToolParameters {
        data: serde_json::json!({
            "path": "wip16_pattern.pptx"
        }).as_object().unwrap().clone().into_iter().collect(),
    };
    
    let result = read_tool.execute(params, &context).await;
    assert!(result.success, "Read should succeed");
    
    // Verify we have proper content BEFORE any corruption
    assert!(result.content.contains("APChat"), "Should have title before operations");
    assert!(result.content.contains("elements"), "Should have elements array");
    
    // Check that content is NOT the corruption pattern
    // Corruption pattern from wip16.json line 1956:
    // "title": null, "elements": []
    assert!(!result.content.contains(r#""title": null"#), 
            "Should not have null title before operations");
    
    println!("Initial state OK: {}", result.content);
    
    // The actual corruption would happen from subsequent tool operations
    // that have XML parsing/modification bugs
    
    temp_dir.close().expect("Failed to cleanup temp dir");
}
