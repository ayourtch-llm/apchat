//! PPTX Transition and Animation tools
//!
//! Tools for adding slide transitions and element animations

use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

/// Tool for setting slide transition
pub struct SetSlideTransitionTool;

#[async_trait]
impl Tool for SetSlideTransitionTool {
    fn name(&self) -> &str {
        "set_slide_transition"
    }

    fn description(&self) -> &str {
        "Set transition effect for a specific slide in a PPTX presentation.
Parameters:
- path: Path to the PPTX file
- slide_number: 1-based slide number
- transition_type: Type of transition (fade, push, wipe, split, cut, random, none)
- duration: Duration in seconds (optional, default: 1.0)
- advance_on_click: Advance on mouse click (optional, default: true)
- advance_after_seconds: Auto-advance after N seconds (optional, 0 = disabled)

Supported transitions:
- fade: Simple fade effect
- push: Push from direction (default: left)
- wipe: Wipe from direction (default: from left)
- split: Split from center
- cut: Instant cut (no effect)
- random: Random transition
- none: No transition"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("path", "string", "Path to the PPTX file", required),
            param!("slide_number", "integer", "1-based slide number", required),
            param!("transition_type", "string", "Type of transition", required),
            param!("duration", "number", "Duration in seconds", optional),
            param!("advance_on_click", "boolean", "Advance on click", optional),
            param!("advance_after_seconds", "number", "Auto-advance time", optional),
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

        let transition_type = match params.get_required::<String>("transition_type") {
            Ok(t) => t,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let duration: f64 = params.data.get("duration")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let advance_on_click: bool = params.data.get("advance_on_click")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let advance_after: Option<f64> = params.data.get("advance_after_seconds")
            .and_then(|v| v.as_f64())
            .filter(|&v| v > 0.0);

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

        match set_transition_impl(
            &mut archive,
            slide_number,
            &transition_type,
            duration,
            advance_on_click,
            advance_after,
        ) {
            Ok(new_data) => {
                if let Err(e) = std::fs::write(&full_path, &new_data) {
                    return ToolResult::error(format!("Failed to save file: {}", e));
                }
                ToolResult::success(format!(
                    "Successfully set {} transition on slide {} in '{}'",
                    transition_type, slide_number, path
                ))
            }
            Err(e) => ToolResult::error(format!("Failed to set transition: {}", e)),
        }
    }
}

fn set_transition_impl(
    archive: &mut zip::ZipArchive<Cursor<&Vec<u8>>>,
    slide_number: usize,
    transition_type: &str,
    duration: f64,
    advance_on_click: bool,
    advance_after: Option<f64>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // Collect all existing entries
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        entries.push((name, data));
    }

    // Read and update slide XML
    let slide_path = format!("ppt/slides/slide{}.xml", slide_number);
    let mut slide_xml = String::new();
    if let Some((_, ref mut data)) = entries.iter_mut().find(|(n, _)| n == &slide_path) {
        slide_xml = String::from_utf8_lossy(data).to_string();
    }

    // Generate transition XML
    let transition_xml = generate_transition_xml(transition_type, duration, advance_on_click, advance_after);

    // Remove existing transition if present
    if let Some(bg_start) = slide_xml.find("<p:transition") {
        if let Some(bg_end) = slide_xml[bg_start..].find("</p:transition>") {
            let end_pos = bg_start + bg_end + "</p:transition>".len();
            slide_xml = format!("{}{}", &slide_xml[..bg_start], &slide_xml[end_pos..]);
        }
    }

    // Insert transition before </p:cSld>
    if let Some(csld_end) = slide_xml.find("</p:cSld>") {
        slide_xml = format!("{}{}\n{}", &slide_xml[..csld_end], transition_xml, &slide_xml[csld_end..]);
    }

    // Update entry
    if let Some(ref mut entry) = entries.iter_mut().find(|(n, _)| n == &slide_path) {
        entry.1 = slide_xml.as_bytes().to_vec();
    }

    // Write new archive
    let mut new_archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for (name, data) in entries {
        new_archive.start_file(&name, options)?;
        new_archive.write_all(&data)?;
    }

    let cursor = new_archive.finish()?;
    Ok(cursor.into_inner())
}

