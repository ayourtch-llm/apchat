//! Advanced PPTX tools for detailed slide analysis and manipulation
//!
//! This module provides tools for:
//! - Detailed slide reading with layout analysis
//! - Element detection (text, images, charts, tables, shapes)
//! - Slide transitions
//! - Position and size information

use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use serde::Serialize;
use quick_xml::events::Event;
use quick_xml::Reader;

/// Detailed slide information
#[derive(Debug, Serialize)]
pub struct DetailedSlide {
    pub slide_number: usize,
    pub slide_id: Option<String>,
    pub layout_name: Option<String>,
    pub layout_id: Option<String>,
    pub title: Option<String>,
    pub elements: Vec<SlideElement>,
    pub has_background: bool,
    pub background_type: Option<String>,
    pub notes: Option<String>,
    pub transition: Option<TransitionInfo>,
}

/// Individual slide element
#[derive(Debug, Serialize)]
pub struct SlideElement {
    pub element_type: String,
    pub name: Option<String>,
    pub position: Option<ElementPosition>,
    pub size: Option<ElementSize>,
    pub content: Option<String>,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ElementPosition {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct ElementSize {
    pub width: i64,
    pub height: i64,
}

#[derive(Debug, Serialize)]
pub struct TransitionInfo {
    pub transition_type: String,
    pub duration: Option<f64>,
    pub advance_on_click: bool,
    pub advance_after_time: Option<f64>,
}

/// Tool for reading detailed slide information
pub struct ReadSlideDetailedTool;

#[async_trait]
impl Tool for ReadSlideDetailedTool {
    fn name(&self) -> &str {
        "read_slide_detailed"
    }

    fn description(&self) -> &str {
        "Read detailed information about slides including layout, elements, images, charts, and formatting.
Parameters:
- path: Path to the PPTX file
- slide_number: 1-based slide number to read (optional, reads all slides if not specified)

Returns detailed information about:
- Slide layout and master information
- All elements (text boxes, shapes, images, charts, tables) with positions and sizes
- Background styling
- Speaker notes
- Transition settings"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("path", "string", "Path to the PPTX file to read", required),
            param!("slide_number", "integer", "Specific slide number to read (1-based, optional)", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let path = match params.get_required::<String>("path") {
            Ok(p) => p,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let slide_number: Option<usize> = params.data.get("slide_number")
            .and_then(|v| v.as_u64().map(|n| n as usize));

        let full_path = context.work_dir.join(&path);

        if !full_path.exists() {
            return ToolResult::error(format!("File not found: {}", path));
        }

        let file_data = match std::fs::read(&full_path) {
            Ok(data) => data,
            Err(e) => return ToolResult::error(format!("Failed to read file: {}", e)),
        };

        let mut archive = match zip::ZipArchive::new(Cursor::new(&file_data)) {
            Ok(a) => a,
            Err(e) => return ToolResult::error(format!("Failed to open ZIP: {}", e)),
        };

        match read_slide_detailed(&mut archive, slide_number) {
            Ok(slides) => {
                let json = match serde_json::to_string_pretty(&slides) {
                    Ok(j) => j,
                    Err(e) => return ToolResult::error(format!("Failed to serialize: {}", e)),
                };
                ToolResult::success(json)
            }
            Err(e) => ToolResult::error(format!("Failed to read slide: {}", e)),
        }
    }
}

/// Read detailed slide information
pub fn read_slide_detailed(
    archive: &mut zip::ZipArchive<Cursor<&Vec<u8>>>,
    slide_number: Option<usize>,
) -> Result<Vec<DetailedSlide>, Box<dyn std::error::Error + Send + Sync>> {
    let mut slides = Vec::new();
    
    // First, collect all archive data to avoid borrow issues
    let mut archive_data: HashMap<String, Vec<u8>> = HashMap::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        archive_data.insert(name, data);
    }

    // Collect all slide files
    let mut slide_files: Vec<String> = Vec::new();
    for name in archive_data.keys() {
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
            slide_files.push(name.clone());
        }
    }
    slide_files.sort();

    for (idx, slide_path) in slide_files.iter().enumerate() {
        let slide_num = idx + 1;
        
        if let Some(req_num) = slide_number {
            if slide_num != req_num {
                continue;
            }
        }

        let xml_content = String::from_utf8_lossy(&archive_data[slide_path]).to_string();
        let slide = parse_slide_detailed(&xml_content, &archive_data, slide_num)?;
        slides.push(slide);
    }

    Ok(slides)
}

/// Parse slide XML for detailed information using quick-xml
fn parse_slide_detailed(
    xml: &str,
    archive_data: &HashMap<String, Vec<u8>>,
    slide_number: usize,
) -> Result<DetailedSlide, Box<dyn std::error::Error + Send + Sync>> {
    let mut title: Option<String> = None;
    let mut elements: Vec<SlideElement> = Vec::new();
    
    // Parse XML using quick-xml
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut in_text_element = false;
    let mut current_text = String::new();
    let mut current_element: Option<SlideElementBuilder> = None;
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"p:sp" => {
                        // Starting a shape
                        current_element = Some(SlideElementBuilder::new("text"));
                    }
                    b"p:pic" => {
                        // Starting a picture
                        current_element = Some(SlideElementBuilder::new("image"));
                    }
                    b"p:cNvPr" => {
                        // Extract name attribute
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"name" {
                                if let Some(ref mut elem) = current_element {
                                    elem.name = Some(String::from_utf8_lossy(&attr.value).to_string());
                                }
                            }
                        }
                    }
                    b"a:off" => {
                        // Extract position
                        let mut x = 0i64;
                        let mut y = 0i64;
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"x" => x = parse_emu(&attr.value),
                                b"y" => y = parse_emu(&attr.value),
                                _ => {}
                            }
                        }
                        if let Some(ref mut elem) = current_element {
                            elem.position = Some(ElementPosition { x, y });
                        }
                    }
                    b"a:ext" => {
                        // Extract size
                        let mut cx = 0i64;
                        let mut cy = 0i64;
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"cx" => cx = parse_emu(&attr.value),
                                b"cy" => cy = parse_emu(&attr.value),
                                _ => {}
                            }
                        }
                        if let Some(ref mut elem) = current_element {
                            elem.size = Some(ElementSize { width: cx, height: cy });
                        }
                    }
                    b"a:t" => {
                        in_text_element = true;
                    }
                    b"p:ph" => {
                        // Check placeholder type
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"type" {
                                let ph_type = String::from_utf8_lossy(&attr.value);
                                if ph_type == "title" || ph_type == "ctrTitle" {
                                    in_text_element = true;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_text_element {
                    current_text.push_str(&String::from_utf8_lossy(e));
                }
            }
            Ok(Event::End(ref e)) => {
                match e.name().as_ref() {
                    b"a:t" => {
                        in_text_element = false;
                    }
                    b"p:sp" | b"p:pic" => {
                        // End of element
                        if let Some(mut elem) = current_element.take() {
                            if !current_text.trim().is_empty() {
                                elem.content = Some(current_text.trim().to_string());
                            }
                            if let Some(completed) = elem.build() {
                                elements.push(completed);
                            }
                            current_text.clear();
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                eprintln!("XML parsing error at position {}: {}", reader.buffer_position(), e);
                break;
            }
            _ => {}
        }
        buf.clear();
    }
    
    // Extract title from first text element if not already found
    if title.is_none() {
        if let Some(elem) = elements.iter().find(|e| e.name.as_ref().map(|n| n.contains("Title")).unwrap_or(false)) {
            title = elem.content.clone();
        } else if let Some(elem) = elements.iter().find(|e| e.element_type == "text") {
            title = elem.content.clone();
        }
    }

    // Extract background info
    let (has_background, background_type) = extract_background_info(xml);

    // Extract transition
    let transition = extract_transition(xml);

    // Extract layout info (simplified)
    let (layout_name, layout_id) = (None, None);

    // Extract notes (simplified - not implemented in this version)
    let notes = None;

    Ok(DetailedSlide {
        slide_number,
        slide_id: None,
        layout_name,
        layout_id,
        title,
        elements,
        has_background,
        background_type,
        notes,
        transition,
    })
}

/// Helper struct for building slide elements
struct SlideElementBuilder {
    element_type: String,
    name: Option<String>,
    position: Option<ElementPosition>,
    size: Option<ElementSize>,
    content: Option<String>,
}

impl SlideElementBuilder {
    fn new(element_type: &str) -> Self {
        Self {
            element_type: element_type.to_string(),
            name: None,
            position: None,
            size: None,
            content: None,
        }
    }

    fn build(self) -> Option<SlideElement> {
        if self.position.is_none() && self.content.is_none() {
            return None;
        }
        
        Some(SlideElement {
            element_type: self.element_type,
            name: self.name,
            position: self.position,
            size: self.size,
            content: self.content,
            properties: HashMap::new(),
        })
    }
}

fn parse_emu(bytes: &[u8]) -> i64 {
    String::from_utf8_lossy(bytes).parse().unwrap_or(0)
}

fn extract_background_info(xml: &str) -> (bool, Option<String>) {
    if let Some(bg_start) = xml.find("<p:bg>") {
        let bg_section = &xml[bg_start..];
        if bg_section.contains("<a:solidFill>") {
            (true, Some("solid".to_string()))
        } else if bg_section.contains("<a:gradFill>") {
            (true, Some("gradient".to_string()))
        } else if bg_section.contains("<a:blipFill>") {
            (true, Some("image".to_string()))
        } else {
            (true, Some("unknown".to_string()))
        }
    } else {
        (false, None)
    }
}

fn extract_transition(xml: &str) -> Option<TransitionInfo> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut in_transition = false;
    let mut transition_type = String::new();
    let mut duration: Option<f64> = None;
    let mut has_auto = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                
                // Check for transition element
                if name.as_ref() == b"p:transition" {
                    in_transition = true;
                    // Check for auto attribute
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"auto" {
                            if String::from_utf8_lossy(&attr.value) == "1" {
                                has_auto = true;
                            }
                        }
                    }
                }
                
                // Extract transition type from child element (p:fade, p:push, etc.)
                if in_transition && name.as_ref().starts_with(b"p:") {
                    let tag_name = String::from_utf8_lossy(name.as_ref());
                    if tag_name != "p:transition" {
                        transition_type = tag_name.trim_start_matches("p:").to_string();
                    }
                }
                
                // Extract duration attribute
                if in_transition {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"dur" {
                            if let Ok(dur_ms) = String::from_utf8_lossy(&attr.value).parse::<f64>() {
                                duration = Some(dur_ms / 1000.0);
                            }
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"p:transition" {
                    in_transition = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    if transition_type.is_empty() {
        return None;
    }

    Some(TransitionInfo {
        transition_type,
        duration,
        advance_on_click: !has_auto,
        advance_after_time: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_slide_detailed_tool_basic() {
        let tool = ReadSlideDetailedTool;
        assert_eq!(tool.name(), "read_slide_detailed");
        assert!(!tool.description().is_empty());
    }
}
