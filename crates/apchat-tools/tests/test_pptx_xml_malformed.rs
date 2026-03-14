//! Unit tests that reproduce the PPTX XML corruption bug
//!
//! These tests demonstrate the root cause of the issue seen in wip16.json:
//! "XML parsing error: Expecting </a:r> found </a:rPr>"
//!
//! ROOT CAUSE: When format_text_in_element() encounters <a:rPr> tags,
//! it writes them as Empty events, but then the original </a:rPr> end tag
//! is still written later, creating malformed XML like:
//!
//!   <a:r><a:rPr attrs.../><a:t>text</a:t></a:r></a:rPr>
//!                                                        ^ orphaned end tag
//!
//! This causes Keynote and other PPTX readers to fail.

#[cfg(test)]
mod pptx_xml_bug_tests {
    use quick_xml::events::{BytesStart, Event};
    use quick_xml::Reader;
    use quick_xml::Writer;
    use std::collections::HashMap;
    use std::io::Cursor;

    /// This is the EXACT buggy code from pptx_element_edit.rs:1119-1128
    fn format_text_buggy_version(
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

                        // BUG: Writing Start tag as Empty!
                        let modified = modify_rpr(e, color);
                        writer.write_event(Event::Empty(modified))?;
                        // This writes: <a:rPr attrs.../>
                        // But the original </a:rPr> will still be written later!
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
                    // BUG: This </a:rPr> is still written even though we wrote <a:rPr/> above
                    writer.write_event(Event::End(e.to_owned()))?;
                }
                Ok(Event::Empty(ref e)) => {
                    if in_target_element && e.name().as_ref() == b"a:rPr" {
                        let modified = modify_rpr(e, color);
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

    /// This is the FIXED code
    fn format_text_fixed_version(
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

                        // FIX: Write as Start, not Empty - preserve children
                        let modified = modify_rpr(e, color);
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

                    writer.write_event(Event::End(e.to_owned()))?;
                }
                Ok(Event::Empty(ref e)) => {
                    if in_target_element && e.name().as_ref() == b"a:rPr" {
                        let modified = modify_rpr(e, color);
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

    fn modify_rpr(event: &BytesStart, color: &Option<String>) -> BytesStart<'static> {
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

    /// Test that reproduces the EXACT error from wip16.json line 909
    #[test]
    fn test_reproduces_wip16_error() {
        // Minimal XML structure that triggers the bug
        let xml = r#"<a:r><a:rPr lang="en-US"><a:solidFill/></a:rPr><a:t>Hi</a:t></a:r>"#;
        let color = Some("FFFFFF".to_string());

        let buggy_result = format_text_buggy_version(xml, &color);
        assert!(
            buggy_result.is_ok(),
            "Buggy version should not error during processing"
        );

        let buggy_xml = buggy_result.unwrap();
        println!("Buggy output: {}", buggy_xml);

        // The bug: orphaned </a:rPr> tag appears AFTER the parent </a:r>
        assert!(
            buggy_xml.contains("</a:rPr>"),
            "Should have orphaned end tag"
        );

        // Try to parse the buggy XML - it should fail!
        let mut verify_reader = Reader::from_str(&buggy_xml);
        let mut verify_buf = Vec::new();
        let mut found_error = false;
        loop {
            match verify_reader.read_event_into(&mut verify_buf) {
                Ok(Event::Eof) => break,
                Err(_) => {
                    found_error = true;
                    break;
                }
                _ => {}
            }
            verify_buf.clear();
        }

        assert!(
            found_error,
            "Buggy XML should be malformed and fail to parse"
        );
        println!("✓ Buggy XML is malformed as expected");
    }

    /// Test that the fixed version produces valid XML
    #[test]
    fn test_fixed_version_produces_valid_xml() {
        let xml = r#"<a:r><a:rPr lang="en-US"><a:solidFill/></a:rPr><a:t>Hi</a:t></a:r>"#;
        let color = Some("FFFFFF".to_string());

        let fixed_result = format_text_fixed_version(xml, &color);
        assert!(fixed_result.is_ok(), "Fixed version should succeed");

        let fixed_xml = fixed_result.unwrap();
        println!("Fixed output: {}", fixed_xml);

        // Verify balanced tags
        let open_count = fixed_xml.matches("<a:rPr").count();
        let close_count = fixed_xml.matches("</a:rPr>").count();
        assert_eq!(open_count, close_count, "Tags should be balanced");

        // Try to parse - should succeed
        let mut verify_reader = Reader::from_str(&fixed_xml);
        let mut verify_buf = Vec::new();
        loop {
            match verify_reader.read_event_into(&mut verify_buf) {
                Ok(Event::Eof) => break,
                Err(e) => panic!("Fixed XML should be valid: {}", e),
                _ => {}
            }
            verify_buf.clear();
        }

        println!("✓ Fixed XML is well-formed");
    }

    /// Test with the realistic structure from wip16.json
    #[test]
    fn test_realistic_slide_xml() {
        // This is what create_presentation generates (simplified)
        let xml = r#"<p:txBody>
<a:p>
<a:r>
<a:rPr lang="en-US" sz="4400" b="1">
<a:solidFill><a:srgbClr val="000000"/></a:solidFill>
<a:latin typeface="Calibri"/>
</a:rPr>
<a:t>Breaking the Barrier</a:t>
</a:r>
<a:endParaRPr lang="en-US" sz="4400"/>
</a:p>
</p:txBody>"#;

        let color = Some("FFFFFF".to_string());

        // Buggy version
        let buggy_result = format_text_buggy_version(xml, &color);
        assert!(buggy_result.is_ok());
        let buggy_xml = buggy_result.unwrap();

        // Count tags
        let open_rpr = buggy_xml.matches("<a:rPr").count();
        let close_rpr = buggy_xml.matches("</a:rPr>").count();

        println!("Buggy: {} open, {} close", open_rpr, close_rpr);
        println!("Buggy XML: {}", buggy_xml);

        // Fixed version
        let fixed_result = format_text_fixed_version(xml, &color);
        assert!(fixed_result.is_ok());
        let fixed_xml = fixed_result.unwrap();

        let open_rpr_fixed = fixed_xml.matches("<a:rPr").count();
        let close_rpr_fixed = fixed_xml.matches("</a:rPr>").count();

        println!("Fixed: {} open, {} close", open_rpr_fixed, close_rpr_fixed);

        assert_eq!(
            open_rpr_fixed, close_rpr_fixed,
            "Fixed version should have balanced tags"
        );
    }
}
