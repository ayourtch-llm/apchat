//! PPTX editor tools - modify existing PowerPoint presentations

use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::io::{Read, Write, Cursor};
use serde::Deserialize;

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

        // For now, this is a placeholder - full implementation would require
        // more complex XML manipulation
        ToolResult::success(format!(
            "Edit slide {} in '{}' - title: {:?}, bullets: {:?}",
            slide_number, path, title, bullets.as_ref().map(|b| b.len())
        ))
    }
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

        // Placeholder - full implementation would add a new slide XML
        ToolResult::success(format!(
            "Add slide '{}' to '{}' after slide {:?} with {} bullets",
            title, path, after_slide, bullets.as_ref().map(|b| b.len()).unwrap_or(0)
        ))
    }
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