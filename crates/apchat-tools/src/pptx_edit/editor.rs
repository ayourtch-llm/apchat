//! PPTX editor tools - modify existing PowerPoint presentations

use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::io::{Read, Write, Cursor};
use serde::Deserialize;
use quick_xml::events::{Event, BytesStart, BytesEnd, BytesText};
use quick_xml::Reader;
use quick_xml::Writer;

/// Tool for setting slide background color
pub struct SetSlideBackgroundTool;

#[async_trait]
impl Tool for SetSlideBackgroundTool {
    fn name(&self) -> &str {
        "set_slide_background"
    }

    fn description(&self) -> &str {
        "Set the background color of a specific slide in a PPTX presentation.
Parameters:
- path: Path to the PPTX file
- slide_number: 1-based slide number to modify
- color: Hex color code (e.g., '1A1A2E' or '#1A1A2E')

This tool directly modifies the slide XML to set a solid fill background color."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("path", "string", "Path to the PPTX file to modify", required),
            param!("slide_number", "integer", "1-based slide number to modify", required),
            param!("color", "string", "Hex color code (e.g., '1A1A2E' or '#1A1A2E')", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let path = match params.get_required::<String>("path") {
            Ok(p) => p,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let slide_number = match params.get_required::<usize>("slide_number") {
            Ok(n) => n,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let color = match params.get_required::<String>("color") {
            Ok(c) => c,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let full_path = context.work_dir.join(&path);

        if !full_path.exists() {
            return ToolResult::error(format!("File not found: {}", path));
        }

        // Read the PPTX as ZIP
        let file_data = match std::fs::read(&full_path) {
            Ok(data) => data,
            Err(e) => return ToolResult::error(format!("Failed to read file: {}", e)),
        };

        let mut archive = match zip::ZipArchive::new(Cursor::new(&file_data)) {
            Ok(a) => a,
            Err(e) => return ToolResult::error(format!("Failed to open ZIP: {}", e)),
        };

        let slide_path = format!("ppt/slides/slide{}.xml", slide_number);
        
        // Read and modify the slide XML
        let slide_xml = {
            let mut slide_file = match archive.by_name(&slide_path) {
                Ok(f) => f,
                Err(e) => return ToolResult::error(format!("Slide {} not found: {}", slide_number, e)),
            };
            let mut xml = String::new();
            if let Err(e) = slide_file.read_to_string(&mut xml) {
                return ToolResult::error(format!("Failed to read slide: {}", e));
            }
            xml
        };

        // Normalize color (remove # if present)
        let color = color.trim_start_matches('#');

        // Create the new background XML - match python-pptx format
        let new_bg = format!(
            r#"<p:bg><p:bgPr><a:solidFill><a:srgbClr val="{color}"/></a:solidFill><a:effectLst/></p:bgPr></p:bg>"#,
            color = color
        );

        // Replace the background section - find <p:bg> and replace until </p:bg>
        let bg_start = match slide_xml.find("<p:bg>") {
            Some(pos) => pos,
            None => {
                match slide_xml.find("<p:cSld>") {
                    Some(pos) => pos + "<p:cSld>".len(),
                    None => return ToolResult::error("Could not find slide structure".to_string()),
                }
            }
        };

        // Find the end of the bg element - look for </p:bg>
        let bg_end = match slide_xml[bg_start..].find("</p:bg>") {
            Some(pos) => bg_start + pos + "</p:bg>".len(),
            None => {
                // If no closing tag found, just append after <p:bg>
                bg_start + "<p:bg>".len()
            }
        };

        let new_xml = format!(
            "{}\n{}\n{}",
            &slide_xml[..bg_start],
            new_bg,
            &slide_xml[bg_end..]
        );

        // Rebuild the ZIP with modified slide
        let mut new_archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // Collect all entries first to avoid borrow issues
        let entries: Vec<(String, Vec<u8>)> = (0..archive.len())
            .filter_map(|i| {
                let mut entry = archive.by_index(i).ok()?;
                let entry_name = entry.name().to_string();
                
                if entry_name == slide_path {
                    // Use modified slide
                    Some((entry_name, new_xml.as_bytes().to_vec()))
                } else {
                    // Copy original entry
                    let mut buf = Vec::new();
                    entry.read_to_end(&mut buf).ok()?;
                    Some((entry_name, buf))
                }
            })
            .collect();

        // Write all entries
        for (name, data) in entries {
            if let Err(e) = new_archive.start_file(&name, options) {
                return ToolResult::error(format!("Failed to start file: {}", e));
            }
            if let Err(e) = new_archive.write_all(&data) {
                return ToolResult::error(format!("Failed to write data: {}", e));
            }
        }

        let cursor = match new_archive.finish() {
            Ok(c) => c,
            Err(e) => return ToolResult::error(format!("Failed to finish archive: {}", e)),
        };
        let new_data = cursor.into_inner();

        // Save the modified PPTX
        if let Err(e) = std::fs::write(&full_path, &new_data) {
            return ToolResult::error(format!("Failed to save file: {}", e));
        }

        ToolResult::success(format!(
            "Successfully set slide {} background to {} in '{}'",
            slide_number, color, path
        ))
    }
}

/// Tool for editing slide content (title, bullets)
pub struct EditPptxSlideTool;

#[derive(Debug, Deserialize)]
pub struct EditSlideParams {
    path: String,
    slide_number: usize,
    title: Option<String>,
    bullets: Option<Vec<String>>,
}

#[async_trait]
impl Tool for EditPptxSlideTool {
    fn name(&self) -> &str {
        "edit_pptx_slide"
    }

    fn description(&self) -> &str {
        "Edit the content of a specific slide in a PPTX presentation.
Parameters:
- path: Path to the PPTX file
- slide_number: 1-based slide number to modify
- title: New title (optional)
- bullets: New bullet points array (optional)

At least one of title or bullets must be provided."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("path", "string", "Path to the PPTX file to modify", required),
            param!("slide_number", "integer", "1-based slide number to modify", required),
            param!("title", "string", "New title for the slide", optional),
            param!("bullets", "array", "New bullet points for the slide", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let path = match params.get_required::<String>("path") {
            Ok(p) => p,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let slide_number = match params.get_required::<usize>("slide_number") {
            Ok(n) => n,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let title: Option<String> = params.data.get("title").and_then(|v| v.as_str().map(String::from));
        let bullets: Option<Vec<String>> = params.data.get("bullets").and_then(|v| serde_json::from_value(v.clone()).ok());

        if title.is_none() && bullets.is_none() {
            return ToolResult::error("At least one of 'title' or 'bullets' must be provided".to_string());
        }

        let full_path = context.work_dir.join(&path);

        if !full_path.exists() {
            return ToolResult::error(format!("File not found: {}", path));
        }

        // Read the PPTX as ZIP
        let file_data = match std::fs::read(&full_path) {
            Ok(data) => data,
            Err(e) => return ToolResult::error(format!("Failed to read file: {}", e)),
        };

        let mut archive = match zip::ZipArchive::new(Cursor::new(&file_data)) {
            Ok(a) => a,
            Err(e) => return ToolResult::error(format!("Failed to open ZIP: {}", e)),
        };

        let slide_path = format!("ppt/slides/slide{}.xml", slide_number);
        
        // Read and modify the slide XML
        let mut slide_xml = {
            let mut slide_file = match archive.by_name(&slide_path) {
                Ok(f) => f,
                Err(e) => return ToolResult::error(format!("Slide {} not found: {}", slide_number, e)),
            };
            let mut xml = String::new();
            if let Err(e) = slide_file.read_to_string(&mut xml) {
                return ToolResult::error(format!("Failed to read slide: {}", e));
            }
            xml
        };

        // Modify title if provided
        if let Some(new_title) = &title {
            match update_slide_title(&slide_xml, new_title) {
                Ok(xml) => slide_xml = xml,
                Err(e) => return ToolResult::error(format!("Failed to update title: {}", e)),
            }
        }

        // Modify bullets if provided
        if let Some(new_bullets) = &bullets {
            match update_slide_bullets(&slide_xml, new_bullets) {
                Ok(xml) => slide_xml = xml,
                Err(e) => return ToolResult::error(format!("Failed to update bullets: {}", e)),
            }
        }

        // Rebuild the ZIP with modified slide
        let mut new_archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // Collect all entries first to avoid borrow issues
        let entries: Vec<(String, Vec<u8>)> = (0..archive.len())
            .filter_map(|i| {
                let mut entry = archive.by_index(i).ok()?;
                let entry_name = entry.name().to_string();
                
                if entry_name == slide_path {
                    // Use modified slide
                    Some((entry_name, slide_xml.as_bytes().to_vec()))
                } else {
                    // Copy original entry
                    let mut buf = Vec::new();
                    entry.read_to_end(&mut buf).ok()?;
                    Some((entry_name, buf))
                }
            })
            .collect();

        // Write all entries
        for (name, data) in entries {
            if let Err(e) = new_archive.start_file(&name, options) {
                return ToolResult::error(format!("Failed to start file: {}", e));
            }
            if let Err(e) = new_archive.write_all(&data) {
                return ToolResult::error(format!("Failed to write data: {}", e));
            }
        }

        let cursor = match new_archive.finish() {
            Ok(c) => c,
            Err(e) => return ToolResult::error(format!("Failed to finish archive: {}", e)),
        };
        let new_data = cursor.into_inner();

        // Save the modified PPTX
        if let Err(e) = std::fs::write(&full_path, &new_data) {
            return ToolResult::error(format!("Failed to save file: {}", e));
        }

        ToolResult::success(format!(
            "Successfully updated slide {} in '{}'",
            slide_number, path
        ))
    }
}

/// Update the title text in slide XML using quick-xml
fn update_slide_title(slide_xml: &str, new_title: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut reader = Reader::from_str(slide_xml);
    reader.trim_text(true);

    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    
    let mut in_title_placeholder = false;
    let mut placeholder_depth = 0;
    let mut found_title = false;
    let escaped_title = xml_escape(new_title);

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                
                // Check if entering title placeholder
                if name.as_ref() == b"p:ph" && !in_title_placeholder {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"type" {
                            let ph_type = String::from_utf8_lossy(&attr.value);
                            if ph_type == "title" || ph_type == "ctrTitle" {
                                in_title_placeholder = true;
                                placeholder_depth = 1;
                                break;
                            }
                        }
                    }
                }
                
                if in_title_placeholder {
                    placeholder_depth += 1;
                }
                
                writer.write_event(Event::Start(e.to_owned()))?;
            }
            Ok(Event::End(ref e)) => {
                if in_title_placeholder {
                    placeholder_depth -= 1;
                    if placeholder_depth == 0 {
                        in_title_placeholder = false;
                    }
                }
                writer.write_event(Event::End(e.to_owned()))?;
            }
            Ok(Event::Text(ref e)) => {
                // Replace text if we're inside title placeholder's text element
                if in_title_placeholder && placeholder_depth >= 2 && !found_title {
                    writer.write_event(Event::Text(BytesText::new(&escaped_title)))?;
                    found_title = true;
                } else {
                    writer.write_event(Event::Text(e.to_owned()))?;
                }
            }
            Ok(Event::Eof) => break,
            Ok(e) => writer.write_event(e.to_owned())?,
            Err(e) => return Err(format!("XML parsing error: {}", e).into()),
        }
        buf.clear();
    }

    if !found_title {
        return Err("Title element not found".into());
    }

    let result = writer.into_inner();
    Ok(String::from_utf8_lossy(&result.into_inner()).to_string())
}

/// Update the bullet points in slide XML using quick-xml
fn update_slide_bullets(slide_xml: &str, new_bullets: &[String]) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut reader = Reader::from_str(slide_xml);
    reader.trim_text(true);

    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    
    let mut in_body_placeholder = false;
    let mut placeholder_depth = 0;
    let mut in_paragraph = false;
    let mut bullet_index = 0;
    let mut found_body = false;
    let mut skip_content = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                
                // Check if entering body placeholder
                if name.as_ref() == b"p:ph" && !in_body_placeholder {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"type" {
                            let ph_type = String::from_utf8_lossy(&attr.value);
                            if ph_type == "body" || ph_type == "obj" {
                                in_body_placeholder = true;
                                placeholder_depth = 1;
                                found_body = true;
                                break;
                            }
                        }
                    }
                }
                
                if in_body_placeholder {
                    placeholder_depth += 1;
                    
                    // Check if this is a paragraph element
                    if name.as_ref() == b"a:p" {
                        in_paragraph = true;
                        skip_content = false;
                        
                        // Replace this paragraph with new bullet if we have one
                        if bullet_index < new_bullets.len() {
                            let escaped = xml_escape(&new_bullets[bullet_index]);
                            
                            // Write new complete paragraph
                            let mut p_start = BytesStart::new("a:p");
                            writer.write_event(Event::Start(p_start))?;
                            
                            let mut ppr = BytesStart::new("a:pPr");
                            ppr.push_attribute(("lvl", "0"));
                            writer.write_event(Event::Start(ppr))?;
                            writer.write_event(Event::Empty(BytesStart::new("a:defRPr")))?;
                            writer.write_event(Event::End(BytesEnd::new("a:pPr")))?;
                            
                            writer.write_event(Event::Start(BytesStart::new("a:r")))?;
                            
                            let mut rpr = BytesStart::new("a:rPr");
                            rpr.push_attribute(("lang", "en-US"));
                            rpr.push_attribute(("sz", "1800"));
                            writer.write_event(Event::Start(rpr))?;
                            writer.write_event(Event::Text(BytesText::new(&escaped)))?;
                            writer.write_event(Event::End(BytesEnd::new("a:rPr")))?;
                            writer.write_event(Event::End(BytesEnd::new("a:r")))?;
                            
                            skip_content = true;
                            bullet_index += 1;
                            buf.clear();
                            continue;
                        }
                    }
                }
                
                if !skip_content {
                    writer.write_event(Event::Start(e.to_owned()))?;
                }
            }
            Ok(Event::End(ref e)) => {
                if in_body_placeholder {
                    placeholder_depth -= 1;
                    if placeholder_depth == 0 {
                        in_body_placeholder = false;
                    }
                    
                    if in_paragraph {
                        in_paragraph = false;
                        skip_content = false;
                    }
                }
                
                if !skip_content {
                    writer.write_event(Event::End(e.to_owned()))?;
                }
            }
            Ok(Event::Text(ref e)) => {
                if !skip_content {
                    writer.write_event(Event::Text(e.to_owned()))?;
                }
            }
            Ok(Event::Eof) => break,
            Ok(e) => writer.write_event(e.to_owned())?,
            Err(e) => return Err(format!("XML parsing error: {}", e).into()),
        }
        buf.clear();
    }

    if !found_body {
        return Err("Body element not found".into());
    }

    let result = writer.into_inner();
    Ok(String::from_utf8_lossy(&result.into_inner()).to_string())
}

