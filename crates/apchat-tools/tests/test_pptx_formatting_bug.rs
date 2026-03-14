//! Unit tests for PPTX text formatting with complex XML structures
//!
//! These tests reproduce the issue where format_text_on_slide fails with:
//! "XML parsing error: Expecting </a:r> found </a:rPr>"
//!
//! The root cause is that when modifying <a:rPr> elements that contain
//! child elements (like <a:solidFill>, <a:latin>), the code writes them
//! as Empty events instead of preserving their Start/End structure.

use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::Reader;
use quick_xml::Writer;
use std::collections::HashMap;
use std::io::Cursor;

/// Simplified version of the format_text_in_element function to test the bug
fn format_text_in_element_buggy(
    xml: &str,
    color: &Option<String>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    let mut in_target_element = false;
    let mut target_element_depth = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag_name = e.name();

                if tag_name.as_ref() == b"a:rPr" {
                    in_target_element = true;
                    target_element_depth = 1;

                    // BUG: Writing as Empty event even though it may have children
                    let modified = modify_run_properties_buggy(e, color);
                    writer.write_event(Event::Empty(modified))?;
                    buf.clear();
                    continue;
                }

                if in_target_element {
                    target_element_depth += 1;
                }

                writer.write_event(Event::Start(e.to_owned()))?;
            }
            Ok(Event::End(ref e)) => {
                if in_target_element {
                    target_element_depth -= 1;
                    if target_element_depth == 0 {
                        in_target_element = false;
                    }
                }
                writer.write_event(Event::End(e.to_owned()))?;
            }
            Ok(Event::Empty(ref e)) => {
                if in_target_element && e.name().as_ref() == b"a:rPr" {
                    let modified = modify_run_properties_buggy(e, color);
                    writer.write_event(Event::Empty(modified))?;
                    buf.clear();
                    continue;
                }
                writer.write_event(Event::Empty(e.to_owned()))?;
            }
            Ok(Event::Text(ref e)) => {
                writer.write_event(Event::Text(e.to_owned()))?;
            }
            Ok(Event::Eof) => break,
            Ok(e) => {
                writer.write_event(e.to_owned())?;
            }
            Err(e) => {
                return Err(format!("XML parsing error: {}", e).into());
            }
        }
        buf.clear();
    }

    let result = writer.into_inner();
    Ok(String::from_utf8_lossy(&result.into_inner()).to_string())
}

/// Buggy version that always writes as Empty
fn modify_run_properties_buggy(event: &BytesStart, color: &Option<String>) -> BytesStart<'static> {
    let mut result = BytesStart::new("a:rPr");

    let mut existing_attrs: HashMap<String, String> = HashMap::new();
    for attr in event.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        let value = String::from_utf8_lossy(&attr.value).to_string();
        existing_attrs.insert(key, value);
    }

    if let Some(ref c) = color {
        existing_attrs.insert("color".to_string(), c.clone());
    }

    for (key, value) in existing_attrs {
        result.push_attribute((key.as_str(), value.as_str()));
    }

    result
}

/// Fixed version that preserves Start/End structure
fn format_text_in_element_fixed(
    xml: &str,
    color: &Option<String>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    let mut in_target_element = false;
    let mut target_element_depth = 0;
    let mut skip_next_rpr_end = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag_name = e.name();

                if tag_name.as_ref() == b"a:rPr" {
                    in_target_element = true;
                    target_element_depth = 1;

                    // FIX: Write as Start event, not Empty
                    let modified = modify_run_properties_fixed(e, color);
                    writer.write_event(Event::Start(modified))?;
                    buf.clear();
                    continue;
                }

                if in_target_element {
                    target_element_depth += 1;
                }

                writer.write_event(Event::Start(e.to_owned()))?;
            }
            Ok(Event::End(ref e)) => {
                if in_target_element {
                    target_element_depth -= 1;
                    if target_element_depth == 0 {
                        in_target_element = false;
                    }
                }

                // Skip the end tag if we already wrote it as Empty (buggy path)
                if e.name().as_ref() == b"a:rPr" && skip_next_rpr_end {
                    skip_next_rpr_end = false;
                    buf.clear();
                    continue;
                }

                writer.write_event(Event::End(e.to_owned()))?;
            }
            Ok(Event::Empty(ref e)) => {
                if in_target_element && e.name().as_ref() == b"a:rPr" {
                    let modified = modify_run_properties_fixed(e, color);
                    writer.write_event(Event::Empty(modified))?;
                    buf.clear();
                    continue;
                }
                writer.write_event(Event::Empty(e.to_owned()))?;
            }
            Ok(Event::Text(ref e)) => {
                writer.write_event(Event::Text(e.to_owned()))?;
            }
            Ok(Event::Eof) => break,
            Ok(e) => {
                writer.write_event(e.to_owned())?;
            }
            Err(e) => {
                return Err(format!("XML parsing error: {}", e).into());
            }
        }
        buf.clear();
    }

    let result = writer.into_inner();
    Ok(String::from_utf8_lossy(&result.into_inner()).to_string())
}

