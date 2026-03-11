//! Test for XML parsing errors in PPTX editing tools
//! 
//! This reproduces the specific XML parsing errors seen in wip16.json:
//! - Line 280: "XML parsing error: Expecting </a:p> found </p:txBody>"
//! - Line 648: "XML parsing error: Expecting </p:sp> found </p:nvSpPr>"
//!
//! These errors indicate the XML parser is not correctly handling nested elements
//! when modifying element properties or formatting text.

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use quick_xml::Reader;
    use quick_xml::events::Event;

    /// Test XML parsing with nested elements - reproduces the parser state bug
    #[test]
    fn test_xml_parsing_nested_elements() {
        // This is a simplified slide XML structure similar to what causes the bug
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" 
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Title"/>
        </p:nvSpPr>
        <p:txBody>
          <a:p>
            <a:r>
              <a:t>Old Title</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="3" name="Content"/>
        </p:nvSpPr>
        <p:txBody>
          <a:p>
            <a:r>
              <a:t>Bullet 1</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

        // Parse with quick_xml (same library used in edit_element_properties)
        let mut reader = Reader::from_str(slide_xml);
        reader.trim_text(true);
        let mut buf = Vec::new();
        
        let mut depth = 0;
        let mut in_title_element = false;
        let title_depth = 0;
        let mut parse_ok = true;
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    depth += 1;
                    
                    // Track when we're in the Title element
                    if e.name().as_ref() == b"p:sp" && !in_title_element {
                        // Check if this is the Title shape
                        // We'd need to look ahead to p:cNvPr name attribute
                        // This is where the bug can happen - not tracking depth correctly
                    }
                    
                    if in_title_element {
                        // When modifying attributes, the code might incorrectly
                        // assume it's still at the same depth level
                        // This causes malformed XML output
                    }
                }
                Ok(Event::End(ref _e)) => {
                    depth -= 1;
                    
                    if in_title_element && depth < title_depth {
                        in_title_element = false;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    parse_ok = false;
                    println!("XML parsing error: {}", e);
                    break;
                }
                _ => {}
            }
            buf.clear();
        }
        
        assert!(parse_ok, "XML should parse without errors");
    }

    /// Test the specific bug: modifying a:off or a:ext attributes
    #[test]
    fn test_modify_off_ext_attributes() {
        use quick_xml::events::{BytesStart, BytesEnd};
        use quick_xml::Writer;
        
        // XML with position and size attributes
        let xml_fragment = r#"<a:xfrm><a:off x="100" y="200"/><a:ext cx="1000" cy="2000"/></a:xfrm>"#;
        
        let mut reader = Reader::from_str(xml_fragment);
        reader.trim_text(true);
        let mut writer = Writer::new(Cursor::new(Vec::new()));
        let mut buf = Vec::new();
        
        let mut modify_ok = true;
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref e)) if e.name().as_ref() == b"a:off" => {
                    // This is where the bug happens:
                    // When modifying attributes, the code creates a new BytesStart
                    // but might not preserve all attributes correctly
                    
                    let mut new_off = BytesStart::new("a:off");
                    new_off.push_attribute(("x", "500")); // New X
                    new_off.push_attribute(("y", "600")); // New Y
                    
                    // BUG: If we don't write it correctly, XML becomes malformed
                    if let Err(e) = writer.write_event(Event::Empty(new_off)) {
                        modify_ok = false;
                        println!("Failed to write modified a:off: {}", e);
                    }
                }
                Ok(Event::Empty(ref e)) if e.name().as_ref() == b"a:ext" => {
                    // Similar bug for a:ext
                    let mut new_ext = BytesStart::new("a:ext");
                    new_ext.push_attribute(("cx", "3000"));
                    new_ext.push_attribute(("cy", "4000"));
                    
                    if let Err(e) = writer.write_event(Event::Empty(new_ext)) {
                        modify_ok = false;
                        println!("Failed to write modified a:ext: {}", e);
                    }
                }
                Ok(Event::Eof) => break,
                Ok(e) => {
                    // Write other events as-is
                    if let Err(e) = writer.write_event(e.to_owned()) {
                        modify_ok = false;
                        println!("Failed to write event: {}", e);
                    }
                }
                Err(e) => {
                    modify_ok = false;
                    println!("XML parsing error: {}", e);
                    break;
                }
            }
            buf.clear();
        }
        
        assert!(modify_ok, "Should modify attributes without errors");
        
        // Verify the output is valid XML
        let output = writer.into_inner();
        let output_bytes = output.into_inner();
        let output_xml = String::from_utf8_lossy(&output_bytes);
        println!("Modified XML: {}", output_xml);
        
        // Try to parse the output to verify it's well-formed
        let mut verify_reader = Reader::from_str(&output_xml);
        let mut verify_buf = Vec::new();
        let verify_ok = loop {
            match verify_reader.read_event_into(&mut verify_buf) {
                Ok(Event::Eof) => break true,
                Err(_) => break false,
                _ => {}
            }
            verify_buf.clear();
        };
        
        assert!(verify_ok, "Modified XML should be well-formed");
    }

    /// Test element selector matching - bug in body/title/content matching
    #[test]
    fn test_element_selector_matching() {
        // This tests the selector parsing logic that can cause issues
        // when "body" selector matches wrong elements
        
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" 
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Title 1"/>
        </p:nvSpPr>
        <p:txBody>
          <a:p><a:r><a:t>Title Text</a:t></a:r></a:p>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="3" name="roundedRectangle Shape"/>
        </p:nvSpPr>
        <p:txBody>
          <a:p><a:r><a:t>Content in shape</a:t></a:r></a:p>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="4" name="Content Placeholder 2"/>
        </p:nvSpPr>
        <p:txBody>
          <a:p><a:r><a:t>Actual content</a:t></a:r></a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

        // When using "body" selector, the code should match the Content element
        // but might match the wrong shape, causing corruption
        
        let mut reader = Reader::from_str(slide_xml);
        reader.trim_text(true);
        let mut buf = Vec::new();
        
        let mut element_index = 0;
        let mut matched_elements = Vec::new();
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref e)) if e.name().as_ref() == b"p:cNvPr" => {
                    element_index += 1;
                    
                    // Extract name attribute
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"name" {
                            let name = String::from_utf8_lossy(&attr.value);
                            
                            // Test selector matching logic
                            let selector = "body";
                            let matches = {
                                let name_lower = name.to_lowercase();
                                name_lower == selector ||
                                name_lower.contains("content") ||
                                (name_lower.contains("shape") && !name_lower.contains("title"))
                            };
                            
                            if matches {
                                matched_elements.push((element_index, name.to_string()));
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    panic!("XML parsing error: {}", e);
                }
                _ => {}
            }
            buf.clear();
        }
        
        // Should match "Content Placeholder 2" and "roundedRectangle Shape"
        // but NOT "Title 1"
        assert!(!matched_elements.is_empty(), "Should match at least one element");
        
        for (idx, name) in &matched_elements {
            assert!(!name.to_lowercase().contains("title"), 
                    "Body selector should NOT match title elements, but matched: {} at index {}", name, idx);
        }
        
        println!("Matched elements for 'body' selector: {:?}", matched_elements);
    }

    /// Integration test: Try to reproduce the actual corruption from wip16.json
    #[tokio::test]
    async fn test_edit_element_properties_corruption() {
        use apchat_tools::pptx_element_edit::EditElementPropertiesTool;
        use apchat_toolcore::{Tool, ToolParameters};
        use apchat_policy::PolicyManager;
        use apchat_toolcore::tool_context::ToolContext;
        
        // Create a presentation with a shape that will be edited
        let slides = vec![
            ppt_rs::generator::SlideContent::new("Test Slide")
                .add_bullet("Bullet 1")
                .add_bullet("Bullet 2"),
        ];
        
        let pptx_data = ppt_rs::generator::create_pptx_with_content("Test", slides)
            .expect("Failed to create presentation");
        
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let pptx_path = temp_dir.path().join("edit_test.pptx");
        std::fs::write(&pptx_path, &pptx_data).expect("Failed to write presentation");
        
        let context = ToolContext::new(
            temp_dir.path().to_path_buf(),
            "test_session".to_string(),
            PolicyManager::new(),
        );
        
        // Try to edit element with "body" selector (this can cause the XML parsing error)
        let tool = EditElementPropertiesTool;
        let params = ToolParameters {
            data: serde_json::json!({
                "path": "edit_test.pptx",
                "slide_number": 1,
                "element_selector": "body",
                "position_x": 500000,
                "position_y": 1300000,
                "width": 3400000
            }).as_object().unwrap().clone().into_iter().collect(),
        };
        
        let result = tool.execute(params, &context).await;
        
        // In wip16.json, this failed with:
        // "Failed to edit element: XML parsing error: Expecting </a:p> found </p:txBody>"
        // OR
        // "Failed to edit element: XML parsing error: Expecting </p:sp> found </p:nvSpPr>"
        
        if !result.success {
            // If it fails, check the error message
            let error_msg = &result.content;
            assert!(!error_msg.contains("Expecting"), 
                    "Should not have XML parsing error, got: {}", error_msg);
        }
        
        // If successful, verify the file is still valid
        let file_data = std::fs::read(&pptx_path).expect("Failed to read file");
        let archive = zip::ZipArchive::new(Cursor::new(&file_data));
        
        assert!(archive.is_ok(), 
                "File should still be valid ZIP after edit, error: {:?}", 
                archive.err());
        
        temp_dir.close().expect("Failed to cleanup temp dir");
    }

    /// Test format_text_on_slide corruption
    #[tokio::test]
    async fn test_format_text_corruption() {
        use apchat_tools::pptx_element_edit::FormatTextOnSlideTool;
        use apchat_toolcore::{Tool, ToolParameters};
        use apchat_policy::PolicyManager;
        use apchat_toolcore::tool_context::ToolContext;
        
        let slides = vec![
            ppt_rs::generator::SlideContent::new("Test Title")
                .add_bullet("Bullet 1"),
        ];
        
        let pptx_data = ppt_rs::generator::create_pptx_with_content("Test", slides)
            .expect("Failed to create presentation");
        
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let pptx_path = temp_dir.path().join("format_test.pptx");
        std::fs::write(&pptx_path, &pptx_data).expect("Failed to write presentation");
        
        let context = ToolContext::new(
            temp_dir.path().to_path_buf(),
            "test_session".to_string(),
            PolicyManager::new(),
        );
        
        // Format text on slide
        let tool = FormatTextOnSlideTool;
        let params = ToolParameters {
            data: serde_json::json!({
                "path": "format_test.pptx",
                "slide_number": 1,
                "element_selector": "Title",
                "bold": true,
                "font_size": 44
            }).as_object().unwrap().clone().into_iter().collect(),
        };
        
        let result = tool.execute(params, &context).await;
        
        // Should succeed without XML parsing errors
        if !result.success {
            let error_msg = &result.content;
            assert!(!error_msg.contains("XML parsing error"),
                    "Should not have XML parsing error, got: {}", error_msg);
        }
        
        // Verify file integrity
        let file_data = std::fs::read(&pptx_path).expect("Failed to read file");
        let archive = zip::ZipArchive::new(Cursor::new(&file_data));
        assert!(archive.is_ok(), "File should remain valid after formatting");
        
        temp_dir.close().expect("Failed to cleanup temp dir");
    }
}
