//! Tool for applying a template/style to an existing PPTX presentation
//!
//! This tool takes an existing presentation and re-applies it using a template,
//! preserving the slide content while adopting the template's theme, layouts, and styling.

use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

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

        // Extract slides from input
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

        // Get presentation title from metadata or first slide
        let title = extract_title_from_pptx(&mut input_archive).unwrap_or_else(|_| {
            slides.first().map(|s| s.title.clone()).unwrap_or_else(|| "Presentation".to_string())
        });

        let author = extract_author_from_pptx(&mut input_archive).unwrap_or_else(|_| "Unknown".to_string());

        // Apply template using the existing function
        match apply_template_to_slides(&title, &author, &pptx_slides, &is_title_slide, &template_path, context) {
            Ok(pptx_data) => {
                let output_full_path = context.work_dir.join(&output_path);
                match std::fs::write(&output_full_path, &pptx_data) {
                    Ok(()) => ToolResult::success(format!(
                        "Successfully applied template '{}' to '{}' and saved to '{}'",
                        template_path, input_path, output_path
                    )),
                    Err(e) => ToolResult::error(format!("Failed to save output presentation: {}", e)),
                }
            }
            Err(e) => ToolResult::error(format!("Failed to apply template: {}", e)),
        }
    }
}

/// Extract slide information from a PPTX archive
fn extract_slides_from_pptx(
    archive: &mut zip::ZipArchive<Cursor<&Vec<u8>>>,
) -> Result<Vec<ExtractedSlide>, Box<dyn std::error::Error + Send + Sync>> {
    let mut slides = Vec::new();

    // Find all slide files
    for i in 0..archive.len() {
        let entry_name = archive.by_index(i)?.name().to_string();
        if entry_name.starts_with("ppt/slides/slide") && entry_name.ends_with(".xml") {
            let mut entry = archive.by_index(i)?;
            let mut xml_content = String::new();
            entry.read_to_string(&mut xml_content)?;

            // Parse slide XML to extract title and bullets
            let slide = parse_slide_xml(&xml_content)?;
            slides.push(slide);
        }
    }

    // Sort slides by number
    slides.sort_by_key(|s| s.slide_number);

    Ok(slides)
}

/// Extracted slide data
#[derive(Debug)]
struct ExtractedSlide {
    slide_number: usize,
    title: String,
    bullets: Vec<String>,
}

/// Parse slide XML to extract title and bullet points
fn parse_slide_xml(xml: &str) -> Result<ExtractedSlide, Box<dyn std::error::Error + Send + Sync>> {
    use regex::Regex;

    // Extract slide number from the XML
    let id_re = Regex::new(r#"<p:sldId id="(\d+)""#).unwrap();
    let slide_number = id_re
        .captures(xml)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<usize>().ok())
        .unwrap_or(0);

    // Extract title - look for title placeholder
    let title = extract_text_from_placeholder(xml, "title", "ctrTitle");

    // Extract bullets - look for body placeholder content
    let bullets = extract_bullets_from_body(xml);

    Ok(ExtractedSlide {
        slide_number,
        title,
        bullets,
    })
}

/// Extract text from a placeholder by type
fn extract_text_from_placeholder(xml: &str, placeholder_type: &str, _alt_type: &str) -> String {
    use regex::Regex;

    // Find the placeholder section
    let ph_re = Regex::new(&format!(r#"<p:ph type="{}"/>([^<]*(?:<[^/][^>]*>[^<]*)*)</p:nvPr>"#, placeholder_type)).unwrap();
    
    if let Some(captures) = ph_re.captures(xml) {
        if let Some(section) = captures.get(1) {
            // Extract text from the following <p:txBody> section
            let start_pos = section.end() + xml[section.range()].len();
            let remaining = &xml[start_pos..];
            
            // Find the text within <a:t> tags following this placeholder
            let text_re = Regex::new(r#"<a:t>([^<]*)</a:t>"#).unwrap();
            if let Some(text_match) = text_re.captures(remaining) {
                if let Some(text) = text_match.get(1) {
                    return text.as_str().trim().to_string();
                }
            }
        }
    }

    // Fallback: try to find any text in <a:t> tags
    let text_re = Regex::new(r#"<a:t>([^<]+)</a:t>"#).unwrap();
    if let Some(captures) = text_re.captures(xml) {
        if let Some(text) = captures.get(1) {
            return text.as_str().trim().to_string();
        }
    }

    String::new()
}

/// Extract bullet points from body placeholder
fn extract_bullets_from_body(xml: &str) -> Vec<String> {
    use regex::Regex;

    let mut bullets = Vec::new();
    let mut first_para_skipped = false;
    
    // Find all bullet paragraphs
    // Use (?s) flag to make . match newlines (DOTALL mode)
    let bullet_re = Regex::new(r#"(?s)<a:p[^>]*>(.*?)</a:p>"#).unwrap();
    
    for captures in bullet_re.captures_iter(xml) {
        if let Some(para) = captures.get(1) {
            // Skip the first paragraph (which is typically the title)
            if !first_para_skipped {
                first_para_skipped = true;
                continue;
            }
            
            // Extract text from <a:t> tags within this paragraph
            let text_re = Regex::new(r#"<a:t>([^<]+)</a:t>"#).unwrap();
            for text_capture in text_re.captures_iter(para.as_str()) {
                if let Some(text) = text_capture.get(1) {
                    let text = text.as_str().trim();
                    if !text.is_empty() {
                        bullets.push(text.to_string());
                    }
                }
            }
        }
    }

    bullets
}

/// Extract presentation title from docProps/core.xml
fn extract_title_from_pptx(
    archive: &mut zip::ZipArchive<Cursor<&Vec<u8>>>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if let Ok(mut entry) = archive.by_name("docProps/core.xml") {
        let mut content = String::new();
        entry.read_to_string(&mut content)?;
        
        use regex::Regex;
        let title_re = Regex::new(r#"<cp:title[^>]*>([^<]*)</cp:title>"#).unwrap();
        if let Some(captures) = title_re.captures(&content) {
            if let Some(title) = captures.get(1) {
                return Ok(title.as_str().trim().to_string());
            }
        }
    }
    
    Err("Title not found".into())
}

/// Extract author from docProps/core.xml
fn extract_author_from_pptx(
    archive: &mut zip::ZipArchive<Cursor<&Vec<u8>>>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if let Ok(mut entry) = archive.by_name("docProps/core.xml") {
        let mut content = String::new();
        entry.read_to_string(&mut content)?;
        
        use regex::Regex;
        let author_re = Regex::new(r#"<cp:creator[^>]*>([^<]*)</cp:creator>"#).unwrap();
        if let Some(captures) = author_re.captures(&content) {
            if let Some(author) = captures.get(1) {
                return Ok(author.as_str().trim().to_string());
            }
        }
    }
    
    Err("Author not found".into())
}

/// Apply template to slides using the existing template implementation
fn apply_template_to_slides(
    title: &str,
    author: &str,
    slides: &[ppt_rs::generator::SlideContent],
    is_title_slide: &[bool],
    template_path: &str,
    context: &ToolContext,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    use crate::template_impl::create_presentation_from_template;
    create_presentation_from_template(title, author, slides, is_title_slide, template_path, context)
}