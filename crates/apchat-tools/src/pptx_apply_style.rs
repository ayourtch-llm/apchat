//! Tool for applying a template/style to an existing PPTX presentation
//!
//! This tool takes an existing presentation and re-applies it using a template,
//! preserving the slide content while adopting the template's theme, layouts, and styling.

use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use quick_xml::events::Event;
use quick_xml::Reader;

/// Tool for applying a template style to an existing presentation
pub struct ApplyPptxStyleTool;

#[async_trait]
impl Tool for ApplyPptxStyleTool {
    fn name(&self) -> &str {
        "apply_pptx_style"
    }

    fn description(&self) -> &str {
        "Apply a template's style/theme to an existing PPTX presentation.
This tool takes an existing presentation and recreates it using a template file,
preserving all slide content (titles, bullet points) while adopting the template's:
- Theme colors and fonts
- Slide masters and layouts
- Logos and branding elements
- Overall visual style

Parameters:
- input_path: Path to the existing PPTX presentation to restyle
- template_path: Path to the template PPTX/PPTM file to apply
- output_path: Path where the restyled presentation will be saved"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("input_path", "string", "Path to the existing PPTX presentation to restyle", required),
            param!("template_path", "string", "Path to the template PPTX/PPTM file to apply", required),
            param!("output_path", "string", "Path where the restyled presentation will be saved", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        // Parse parameters
        let input_path = match params.get_required::<String>("input_path") {
            Ok(p) => p,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let template_path = match params.get_required::<String>("template_path") {
            Ok(p) => p,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let output_path = match params.get_required::<String>("output_path") {
            Ok(p) => p,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        // Read and parse the input presentation
        let input_full_path = context.work_dir.join(&input_path);
        if !input_full_path.exists() {
            return ToolResult::error(format!("Input presentation not found: {}", input_path));
        }

        let input_data = match std::fs::read(&input_full_path) {
            Ok(data) => data,
            Err(e) => return ToolResult::error(format!("Failed to read input presentation: {}", e)),
        };

        // Extract slide content from input presentation
        let mut input_archive = match zip::ZipArchive::new(Cursor::new(&input_data)) {
            Ok(archive) => archive,
            Err(e) => return ToolResult::error(format!("Failed to open input presentation: {}", e)),
        };

        // Extract slides from input using quick-xml
        let slides = match extract_slides_from_pptx(&mut input_archive) {
            Ok(slides) => slides,
            Err(e) => return ToolResult::error(format!("Failed to extract slides: {}", e)),
        };

        if slides.is_empty() {
            return ToolResult::error("No slides found in input presentation".to_string());
        }

        // Convert extracted slides to ppt-rs format
        let pptx_slides: Vec<ppt_rs::generator::SlideContent> = slides
            .iter()
            .map(|s| {
                let mut slide_content = ppt_rs::generator::SlideContent::new(&s.title);
                for bullet in &s.bullets {
                    slide_content = slide_content.add_bullet(bullet);
                }
                slide_content
            })
            .collect();

        let is_title_slide: Vec<bool> = vec![false; pptx_slides.len()];

        // Read and parse the template
        let template_full_path = context.work_dir.join(&template_path);
        if !template_full_path.exists() {
            return ToolResult::error(format!("Template not found: {}", template_path));
        }

        let template_data = match std::fs::read(&template_full_path) {
            Ok(data) => data,
            Err(e) => return ToolResult::error(format!("Failed to read template: {}", e)),
        };

        // Extract metadata from template using quick-xml
        let mut template_archive = match zip::ZipArchive::new(Cursor::new(&template_data)) {
            Ok(archive) => archive,
            Err(e) => return ToolResult::error(format!("Failed to open template: {}", e)),
        };

        let (title, author) = match extract_template_metadata(&mut template_archive) {
            Ok(metadata) => metadata,
            Err(e) => return ToolResult::error(format!("Failed to extract template metadata: {}", e)),
        };

        // Generate new presentation using ppt-rs
        match apply_template_to_slides(&title, &author, &pptx_slides, &is_title_slide, &template_path, crate::template_impl::SlideSize::Screen16x9, context) {
            Ok(_) => ToolResult::success(format!(
                "Successfully applied template style from '{}' to '{}', output: '{}'",
                template_path, input_path, output_path
            )),
            Err(e) => ToolResult::error(format!("Failed to apply template: {}", e)),
        }
    }
}

/// Extracted slide data
#[derive(Debug)]
struct ExtractedSlide {
    slide_number: usize,
    title: String,
    bullets: Vec<String>,
}

/// Extract all slides from a PPTX using quick-xml for XML parsing
fn extract_slides_from_pptx(
    archive: &mut zip::ZipArchive<Cursor<&Vec<u8>>>,
) -> Result<Vec<ExtractedSlide>, Box<dyn std::error::Error + Send + Sync>> {
    let mut slides = Vec::new();

    // Collect slide files
    let mut slide_files: Vec<String> = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
            slide_files.push(name);
        }
    }

    slide_files.sort();

    // Parse each slide
    for slide_file in slide_files {
        let mut entry = archive.by_name(&slide_file)?;
        let mut xml_content = String::new();
        entry.read_to_string(&mut xml_content)?;

        match parse_slide_xml(&xml_content) {
            Ok(slide) => slides.push(slide),
            Err(e) => eprintln!("Warning: Failed to parse {}: {}", slide_file, e),
        }
    }

    Ok(slides)
}

/// Parse slide XML using quick-xml to extract title and bullet points
fn parse_slide_xml(xml: &str) -> Result<ExtractedSlide, Box<dyn std::error::Error + Send + Sync>> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut title = String::new();
    let mut bullets: Vec<String> = Vec::new();
    
    // Track shape elements and their names
    let mut in_candidate_shape = false;
    let mut candidate_depth = 0;
    let mut shape_is_title = false;
    let mut shape_is_body = false;
    
    let mut in_text = false;
    let mut current_text = String::new();
    let mut slide_number = 0usize;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                
                // Check for slide ID attribute
                if name.as_ref() == b"p:sldId" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"id" {
                            if let Ok(num) = String::from_utf8_lossy(&attr.value).parse::<usize>() {
                                slide_number = num;
                            }
                        }
                    }
                }
                
                // Check if entering a shape element
                if name.as_ref() == b"p:sp" || name.as_ref() == b"p:pic" {
                    in_candidate_shape = true;
                    candidate_depth = 1;
                    shape_is_title = false;
                    shape_is_body = false;
                } else if in_candidate_shape {
                    candidate_depth += 1;
                    
                    // Check shape name from cNvPr element
                    if name.as_ref() == b"p:cNvPr" {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"name" {
                                let shape_name = String::from_utf8_lossy(&attr.value).to_lowercase();
                                if shape_name.contains("title") {
                                    shape_is_title = true;
                                } else if shape_name.contains("content") || shape_name.contains("body") {
                                    shape_is_body = true;
                                }
                            }
                        }
                    }
                }
                
                // Track text elements in title or body shapes
                if (shape_is_title || shape_is_body) && name.as_ref() == b"a:t" {
                    in_text = true;
                    current_text.clear();
                }
            }
            Ok(Event::Empty(ref e)) => {
                let name = e.name();
                
                // Handle self-closing p:cNvPr tags
                if in_candidate_shape && name.as_ref() == b"p:cNvPr" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"name" {
                            let shape_name = String::from_utf8_lossy(&attr.value).to_lowercase();
                            if shape_name.contains("title") {
                                shape_is_title = true;
                            } else if shape_name.contains("content") || shape_name.contains("body") {
                                shape_is_body = true;
                            }
                        }
                    }
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_text {
                    current_text.push_str(&String::from_utf8_lossy(e));
                }
            }
            Ok(Event::End(ref e)) => {
                if in_candidate_shape {
                    candidate_depth -= 1;
                    if candidate_depth == 0 {
                        in_candidate_shape = false;
                        shape_is_title = false;
                        shape_is_body = false;
                    }
                }
                
                if e.name().as_ref() == b"a:t" && in_text {
                    in_text = false;
                    let t = current_text.trim().to_string();
                    if !t.is_empty() {
                        if shape_is_title && title.is_empty() {
                            title = t;
                        } else if shape_is_body {
                            bullets.push(t);
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parsing error: {}", e).into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(ExtractedSlide {
        slide_number,
        title,
        bullets,
    })
}

/// Extract template metadata (title, author) using quick-xml
fn extract_template_metadata(
    archive: &mut zip::ZipArchive<Cursor<&Vec<u8>>>,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let mut title = String::from("Presentation");
    let mut author = String::from("pptx-rs");

    // Try to read core.xml for metadata
    if let Ok(mut entry) = archive.by_name("docProps/core.xml") {
        let mut xml_content = String::new();
        if entry.read_to_string(&mut xml_content).is_ok() {
            let mut reader = Reader::from_str(&xml_content);
            reader.trim_text(true);
            let mut buf = Vec::new();
            let mut in_title = false;
            let mut in_creator = false;
            let mut current_text = String::new();

            loop {
                match reader.read_event_into(&mut buf) {
                    Ok(Event::Start(ref e)) => {
                        let name = e.name();
                        if name.as_ref() == b"dc:title" {
                            in_title = true;
                            current_text.clear();
                        } else if name.as_ref() == b"dc:creator" {
                            in_creator = true;
                            current_text.clear();
                        }
                    }
                    Ok(Event::Text(ref e)) => {
                        if in_title || in_creator {
                            current_text.push_str(&String::from_utf8_lossy(e));
                        }
                    }
                    Ok(Event::End(ref e)) => {
                        let name = e.name();
                        if name.as_ref() == b"dc:title" && in_title {
                            in_title = false;
                            if !current_text.trim().is_empty() {
                                title = current_text.trim().to_string();
                            }
                        } else if name.as_ref() == b"dc:creator" && in_creator {
                            in_creator = false;
                            if !current_text.trim().is_empty() {
                                author = current_text.trim().to_string();
                            }
                        }
                    }
                    Ok(Event::Eof) => break,
                    Err(_) => break,
                    _ => {}
                }
                buf.clear();
            }
        }
    }

    Ok((title, author))
}

/// Apply template to slides using the existing template implementation
fn apply_template_to_slides(
    title: &str,
    author: &str,
    slides: &[ppt_rs::generator::SlideContent],
    is_title_slide: &[bool],
    template_path: &str,
    slide_size: crate::template_impl::SlideSize,
    context: &ToolContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::template_impl::create_presentation_from_template;
    
    match create_presentation_from_template(title, author, slides, is_title_slide, template_path, slide_size, context) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Template application failed: {}", e).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_pptx_style_tool_basic() {
        let tool = ApplyPptxStyleTool;
        assert_eq!(tool.name(), "apply_pptx_style");
        assert!(!tool.description().is_empty());
    }
    
    #[test]
    fn test_parse_slide_xml_with_shape_names() {
        // Test ppt_rs-generated slides that use shape names instead of placeholders
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld>
<p:spTree>
<p:sp>
<p:nvSpPr>
<p:cNvPr id="2" name="Title"/>
</p:nvSpPr>
<p:txBody>
<a:p><a:r><a:t>My Title</a:t></a:r></a:p>
</p:txBody>
</p:sp>
<p:sp>
<p:nvSpPr>
<p:cNvPr id="3" name="Content"/>
</p:nvSpPr>
<p:txBody>
<a:p><a:r><a:t>Bullet 1</a:t></a:r></a:p>
<a:p><a:r><a:t>Bullet 2</a:t></a:r></a:p>
<a:p><a:r><a:t>Bullet 3</a:t></a:r></a:p>
</p:txBody>
</p:sp>
</p:spTree>
</p:cSld>
</p:sld>"#;

        let slide = parse_slide_xml(slide_xml).expect("Failed to parse slide XML");
        assert_eq!(slide.title, "My Title", "Should extract title from shape named 'Title'");
        assert_eq!(slide.bullets.len(), 3, "Should extract 3 bullets from shape named 'Content'");
        assert_eq!(slide.bullets[0], "Bullet 1");
        assert_eq!(slide.bullets[1], "Bullet 2");
        assert_eq!(slide.bullets[2], "Bullet 3");
    }
    
    #[test]
    fn test_parse_slide_xml_with_placeholder_type() {
        // Test traditional slides that use placeholder types
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld>
<p:spTree>
<p:sp>
<p:nvSpPr>
<p:cNvPr id="2" name="Title"/>
<p:nvPr><p:ph type="title"/></p:nvPr>
</p:nvSpPr>
<p:txBody>
<a:p><a:r><a:t>Placeholder Title</a:t></a:r></a:p>
</p:txBody>
</p:sp>
<p:sp>
<p:nvSpPr>
<p:cNvPr id="3" name="Content"/>
<p:nvPr><p:ph type="body"/></p:nvPr>
</p:nvSpPr>
<p:txBody>
<a:p><a:r><a:t>Bullet A</a:t></a:r></a:p>
<a:p><a:r><a:t>Bullet B</a:t></a:r></a:p>
</p:txBody>
</p:sp>
</p:spTree>
</p:cSld>
</p:sld>"#;

        let slide = parse_slide_xml(slide_xml).expect("Failed to parse slide XML");
        assert_eq!(slide.title, "Placeholder Title");
        assert_eq!(slide.bullets.len(), 2);
        assert_eq!(slide.bullets[0], "Bullet A");
        assert_eq!(slide.bullets[1], "Bullet B");
    }
    
    #[test]
    fn test_parse_slide_xml_with_self_closing_cnvpr() {
        // Test slides with self-closing p:cNvPr tags
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld>
<p:spTree>
<p:sp>
<p:nvSpPr><p:cNvPr id="2" name="Title 1"/></p:nvSpPr>
<p:txBody><a:p><a:r><a:t>Self-closing Title</a:t></a:r></a:p></p:txBody>
</p:sp>
<p:sp>
<p:nvSpPr><p:cNvPr id="3" name="Body Placeholder 5"/></p:nvSpPr>
<p:txBody><a:p><a:r><a:t>Content from body shape</a:t></a:r></a:p></p:txBody>
</p:sp>
</p:spTree>
</p:cSld>
</p:sld>"#;

        let slide = parse_slide_xml(slide_xml).expect("Failed to parse slide XML");
        assert_eq!(slide.title, "Self-closing Title");
        assert_eq!(slide.bullets.len(), 1);
        assert_eq!(slide.bullets[0], "Content from body shape");
    }
}