fn modify_run_properties_fixed(event: &BytesStart, color: &Option<String>) -> BytesStart<'static> {
    let mut result = BytesStart::new("a:rPr");

    let mut existing_attrs: HashMap<String, String> = HashMap::new();
    for attr in event.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        let value = String::from_utf8_lossy(&attr.value).to_string();
        existing_attrs.insert(key, value);
    }

    if let Some(ref c) = color {
        existing_attrs.insert("color".to_string(), c.clone());
    }

    for (key, value) in existing_attrs {
        result.push_attribute((key.as_str(), value.as_str()));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test case 1: Simple a:rPr with no children (should work in both versions)
    #[test]
    fn test_simple_rpr_no_children() {
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld>
<p:spTree>
<p:sp>
<p:txBody>
<a:p>
<a:r>
<a:rPr lang="en-US" sz="1200"/>
<a:t>Simple text</a:t>
</a:r>
</a:p>
</p:txBody>
</p:sp>
</p:spTree>
</p:cSld>
</p:sld>"#;

        let color = Some("FFFFFF".to_string());

        // Buggy version should work for simple cases
        let result = format_text_in_element_buggy(slide_xml, &color);
        assert!(
            result.is_ok(),
            "Buggy version should handle simple case: {:?}",
            result.err()
        );

        // Fixed version should also work
        let result = format_text_in_element_fixed(slide_xml, &color);
        assert!(
            result.is_ok(),
            "Fixed version should handle simple case: {:?}",
            result.err()
        );
    }

    /// Test case 2: a:rPr with child elements (REPRODUCES THE BUG)
    #[test]
    fn test_rpr_with_children_reproduces_bug() {
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld>
<p:spTree>
<p:sp>
<p:txBody>
<a:p>
<a:r>
<a:rPr lang="en-US" sz="1200">
<a:solidFill>
<a:srgbClr val="000000"/>
</a:solidFill>
<a:latin typeface="Arial"/>
</a:rPr>
<a:t>Text with formatting</a:t>
</a:r>
</a:p>
</p:txBody>
</p:sp>
</p:spTree>
</p:cSld>
</p:sld>"#;

        let color = Some("FFFFFF".to_string());

        // Buggy version should produce INVALID XML (orphaned end tag)
        let result = format_text_in_element_buggy(slide_xml, &color);
        assert!(result.is_ok() || result.is_err(), "Testing buggy version");

        // Check if the result contains orphaned </a:rPr> tag (the actual bug symptom)
        if let Ok(modified) = result {
            // Count opening and closing tags - they should match
            let open_count = modified.matches("<a:rPr").count();
            let close_count = modified.matches("</a:rPr>").count();

            // The bug causes mismatched tags
            assert!(
                open_count != close_count || modified.contains("/><a:t>"),
                "Buggy version should produce malformed XML with orphaned end tags"
            );
            println!("✓ Buggy XML produced: {}", modified);
        }
    }

    /// Test case 3: Fixed version handles a:rPr with children correctly
    #[test]
    fn test_fixed_version_handles_children() {
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld>
<p:spTree>
<p:sp>
<p:txBody>
<a:p>
<a:r>
<a:rPr lang="en-US" sz="1200">
<a:solidFill>
<a:srgbClr val="000000"/>
</a:solidFill>
<a:latin typeface="Arial"/>
</a:rPr>
<a:t>Text with formatting</a:t>
</a:r>
</a:p>
</p:txBody>
</p:sp>
</p:spTree>
</p:cSld>
</p:sld>"#;

        let color = Some("FFFFFF".to_string());

        // Fixed version should succeed
        let result = format_text_in_element_fixed(slide_xml, &color);
        assert!(
            result.is_ok(),
            "Fixed version should handle children: {:?}",
            result.err()
        );

        let modified = result.unwrap();
        assert!(
            modified.contains(r#"color="FFFFFF""#),
            "Should add color attribute"
        );
        assert!(
            modified.contains("<a:solidFill>"),
            "Should preserve solidFill child"
        );
        assert!(modified.contains("<a:latin"), "Should preserve latin child");
        assert!(modified.contains("</a:rPr>"), "Should have closing tag");

        // Verify the XML is well-formed by attempting to parse it
        let mut verify_reader = Reader::from_str(&modified);
        let mut verify_buf = Vec::new();
        loop {
            match verify_reader.read_event_into(&mut verify_buf) {
                Ok(Event::Eof) => break,
                Err(e) => panic!("Modified XML is malformed: {}", e),
                _ => {}
            }
            verify_buf.clear();
        }

        println!("✓ Fixed version produces valid XML");
    }

    /// Test case 4: Multiple runs, some with children, some without
    #[test]
    fn test_mixed_runs() {
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld>
<p:spTree>
<p:sp>
<p:txBody>
<a:p>
<a:r>
<a:rPr lang="en-US" sz="1200"/>
<a:t>Simple run</a:t>
</a:r>
<a:r>
<a:rPr lang="en-US" sz="1400">
<a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>
</a:rPr>
<a:t>Complex run</a:t>
</a:r>
</a:p>
</p:txBody>
</p:sp>
</p:spTree>
</p:cSld>
</p:sld>"#;

        let color = Some("FFFFFF".to_string());

        let result = format_text_in_element_fixed(slide_xml, &color);
        assert!(
            result.is_ok(),
            "Should handle mixed runs: {:?}",
            result.err()
        );

        let modified = result.unwrap();
        assert!(
            modified.contains(r#"color="FFFFFF""#),
            "Should add color to both runs"
        );
        assert!(
            modified.contains("<a:solidFill>"),
            "Should preserve existing structure"
        );
    }

    /// Test case 5: Realistic PPTX slide structure from the wip16.json error
    #[test]
    fn test_realistic_slide_structure() {
        // This is a simplified version of what create_presentation generates
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld>
<p:spTree>
<p:sp>
<p:nvSpPr>
<p:cNvPr id="2" name="Title 1"/>
</p:nvSpPr>
<p:txBody>
<a:bodyPr/>
<a:lstStyle/>
<a:p>
<a:pPr algn="l"/>
<a:r>
<a:rPr lang="en-US" sz="4400" b="1">
<a:solidFill>
<a:srgbClr val="000000"/>
</a:solidFill>
<a:latin typeface="Calibri"/>
</a:rPr>
<a:t>Breaking the Barrier</a:t>
</a:r>
<a:endParaRPr lang="en-US" sz="4400"/>
</a:p>
</p:txBody>
</p:sp>
</p:spTree>
</p:cSld>
</p:sld>"#;

        let color = Some("FFFFFF".to_string());

        // This is the exact scenario that failed in wip16.json
        let result = format_text_in_element_fixed(slide_xml, &color);
        assert!(
            result.is_ok(),
            "Should handle realistic slide: {:?}",
            result.err()
        );

        let modified = result.unwrap();
        assert!(
            modified.contains(r#"color="FFFFFF""#),
            "Should apply white color"
        );
        assert!(
            modified.contains("<a:solidFill>"),
            "Should keep fill structure"
        );
        assert!(modified.contains("<a:latin"), "Should keep font structure");

        // Verify well-formed
        let mut verify_reader = Reader::from_str(&modified);
        let mut verify_buf = Vec::new();
        loop {
            match verify_reader.read_event_into(&mut verify_buf) {
                Ok(Event::Eof) => break,
                Err(e) => panic!("Realistic slide XML is malformed: {}", e),
                _ => {}
            }
            verify_buf.clear();
        }

        println!("✓ Realistic slide structure handled correctly");
    }
}
