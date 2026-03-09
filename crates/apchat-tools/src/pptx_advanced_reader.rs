//! Advanced PPTX tools for detailed slide analysis and manipulation
//!
//! This module provides tools for:
//! - Detailed slide reading with layout analysis
//! - Image manipulation (add, edit, remove)
//! - Chart and diagram manipulation
//! - Slide transitions
//! - Element animations

use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use serde::{Deserialize, Serialize};

/// Tool for reading detailed slide information including layout, elements, and media
pub struct ReadSlideDetailedTool;

#[derive(Debug, Serialize)]
struct SlideElement {
    element_type: String,
    id: Option<String>,
    name: Option<String>,
    position: Option<ElementPosition>,
    size: Option<ElementSize>,
    content: Option<String>,
    properties: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
struct ElementPosition {
    x: i64,
    y: i64,
}

#[derive(Debug, Serialize)]
struct ElementSize {
    width: i64,
    height: i64,
}

#[derive(Debug, Serialize)]
struct DetailedSlide {
    slide_number: usize,
    slide_id: Option<String>,
    layout_name: Option<String>,
    layout_id: Option<String>,
    title: Option<String>,
    elements: Vec<SlideElement>,
    has_background: bool,
    background_type: Option<String>,
    notes: Option<String>,
    transition: Option<TransitionInfo>,
}

#[derive(Debug, Serialize)]
struct TransitionInfo {
    transition_type: String,
    duration: Option<f64>,
    advance_on_click: bool,
    advance_after_time: Option<f64>,
}

#[async_trait]
impl Tool for ReadSlideDetailedTool {
    fn name(&self) -> &str {
        "read_slide_detailed"
    }