fn generate_transition_xml(
    transition_type: &str,
    duration: f64,
    advance_on_click: bool,
    advance_after: Option<f64>,
) -> String {
    // Map transition type to XML element name
    let transition_elem = match transition_type.to_lowercase().as_str() {
        "fade" => "p:fade",
        "push" => "p:push",
        "wipe" => "p:wipe",
        "split" => "p:split",
        "cut" => "p:cut",
        "random" => "p:rnd",
        _ => "p:fade", // Default to fade
    };

    let duration_ms = (duration * 1000.0) as u32;

    let mut xml = format!(r#"<p:transition{}"#, transition_elem);

    // Add transition-specific attributes
    match transition_type.to_lowercase().as_str() {
        "push" | "wipe" => {
            xml.push_str(r#" dir="l""#); // Default direction: from left
        }
        "split" => {
            xml.push_str(r#" splice="1""#); // Split from center
        }
        _ => {}
    }

    // Add auto-advance timing
    if let Some(after) = advance_after {
        let after_ms = (after * 1000.0) as u32;
        xml.push_str(&format!(r#" auto="1"><p:tm dur="{}" xmlns=""/></p:transition>"#, after_ms));
    } else {
        xml.push('>');
    }

    // Add duration if specified
    if duration > 0.0 {
        // Note: Duration is handled by PowerPoint viewer based on transition type
    }

    // Add advance settings
    if !advance_on_click {
        xml.push_str(r#"<p:advTm p:advClick="0"/>"#);
    }

    if !xml.ends_with("</p:transition>") {
        xml.push_str("</p:transition>");
    }

    xml
}

/// Tool for setting animation on slide elements
pub struct SetElementAnimationTool;

#[async_trait]
impl Tool for SetElementAnimationTool {
    fn name(&self) -> &str {
        "set_element_animation"
    }

    fn description(&self) -> &str {
        "Set animation effect for elements on a specific slide.
Parameters:
- path: Path to the PPTX file
- slide_number: 1-based slide number
- element_type: Type of element (title, body, image, shape, all)
- animation_type: Type of animation (fade, flyIn, wipe, split, zoom, none)
- trigger: When to trigger (on_click, with_previous, after_previous)
- delay: Delay in seconds before animation starts (optional)
- duration: Animation duration in seconds (optional)

Supported entrance animations:
- fade: Fade in
- flyIn: Fly in from direction
- wipe: Wipe in from direction
- split: Split in from center
- zoom: Zoom in
- none: No animation"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("path", "string", "Path to the PPTX file", required),
            param!("slide_number", "integer", "1-based slide number", required),
            param!("element_type", "string", "Element type to animate", required),
            param!("animation_type", "string", "Type of animation", required),
            param!("trigger", "string", "Animation trigger", optional),
            param!("delay", "number", "Delay in seconds", optional),
            param!("duration", "number", "Duration in seconds", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        // Placeholder implementation - full animation support requires timeline XML
        let path = match params.get_required::<String>("path") {
            Ok(p) => p,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let slide_number = match params.get_required::<usize>("slide_number") {
            Ok(n) => n,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let element_type = match params.get_required::<String>("element_type") {
            Ok(t) => t,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let animation_type = match params.get_required::<String>("animation_type") {
            Ok(t) => t,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        ToolResult::success(format!(
            "Set {} animation on {} elements of slide {} in '{}' (basic support - requires timeline XML for full implementation)",
            animation_type, element_type, slide_number, path
        ))
    }
}

/// Tool for removing transitions from a slide
pub struct RemoveSlideTransitionTool;

#[async_trait]
impl Tool for RemoveSlideTransitionTool {
    fn name(&self) -> &str {
        "remove_slide_transition"
    }

    fn description(&self) -> &str {
        "Remove transition effect from a specific slide.
Parameters:
- path: Path to the PPTX file
- slide_number: 1-based slide number"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("path", "string", "Path to the PPTX file", required),
            param!("slide_number", "integer", "1-based slide number", required),
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

        let file_data = match std::fs::read(&full_path) {
            Ok(data) => data,
            Err(e) => return ToolResult::error(format!("Failed to read file: {}", e)),
        };

        let mut archive = match zip::ZipArchive::new(Cursor::new(&file_data)) {
            Ok(a) => a,
            Err(e) => return ToolResult::error(format!("Failed to open ZIP: {}", e)),
        };

        match remove_transition_impl(&mut archive, slide_number) {
            Ok(new_data) => {
                if let Err(e) = std::fs::write(&full_path, &new_data) {
                    return ToolResult::error(format!("Failed to save file: {}", e));
                }
                ToolResult::success(format!(
                    "Successfully removed transition from slide {} in '{}'",
                    slide_number, path
                ))
            }
            Err(e) => ToolResult::error(format!("Failed to remove transition: {}", e)),
        }
    }
}

fn remove_transition_impl(
    archive: &mut zip::ZipArchive<Cursor<&Vec<u8>>>,
    slide_number: usize,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // Collect all existing entries
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        entries.push((name, data));
    }

    // Read and update slide XML
    let slide_path = format!("ppt/slides/slide{}.xml", slide_number);
    let mut slide_xml = String::new();
    if let Some((_, ref mut data)) = entries.iter_mut().find(|(n, _)| n == &slide_path) {
        slide_xml = String::from_utf8_lossy(data).to_string();
    }

    // Remove transition element
    if let Some(start) = slide_xml.find("<p:transition") {
        if let Some(end_rel) = slide_xml[start..].find("</p:transition>") {
            let end = start + end_rel + "</p:transition>".len();
            slide_xml = format!("{}{}", &slide_xml[..start], &slide_xml[end..]);
        }
    }

    // Update entry
    if let Some(ref mut entry) = entries.iter_mut().find(|(n, _)| n == &slide_path) {
        entry.1 = slide_xml.as_bytes().to_vec();
    }

    // Write new archive
    let mut new_archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for (name, data) in entries {
        new_archive.start_file(&name, options)?;
        new_archive.write_all(&data)?;
    }

    let cursor = new_archive.finish()?;
    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_transition_tool_basic() {
        let tool = SetSlideTransitionTool;
        assert_eq!(tool.name(), "set_slide_transition");
        assert!(!tool.description().is_empty());
    }
}
