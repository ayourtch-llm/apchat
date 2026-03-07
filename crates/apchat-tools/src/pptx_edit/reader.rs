//! PPTX reader tool - read and extract information from PowerPoint presentations

use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::io::Read;
use serde::{Deserialize, Serialize};

/// Tool for reading PPTX presentations
pub struct ReadPptxTool;

/// Information about a single slide
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PptxSlideInfo {
    pub slide_number: usize,
    pub title: Option<String>,
    pub bullet_count: usize,
    pub has_image: bool,
    pub has_chart: bool,
    pub has_table: bool,
    pub notes: Option<String>,
}

/// Information about the entire presentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PptxPresentationInfo {
    pub path: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub slide_count: usize,
    pub slides: Vec<PptxSlideInfo>,
}

#[async_trait]
impl Tool for ReadPptxTool {
    fn name(&self) -> &str {
        "read_pptx"
    }

    fn description(&self) -> &str {
        "Read a PPTX presentation file and extract its structure and content.
Returns information about:
- Presentation metadata (title, author)
- Number of slides
- Each slide's title, bullet points, images, charts, tables, and notes

Use this tool to inspect existing presentations before editing them."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("path", "string", "Path to the PPTX file to read (e.g., 'presentation.pptx')", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let path = match params.get_required::<String>("path") {
            Ok(p) => p,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let full_path = context.work_dir.join(&path);

        // Check if file exists
        if !full_path.exists() {
            return ToolResult::error(format!("File not found: {}", path));
        }

        // Read the file as ZIP
        let file = match std::fs::File::open(&full_path) {
            Ok(f) => f,
            Err(e) => return ToolResult::error(format!("Failed to open file: {}", e)),
        };

        let mut archive = match zip::ZipArchive::new(file) {
            Ok(a) => a,
            Err(e) => return ToolResult::error(format!("Failed to read ZIP: {}", e)),
        };

        let mut info = PptxPresentationInfo {
            path: path.clone(),
            title: None,
            author: None,
            slide_count: 0,
            slides: Vec::new(),
        };

        // Try to read core properties (title, author)
        if let Ok(mut core_file) = archive.by_name("docProps/core.xml") {
            let mut content = String::new();
            let _ = core_file.read_to_string(&mut content);
            
            if let Some(title_end) = content.find("</dc:title>") {
                if let Some(title_start) = content.rfind("<dc:title>") {
                    let title_start = title_start + "<dc:title>".len();
                    if title_start < title_end {
                        info.title = Some(content[title_start..title_end].to_string());
                    }
                }
            }
            
            if let Some(author_end) = content.find("</dc:creator>") {
                if let Some(author_start) = content.rfind("<dc:creator>") {
                    let author_start = author_start + "<dc:creator>".len();
                    if author_start < author_end {
                        info.author = Some(content[author_start..author_end].to_string());
                    }
                }
            }
        }

        // Count and read slides
        let mut slide_count = 0;
        loop {
            let slide_path = format!("ppt/slides/slide{}.xml", slide_count + 1);
            let slide_content = match archive.by_name(&slide_path) {
                Ok(slide_file) => {
                    let mut buf = String::new();
                    let mut file = slide_file;
                    let _ = file.read_to_string(&mut buf);
                    Some((slide_count + 1, buf))
                }
                Err(_) => break,
            };

            if let Some((num, content)) = slide_content {
                slide_count = num;

                let slide_info = PptxSlideInfo {
                    slide_number: slide_count,
                    title: extract_text_between(&content, "<a:t>", "</a:t>"),
                    bullet_count: content.matches("<a:p>").count(),
                    has_image: content.contains("<a:blip "),
                    has_chart: content.contains("chart"),
                    has_table: content.contains("<a:tbl"),
                    notes: None,
                };

                info.slides.push(slide_info);

                // Try to read notes
                let notes_path = format!("ppt/notesSlides/notesSlide{}.xml", slide_count);
                if let Ok(notes_file) = archive.by_name(&notes_path) {
                    let notes_content = {
                        let mut buf = String::new();
                        let mut file = notes_file;
                        let _ = file.read_to_string(&mut buf);
                        buf
                    };
                    if let Some(notes_text) = extract_text_between(&notes_content, "<a:t>", "</a:t>") {
                        if let Some(slide) = info.slides.last_mut() {
                            slide.notes = Some(notes_text);
                        }
                    }
                }
            }
        }

        info.slide_count = slide_count;

        // Convert to JSON and return
        match serde_json::to_string_pretty(&info) {
            Ok(json) => ToolResult::success(json),
            Err(e) => ToolResult::error(format!("Failed to serialize presentation info: {}", e).to_string()),
        }
    }
}

/// Helper function to extract text between two tags
fn extract_text_between(content: &str, start_tag: &str, end_tag: &str) -> Option<String> {
    if let Some(start_pos) = content.find(start_tag) {
        let text_start = start_pos + start_tag.len();
        if let Some(end_pos) = content[text_start..].find(end_tag) {
            let text_end = text_start + end_pos;
            let text = content[text_start..text_end].trim().to_string();
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_pptx_tool_basic() {
        let tool = ReadPptxTool;
        assert_eq!(tool.name(), "read_pptx");
        assert!(!tool.description().is_empty());
        
        let params = tool.parameters();
        assert!(params.contains_key("path"));
    }
}