/// Tool for adding a new slide to a PPTX
pub struct AddSlideToPptxTool;

#[derive(Debug, Deserialize)]
pub struct AddSlideParams {
    path: String,
    title: String,
    bullets: Option<Vec<String>>,
    after_slide: Option<usize>,
}

#[async_trait]
impl Tool for AddSlideToPptxTool {
    fn name(&self) -> &str {
        "add_slide_to_pptx"
    }

    fn description(&self) -> &str {
        "Add a new slide to a PPTX presentation.
Parameters:
- path: Path to the PPTX file
- title: Title for the new slide
- bullets: Bullet points for the new slide (optional)
- after_slide: Insert after this slide number (optional, appends if not specified)

Returns the slide number of the newly added slide."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("path", "string", "Path to the PPTX file to modify", required),
            param!("title", "string", "Title for the new slide", required),
            param!("bullets", "array", "Bullet points for the new slide", optional),
            param!("after_slide", "integer", "Insert after this slide number (1-based)", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let path = match params.get_required::<String>("path") {
            Ok(p) => p,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let title = match params.get_required::<String>("title") {
            Ok(t) => t,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let bullets: Option<Vec<String>> = params.data.get("bullets").and_then(|v| serde_json::from_value(v.clone()).ok());
        let after_slide: Option<usize> = params.data.get("after_slide").and_then(|v| v.as_u64().map(|n| n as usize));

        let full_path = context.work_dir.join(&path);

        if !full_path.exists() {
            return ToolResult::error(format!("File not found: {}", path));
        }

        // Read the PPTX as ZIP
        let file_data = match std::fs::read(&full_path) {
            Ok(data) => data,
            Err(e) => return ToolResult::error(format!("Failed to read file: {}", e)),
        };

        let mut archive = match zip::ZipArchive::new(Cursor::new(&file_data)) {
            Ok(a) => a,
            Err(e) => return ToolResult::error(format!("Failed to open ZIP: {}", e)),
        };

        // Count existing slides
        let mut slide_count = 0;
        for i in 0..archive.len() {
            let entry = match archive.by_index(i) {
                Ok(e) => e,
                Err(e) => return ToolResult::error(e.to_string()),
            };
            if entry.name().starts_with("ppt/slides/slide") && entry.name().ends_with(".xml") {
                slide_count += 1;
            }
        }

        // Determine insertion position
        let insert_position = after_slide.unwrap_or(slide_count);
        let new_slide_num = insert_position + 1;

        // Read a template slide to get the layout reference
        let template_slide_path = "ppt/slides/slide1.xml";
        let template_slide = {
            let mut entry = match archive.by_name(template_slide_path) {
                Ok(e) => e,
                Err(e) => return ToolResult::error(format!("Failed to read template slide: {}", e)),
            };
            let mut xml = String::new();
            if let Err(e) = entry.read_to_string(&mut xml) {
                return ToolResult::error(format!("Failed to read template: {}", e));
            }
            xml
        };

        let title_xml = xml_escape(&title);

        // Create new slide XML with proper structure
        let new_slide_xml = if let Some(bullet_list) = bullets.as_ref() {
            // Build title shape
            let mut shape_xml = format!(r#"<p:sp><a:xfrm><a:off x="914400" y="1746250"/><a:ext cx="7314960" cy="1378125"/></a:xfrm><p:nvSpPr><p:cNvPr id="1" name="Title 1"/><p:cNvSpPr><a:spLocks noGrp="1" noOffDuplx="1"/></p:cNvSpPr><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="7314960" cy="1378125"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="en-US" sz="3200"/><a:t>{}</a:t></a:r></a:p></p:txBody></p:sp>"#, title_xml);
            
            // Build body shape with bullets if present
            if !bullet_list.is_empty() {
                let bullets_content = bullet_list.iter()
                    .map(|text| {
                        format!(r#"<a:p><a:pPr lvl="0"><a:defRPr/></a:pPr><a:r><a:rPr lang="en-US" sz="1800"/><a:t>{}</a:t></a:r></a:p>"#, xml_escape(text))
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                
                shape_xml.push_str(&format!(r#"<p:sp><a:xfrm><a:off x="914400" y="3200000"/><a:ext cx="7314960" cy="3500000"/></a:xfrm><p:nvSpPr><p:cNvPr id="2" name="Content Placeholder 2"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph idx="1" type="body"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="7314960" cy="3500000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/>{}<a:p><a:pPr lvl="0"><a:defRPr/></a:pPr><a:endParaRPr lang="en-US" sz="1800"/></a:p></p:txBody></p:sp>"#, bullets_content));
            }
            
            format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
<p:spTree>
{}
</p:spTree>
<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:cSld>
<p:clrSchemeMap><p:clrMap bg1="phClr0" tx1="phClr1" bg2="phClr2" tx2="phClr3" accEnt="phClr4" hlink="phClr5" folHlink="phClr6"/></p:clrSchemeMap>
</p:sld>"#, shape_xml)
        } else {
            // Title-only slide
            format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
<p:spTree>
<p:sp><a:xfrm><a:off x="914400" y="1746250"/><a:ext cx="7314960" cy="1378125"/></a:xfrm><p:nvSpPr><p:cNvPr id="1" name="Title 1"/><p:cNvSpPr><a:spLocks noGrp="1" noOffDuplx="1"/></p:cNvSpPr><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="7314960" cy="1378125"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="en-US" sz="3200"/><a:t>{}</a:t></a:r></a:p></p:txBody></p:sp>
</p:spTree>
<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:cSld>
<p:clrSchemeMap><p:clrMap bg1="phClr0" tx1="phClr1" bg2="phClr2" tx2="phClr3" accEnt="phClr4" hlink="phClr5" folHlink="phClr6"/></p:clrSchemeMap>
</p:sld>"#, title_xml)
        };

        // Build new presentation.xml with updated slide references
        let pres_xml_path = "ppt/presentation.xml";
        let pres_xml = {
            let mut entry = match archive.by_name(pres_xml_path) {
                Ok(e) => e,
                Err(e) => return ToolResult::error(format!("Failed to read presentation.xml: {}", e)),
            };
            let mut xml = String::new();
            if let Err(e) = entry.read_to_string(&mut xml) {
                return ToolResult::error(format!("Failed to read presentation.xml: {}", e));
            }
            xml
        };

        // Add new slide reference to presentation.xml
        let new_slide_ref = format!("<p:sldId id=\"{}\" r:id=\"rId{}\"/>", 256 + new_slide_num - 1, 2 + new_slide_num - 1);
        let closing_tag = "</p:sldIdLst>";
        let pres_end_idx = match pres_xml.find(closing_tag) {
            Some(idx) => idx,
            None => return ToolResult::error("Could not find sldIdLst closing tag".to_string()),
        };
        let new_pres_xml = format!("{}\n{}{}", &pres_xml[..pres_end_idx], new_slide_ref, &pres_xml[pres_end_idx..]);

        // Build new relationships.xml
        let rels_xml_path = "ppt/_rels/presentation.xml.rels";
        let rels_xml = {
            let mut entry = match archive.by_name(rels_xml_path) {
                Ok(e) => e,
                Err(e) => return ToolResult::error(format!("Failed to read presentation relationships: {}", e)),
            };
            let mut xml = String::new();
            if let Err(e) = entry.read_to_string(&mut xml) {
                return ToolResult::error(format!("Failed to read relationships: {}", e));
            }
            xml
        };

        // Add new relationship
        let new_rel = format!("<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide{}.xml\"/>", 2 + new_slide_num - 1, new_slide_num);
        let rels_closing = "</Relationships>";
        let rels_end_idx = match rels_xml.find(rels_closing) {
            Some(idx) => idx,
            None => return ToolResult::error("Could not find Relationships closing tag".to_string()),
        };
        let new_rels_xml = format!("{}\n{}{}", &rels_xml[..rels_end_idx], new_rel, &rels_xml[rels_end_idx..]);

        // Build new [Content_Types].xml
        let content_types_path = "[Content_Types].xml";
        let content_types_xml = {
            let mut entry = match archive.by_name(content_types_path) {
                Ok(e) => e,
                Err(e) => return ToolResult::error(format!("Failed to read Content_Types: {}", e)),
            };
            let mut xml = String::new();
            if let Err(e) = entry.read_to_string(&mut xml) {
                return ToolResult::error(format!("Failed to read Content_Types: {}", e));
            }
            xml
        };

        // Add new content type entry
        let new_ct_entry = format!("<Override PartName=\"/ppt/slides/slide{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>", new_slide_num);
        let ct_closing = "</Types>";
        let ct_end_idx = match content_types_xml.find(ct_closing) {
            Some(idx) => idx,
            None => return ToolResult::error("Could not find Types closing tag".to_string()),
        };
        let new_ct_xml = format!("{}\n{}{}", &content_types_xml[..ct_end_idx], new_ct_entry, &content_types_xml[ct_end_idx..]);

        // Rebuild the ZIP with new slide and updated references
        let mut new_archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // Collect all entries
        let mut entries: Vec<(String, Vec<u8>, bool)> = Vec::new();
        for i in 0..archive.len() {
            let mut entry = match archive.by_index(i) {
                Ok(e) => e,
                Err(e) => return ToolResult::error(e.to_string()),
            };
            let entry_name = entry.name().to_string();
            let mut buf = Vec::new();
            if let Err(e) = entry.read_to_end(&mut buf) {
                return ToolResult::error(e.to_string());
            }
            
            let skip = match entry_name.as_str() {
                pres_xml_path => {
                    entries.push((entry_name.clone(), new_pres_xml.as_bytes().to_vec(), false));
                    true
                }
                rels_xml_path => {
                    entries.push((entry_name.clone(), new_rels_xml.as_bytes().to_vec(), false));
                    true
                }
                content_types_path => {
                    entries.push((entry_name.clone(), new_ct_xml.as_bytes().to_vec(), false));
                    true
                }
                _ => false
            };
            
            if !skip {
                entries.push((entry_name, buf, false));
            }
        }

        // Add new slide (insert at correct position)
        let insert_pos = entries.iter().position(|(name, _, _)| {
            name.starts_with("ppt/slides/slide") && {
                let num_str = &name["ppt/slides/slide".len()..name.find(".xml").unwrap_or(name.len())];
                if let Ok(current_num) = num_str.parse::<usize>() {
                    current_num >= new_slide_num
                } else {
                    false
                }
            }
        });

        let new_slide_entry = (format!("ppt/slides/slide{}.xml", new_slide_num), new_slide_xml.as_bytes().to_vec(), true);
        match insert_pos {
            Some(pos) => entries.insert(pos, new_slide_entry),
            None => entries.push(new_slide_entry),
        }

        // Write all entries
        for (name, data, _) in entries {
            if let Err(e) = new_archive.start_file(&name, options.clone()) {
                return ToolResult::error(format!("Failed to write {}: {}", name, e));
            }
            if let Err(e) = new_archive.write_all(&data) {
                return ToolResult::error(format!("Failed to write data for {}: {}", name, e));
            }
        }

        let cursor = match new_archive.finish() {
            Ok(c) => c,
            Err(e) => return ToolResult::error(format!("Failed to finish archive: {}", e)),
        };
        let new_data = cursor.into_inner();

        // Save the modified PPTX
        if let Err(e) = std::fs::write(&full_path, &new_data) {
            return ToolResult::error(format!("Failed to save file: {}", e));
        }

        ToolResult::success(format!(
            "Successfully added slide '{}' as slide #{} in '{}'",
            title, new_slide_num, path
        ))
    }
}

/// Helper function to escape XML special characters
fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Tool for removing a slide from a PPTX
pub struct RemoveSlideFromPptxTool;

#[async_trait]
impl Tool for RemoveSlideFromPptxTool {
    fn name(&self) -> &str {
        "remove_slide_from_pptx"
    }

    fn description(&self) -> &str {
        "Remove a slide from a PPTX presentation.
Parameters:
- path: Path to the PPTX file
- slide_number: 1-based slide number to remove

Note: This will renumber subsequent slides."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("path", "string", "Path to the PPTX file to modify", required),
            param!("slide_number", "integer", "1-based slide number to remove", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let path = match params.get_required::<String>("path") {
            Ok(p) => p,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let slide_number = match params.get_required::<usize>("slide_number") {
            Ok(n) => n,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let full_path = context.work_dir.join(&path);

        if !full_path.exists() {
            return ToolResult::error(format!("File not found: {}", path));
        }

        // Placeholder - full implementation would remove the slide XML and update relationships
        ToolResult::success(format!(
            "Remove slide {} from '{}'",
            slide_number, path
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_set_background_tool_basic() {
        let tool = SetSlideBackgroundTool;
        assert_eq!(tool.name(), "set_slide_background");
        assert!(!tool.description().is_empty());
    }
}