    fn description(&self) -> &str {
        "Read detailed information about a specific slide including layout, elements, images, charts, and formatting.
Parameters:
- path: Path to the PPTX file
- slide_number: 1-based slide number to read (optional, reads all slides if not specified)

Returns detailed information about:
- Slide layout and master information
- All elements (text boxes, shapes, images, charts, tables)
- Element positions and sizes
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
fn read_slide_detailed(
    archive: &mut zip::ZipArchive<Cursor<&Vec<u8>>>,
    slide_number: Option<usize>,
) -> Result<Vec<DetailedSlide>, Box<dyn std::error::Error + Send + Sync>> {
    let mut slides = Vec::new();

    // Collect all archive data upfront to avoid borrow issues
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
        
        // Skip if specific slide requested and this isn't it
        if let Some(req_num) = slide_number {
            if slide_num != req_num {
                continue;
            }
        }

        // Read slide XML
        let xml_content = String::from_utf8_lossy(&archive_data[slide_path]).to_string();

        let slide = parse_slide_detailed(&xml_content, &archive_data, slide_num)?;
        slides.push(slide);
    }

    Ok(slides)
}

/// Parse slide XML for detailed information
fn parse_slide_detailed(
    xml: &str,
    archive_data: &HashMap<String, Vec<u8>>,
    slide_number: usize,
) -> Result<DetailedSlide, Box<dyn std::error::Error + Send + Sync>> {
    use regex::Regex;

    // Extract slide ID
    let slide_id = Regex::new(r#"<p:sld[^>]*>"#)
        .unwrap()
        .captures(xml)
        .and_then(|c| c.get(0))
        .and_then(|m| {
            Regex::new(r#"id="([^"]*)""#).unwrap()
                .captures(m.as_str())
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
        });

    // Extract layout information from relationships
    let (layout_name, layout_id) = extract_layout_info(archive_data, slide_number)?;

    // Extract title
    let title = extract_title_from_slide(xml);

    // Extract all elements
    let mut elements = Vec::new();

    // Extract text shapes
    elements.extend(extract_text_shapes(xml));

    // Extract images
    elements.extend(extract_images(xml, archive_data, slide_number)?);

    // Extract charts
    elements.extend(extract_charts(xml, archive_data, slide_number)?);

    // Extract tables
    elements.extend(extract_tables(xml));

    // Extract shapes
    elements.extend(extract_shapes(xml));

    // Check for background
    let (has_background, background_type) = extract_background_info(xml);

    // Extract notes
    let notes = extract_notes(archive_data, slide_number)?;

    // Extract transition
    let transition = extract_transition(archive_data, slide_number)?;

    Ok(DetailedSlide {
        slide_number,
        slide_id,
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

fn extract_layout_info(
    archive_data: &HashMap<String, Vec<u8>>,
    slide_number: usize,
) -> Result<(Option<String>, Option<String>), Box<dyn std::error::Error + Send + Sync>> {
    let rels_path = format!("ppt/slides/_rels/slide{}.xml.rels", slide_number);
    
    if let Some(data) = archive_data.get(&rels_path) {
        let content = String::from_utf8_lossy(data).to_string();

        use regex::Regex;
        // Find slideLayout relationship
        let layout_re = Regex::new(r#"<Relationship[^>]*Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout"[^>]*Target="([^"]*)""#).unwrap();
        if let Some(caps) = layout_re.captures(&content) {
            if let Some(target) = caps.get(1) {
                let layout_path = format!("ppt/slideLayouts/{}", target.as_str());
                if let Some(layout_data) = archive_data.get(&layout_path) {
                    let layout_xml = String::from_utf8_lossy(layout_data).to_string();

                    // Extract layout name
                    let name_re = Regex::new(r#"<p:cNvPr[^>]*name="([^"]*)""#).unwrap();
                    let name = name_re.captures(&layout_xml)
                        .and_then(|c| c.get(1))
                        .map(|m| m.as_str().to_string());

                    // Extract layout ID
                    let id_re = Regex::new(r#"<p:sldLayoutId[^>]*id="([^"]*)""#).unwrap();
                    let id = id_re.captures(&layout_xml)
                        .and_then(|c| c.get(1))
                        .map(|m| m.as_str().to_string());

                    return Ok((name, id));
                }
            }
        }
    }

    Ok((None, None))
}

fn extract_title_from_slide(xml: &str) -> Option<String> {
    use regex::Regex;

    // Find title placeholder
    if let Some(title_start) = xml.find(r#"<p:ph type="title""#)
        .or_else(|| xml.find(r#"<p:ph type="ctrTitle""#))
    {
        // Find text after title placeholder
        if let Some(text_match) = Regex::new(r#"<a:t>([^<]*)</a:t>"#)
            .unwrap()
            .captures(&xml[title_start..])
            .and_then(|c| c.get(1))
        {
            return Some(text_match.as_str().to_string());
        }
    }

    None
}

fn extract_text_shapes(xml: &str) -> Vec<SlideElement> {
    let mut elements = Vec::new();
    use regex::Regex;

    // Find all text shapes
    let shape_re = Regex::new(r#"<p:sp>.*?</p:sp>"#).unwrap();

    for shape_match in shape_re.captures_iter(xml) {
        let shape_xml = shape_match.get(0).unwrap().as_str();

        let name = Regex::new(r#"<p:cNvPr[^>]*name="([^"]*)""#)
            .unwrap()
            .captures(shape_xml)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());

        // Extract position
        let (x, y) = extract_position(shape_xml);
        let (width, height) = extract_size(shape_xml);

        // Extract text content
        let mut text_content = String::new();
        for text_match in Regex::new(r#"<a:t>([^<]*)</a:t>"#).unwrap().captures_iter(shape_xml) {
            if let Some(text) = text_match.get(1) {
                text_content.push_str(text.as_str());
            }
        }

        if !text_content.is_empty() {
            elements.push(SlideElement {
                element_type: "text".to_string(),
                id: None,
                name,
                position: Some(ElementPosition { x, y }),
                size: Some(ElementSize { width, height }),
                content: Some(text_content.trim().to_string()),
                properties: HashMap::new(),
            });
        }
    }

    elements
}

fn extract_images(
    xml: &str,
    archive_data: &HashMap<String, Vec<u8>>,
    slide_number: usize,
) -> Result<Vec<SlideElement>, Box<dyn std::error::Error + Send + Sync>> {
    let mut elements = Vec::new();
    use regex::Regex;

    // Find all picture elements
    let pic_re = Regex::new(r#"<p:pic>.*?</p:pic>"#).unwrap();

    for pic_match in pic_re.captures_iter(xml) {
        let pic_xml = pic_match.get(0).unwrap().as_str();

        let name = Regex::new(r#"<p:cNvPr[^>]*name="([^"]*)""#)
            .unwrap()
            .captures(pic_xml)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());

        let (x, y) = extract_position(pic_xml);
        let (width, height) = extract_size(pic_xml);

        // Extract image relationship ID
        let image_id = Regex::new(r#"<a:blip[^>]*r:embed="([^"]*)""#)
            .unwrap()
            .captures(pic_xml)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());

        let mut properties = HashMap::new();
        if let Some(ref id) = image_id {
            // Try to get image info from relationships
            let rels_path = format!("ppt/slides/_rels/slide{}.xml.rels", slide_number);
            if let Some(data) = archive_data.get(&rels_path) {
                        let content = String::from_utf8_lossy(data).to_string();

                // Find image target
                let target_pattern = format!(r#"<Relationship Id="{}"[^>]*Target="([^"]*)""#, id);
                if let Some(caps) = Regex::new(&target_pattern).unwrap().captures(&content) {
                    if let Some(target) = caps.get(1) {
                        properties.insert("image_path".to_string(), target.as_str().to_string());

                        // Get image dimensions
                        let image_path = format!("ppt/media/{}", target.as_str());
                        if let Some(img_data) = archive_data.get(&image_path) {
                            properties.insert(
                                "image_size".to_string(),
                                format!("{} bytes", img_data.len())
                            );
                        }
                    }
                }
            }
        }

        elements.push(SlideElement {
            element_type: "image".to_string(),
            id: None,
            name,
            position: Some(ElementPosition { x, y }),
            size: Some(ElementSize { width, height }),
            content: None,
            properties,
        });
    }

    Ok(elements)
}

fn extract_charts(
    xml: &str,
    archive_data: &HashMap<String, Vec<u8>>,
    slide_number: usize,
) -> Result<Vec<SlideElement>, Box<dyn std::error::Error + Send + Sync>> {
    let mut elements = Vec::new();
    use regex::Regex;

    // Find all chart elements
    let chart_re = Regex::new(r#"<p:graphic>[^<]*<c:chart[^>]*r:id="([^"]*)""#).unwrap();

    for chart_match in chart_re.captures_iter(xml) {
        let chart_xml = chart_match.get(0).unwrap().as_str();

        let (x, y) = extract_position(chart_xml);
        let (width, height) = extract_size(chart_xml);

        let chart_id = chart_match.get(1).map(|m| m.as_str().to_string());

        let mut properties = HashMap::new();
        if let Some(ref id) = chart_id {
            properties.insert("chart_rid".to_string(), id.clone());

            // Try to get chart data info
            let rels_path = format!("ppt/slides/_rels/slide{}.xml.rels", slide_number);
            if let Some(data) = archive_data.get(&rels_path) {
                        let content = String::from_utf8_lossy(data).to_string();

                let target_pattern = format!(r#"<Relationship Id="{}"[^>]*Target="([^"]*)""#, id);
                if let Some(caps) = Regex::new(&target_pattern).unwrap().captures(&content) {
                    if let Some(target) = caps.get(1) {
                        properties.insert("chart_path".to_string(), target.as_str().to_string());
                    }
                }
            }
        }

        elements.push(SlideElement {
            element_type: "chart".to_string(),
            id: chart_id,
            name: Some("Chart".to_string()),
            position: Some(ElementPosition { x, y }),
            size: Some(ElementSize { width, height }),
            content: None,
            properties,
        });
    }

    Ok(elements)
}

fn extract_tables(xml: &str) -> Vec<SlideElement> {
    let mut elements = Vec::new();
    use regex::Regex;

    // Find table elements (in a:tbl)
    if xml.contains("<a:tbl") {
        let table_matches: Vec<_> = Regex::new(r#"<a:tbl[^>]*>.*?</a:tbl>"#)
            .unwrap()
            .captures_iter(xml)
            .collect();

        for (idx, table_match) in table_matches.iter().enumerate() {
            let table_xml = table_match.get(0).unwrap().as_str();

            // Count rows and columns
            let rows = Regex::new(r#"<a:tr[^>]*>"#).unwrap().captures_iter(table_xml).count();
            let cols = Regex::new(r#"<a:tc[^>]*>"#).unwrap().captures_iter(table_xml).count();

            let cols_per_row = if rows > 0 { cols / rows } else { 0 };

            let mut properties = HashMap::new();
            properties.insert("rows".to_string(), rows.to_string());
            properties.insert("columns".to_string(), cols_per_row.to_string());

            elements.push(SlideElement {
                element_type: "table".to_string(),
                id: None,
                name: Some(format!("Table {}", idx + 1)),
                position: None,
                size: None,
                content: Some(format!("{} rows x {} columns", rows, cols_per_row)),
                properties,
            });
        }
    }

    elements
}

fn extract_shapes(xml: &str) -> Vec<SlideElement> {
    let mut elements = Vec::new();
    use regex::Regex;

    // Find auto shapes
    let shape_re = Regex::new(r#"<p:sp[^>]*>[^<]*<p:spPr>[^<]*<a:prstGeom[^>]*prst="([^"]*)""#).unwrap();

    for shape_match in shape_re.captures_iter(xml) {
        let shape_xml = shape_match.get(0).unwrap().as_str();

        let shape_type = shape_match.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
        let name = Regex::new(r#"<p:cNvPr[^>]*name="([^"]*)""#)
            .unwrap()
            .captures(shape_xml)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());

        let (x, y) = extract_position(shape_xml);
        let (width, height) = extract_size(shape_xml);

        let mut properties = HashMap::new();
        properties.insert("shape_type".to_string(), shape_type);

        elements.push(SlideElement {
            element_type: "shape".to_string(),
            id: None,
            name,
            position: Some(ElementPosition { x, y }),
            size: Some(ElementSize { width, height }),
            content: None,
            properties,
        });
    }

    elements
}

fn extract_position(xml: &str) -> (i64, i64) {
    use regex::Regex;

    if let Some(caps) = Regex::new(r#"<a:off[^>]*x="([^"]*)"[^>]*y="([^"]*)""#)
        .unwrap()
        .captures(xml)
    {
        let x = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
        let y = caps.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
        (x, y)
    } else {
        (0, 0)
    }
}

fn extract_size(xml: &str) -> (i64, i64) {
    use regex::Regex;

    if let Some(caps) = Regex::new(r#"<a:ext[^>]*cx="([^"]*)"[^>]*cy="([^"]*)""#)
        .unwrap()
        .captures(xml)
    {
        let width = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
        let height = caps.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
        (width, height)
    } else {
        (0, 0)
    }
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
        } else if bg_section.contains("<a:pattFill>") {
            (true, Some("pattern".to_string()))
        } else {
            (true, Some("unknown".to_string()))
        }
    } else {
        (false, None)
    }
}

fn extract_notes(
    archive_data: &HashMap<String, Vec<u8>>,
    slide_number: usize,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let notes_path = format!("ppt/notesSlides/notesSlide{}.xml", slide_number);

    if let Some(data) = archive_data.get(&notes_path) {
        let content = String::from_utf8_lossy(data).to_string();

        use regex::Regex;
        let mut notes_text = String::new();
        for text_match in Regex::new(r#"<a:t>([^<]*)</a:t>"#).unwrap().captures_iter(&content) {
            if let Some(text) = text_match.get(1) {
                notes_text.push_str(text.as_str());
            }
        }

        if !notes_text.is_empty() {
            return Ok(Some(notes_text.trim().to_string()));
        }
    }

    Ok(None)
}

fn extract_transition(
    archive_data: &HashMap<String, Vec<u8>>,
    slide_number: usize,
) -> Result<Option<TransitionInfo>, Box<dyn std::error::Error + Send + Sync>> {
    // Try to read slide XML for transition info
    let slide_path = format!("ppt/slides/slide{}.xml", slide_number);

    if let Some(data) = archive_data.get(&slide_path) {
        let content = String::from_utf8_lossy(data).to_string();

        use regex::Regex;
        // Look for <p:transition>...</p:transition> block
        if let Some(transition_match) = Regex::new(r#"(?s)<p:transition>(.*?)</p:transition>"#)
            .unwrap()
            .captures(&content)
        {
            let transition_inner = transition_match.get(1).unwrap().as_str();

            // Extract transition type from child element (e.g., <p:fade dur="800"/>)
            let transition_type = Regex::new(r#"<p:(\w+)"#)
                .unwrap()
                .captures(transition_inner)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| "none".to_string());

            // Extract duration from dur attribute
            let duration = Regex::new(r#"dur="(\d+)"#)
                .unwrap()
                .captures(transition_inner)
                .and_then(|c| c.get(1))
                .and_then(|m| m.as_str().parse::<f64>().ok())
                .map(|ms| ms / 1000.0); // Convert ms to seconds

            // Check for auto-advance
            let advance_on_click = !transition_inner.contains("auto=\"1\"");
            let advance_after_time = None; // Not currently supported in our implementation

            return Ok(Some(TransitionInfo {
                transition_type,
                duration,
                advance_on_click,
                advance_after_time,
            }));
        }
    }

    Ok(None)
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
