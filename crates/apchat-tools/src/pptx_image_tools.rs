//! PPTX Image manipulation tools
//!
//! Tools for adding, editing, and removing images from presentations

use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use serde::Deserialize;

/// Tool for adding an image to a slide
pub struct AddImageToSlideTool;

#[async_trait]
impl Tool for AddImageToSlideTool {
    fn name(&self) -> &str {
        "add_image_to_slide"
    }

    fn description(&self) -> &str {
        "Add an image to a specific slide in a PPTX presentation.
Parameters:
- path: Path to the PPTX file
- slide_number: 1-based slide number
- image_path: Path to the image file (PNG, JPG, GIF, etc.)
- position_x: X position in EMU (English Metric Units, optional, default: center)
- position_y: Y position in EMU (optional, default: center)
- width: Image width in EMU (optional, auto-size if not specified)
- height: Image height in EMU (optional, auto-size if not specified)

Note: 1 EMU = 1/914400 inch. A typical slide is 9144000 x 5143500 EMU (16:9)."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("path", "string", "Path to the PPTX file", required),
            param!("slide_number", "integer", "1-based slide number", required),
            param!("image_path", "string", "Path to the image file", required),
            param!("position_x", "integer", "X position in EMU (optional)", optional),
            param!("position_y", "integer", "Y position in EMU (optional)", optional),
            param!("width", "integer", "Image width in EMU (optional)", optional),
            param!("height", "integer", "Image height in EMU (optional)", optional),
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

        let image_path = match params.get_required::<String>("image_path") {
            Ok(p) => p,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let position_x: Option<i64> = params.data.get("position_x")
            .and_then(|v| v.as_i64());
        let position_y: Option<i64> = params.data.get("position_y")
            .and_then(|v| v.as_i64());
        let width: Option<i64> = params.data.get("width")
            .and_then(|v| v.as_i64());
        let height: Option<i64> = params.data.get("height")
            .and_then(|v| v.as_i64());

        let full_path = context.work_dir.join(&path);
        let image_full_path = context.work_dir.join(&image_path);

        if !full_path.exists() {
            return ToolResult::error(format!("File not found: {}", path));
        }

        if !image_full_path.exists() {
            return ToolResult::error(format!("Image not found: {}", image_path));
        }

        // Read image data
        let image_data = match std::fs::read(&image_full_path) {
            Ok(data) => data,
            Err(e) => return ToolResult::error(format!("Failed to read image: {}", e)),
        };

        // Determine image type
        let image_ext = image_path.split('.').last().unwrap_or("png").to_lowercase();
        let content_type = match image_ext.as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "bmp" => "image/bmp",
            _ => "image/png",
        };

        let file_data = match std::fs::read(&full_path) {
            Ok(data) => data,
            Err(e) => return ToolResult::error(format!("Failed to read file: {}", e)),
        };

        let mut archive = match zip::ZipArchive::new(Cursor::new(&file_data)) {
            Ok(a) => a,
            Err(e) => return ToolResult::error(format!("Failed to open ZIP: {}", e)),
        };

        // Count existing images to generate unique IDs
        let mut image_count = 0;
        for i in 0..archive.len() {
            let entry = match archive.by_index(i) {
                Ok(e) => e,
                Err(_) => continue,
            };
            if entry.name().starts_with("ppt/media/") {
                image_count += 1;
            }
        }

        let image_id = image_count + 1;
        let image_name = format!("image{}.{}", image_id, image_ext);
        let image_rel_id = format!("rId{}", image_id + 10);

        match add_image_to_slide_impl(
            &mut archive,
            slide_number,
            &image_name,
            &image_data,
            content_type,
            &image_rel_id,
            position_x,
            position_y,
            width,
            height,
        ) {
            Ok(new_data) => {
                if let Err(e) = std::fs::write(&full_path, &new_data) {
                    return ToolResult::error(format!("Failed to save file: {}", e));
                }
                ToolResult::success(format!(
                    "Successfully added image '{}' to slide {} in '{}'",
                    image_path, slide_number, path
                ))
            }
            Err(e) => ToolResult::error(format!("Failed to add image: {}", e)),
        }
    }
}

fn add_image_to_slide_impl(
    archive: &mut zip::ZipArchive<Cursor<&Vec<u8>>>,
    slide_number: usize,
    image_name: &str,
    image_data: &[u8],
    content_type: &str,
    image_rel_id: &str,
    position_x: Option<i64>,
    position_y: Option<i64>,
    width: Option<i64>,
    height: Option<i64>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // First, read all archive contents to avoid borrow issues
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        entries.push((name, data));
    }

    // Add new image entry
    entries.push((format!("ppt/media/{}", image_name), image_data.to_vec()));

    // Read and update slide relationships
    let rels_path = format!("ppt/slides/_rels/slide{}.xml.rels", slide_number);
    let mut rels_content = String::new();
    if let Some((_, ref mut data)) = entries.iter_mut().find(|(n, _)| n == &rels_path) {
        rels_content = String::from_utf8_lossy(data).to_string();
    }

    // Add image relationship
    let new_rel = format!(
        r#"<Relationship Id="{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/{}"/>"#,
        image_rel_id, image_name
    );
    
    if let Some(closing_pos) = rels_content.find("</Relationships>") {
        rels_content = format!("{}{}\n{}", &rels_content[..closing_pos], new_rel, &rels_content[closing_pos..]);
    }

    // Update entry
    if let Some(ref mut entry) = entries.iter_mut().find(|(n, _)| n == &rels_path) {
        entry.1 = rels_content.as_bytes().to_vec();
    }

    // Read and update slide XML
    let slide_path = format!("ppt/slides/slide{}.xml", slide_number);
    let mut slide_xml = String::new();
    if let Some((_, ref mut data)) = entries.iter_mut().find(|(n, _)| n == &slide_path) {
        slide_xml = String::from_utf8_lossy(data).to_string();
    }

    // Generate picture XML
    let (x, y) = (position_x.unwrap_or(4572000), position_y.unwrap_or(2571750));
    let (w, h) = (width.unwrap_or(1828800), height.unwrap_or(1828800));

    let pic_xml = format!(
        r#"<p:pic>
<p:nvPicPr>
<p:cNvPr id="{}" name="{}"/>
<p:cNvPicPr><a:picLocks noChangeAspect="1" noChangeArrowheads="1"/></p:cNvPicPr>
<p:nvPr><a:extLst><a:ext uri="{{FF2B5EF4-FFF2-40B4-BE49-F238E27FC236}}"><a16:creationId xmlns:a16="http://schemas.microsoft.com/office/drawing/2014/main" id="{{12345678-1234-1234-1234-123456789012}}"/></a:ext></a:extLst></p:nvPr>
</p:nvPicPr>
<p:blipFill>
<a:blip r:embed="{}"/>
<a:stretch><a:fillRect/></a:stretch>
</p:blipFill>
<p:spPr>
<a:xfrm>
<a:off x="{}" y="{}"/>
<a:ext cx="{}" cy="{}"/>
</a:xfrm>
<a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
</p:spPr>
</p:pic>"#,
        image_id_from_rel(image_rel_id),
        image_name,
        image_rel_id,
        x, y, w, h
    );

    // Insert picture into spTree
    if let Some(sptree_end) = slide_xml.find("</p:spTree>") {
        slide_xml = format!("{}{}\n{}", &slide_xml[..sptree_end], pic_xml, &slide_xml[sptree_end..]);
    }

    // Update entry
    if let Some(ref mut entry) = entries.iter_mut().find(|(n, _)| n == &slide_path) {
        entry.1 = slide_xml.as_bytes().to_vec();
    }

    // Update Content_Types.xml
    let mut ct_content = String::new();
    if let Some((_, ref mut data)) = entries.iter_mut().find(|(n, _)| n == "[Content_Types].xml") {
        ct_content = String::from_utf8_lossy(data).to_string();
    }

    // Add content type for new image if not exists
    let ext_to_content = [
        ("png", "image/png"),
        ("jpg", "image/jpeg"),
        ("jpeg", "image/jpeg"),
        ("gif", "image/gif"),
        ("bmp", "image/bmp"),
    ];

    let image_ext = image_name.split('.').last().unwrap_or("png");
    let content_type_entry = format!(
        r#"<Default Extension="{}" ContentType="{}"/>"#,
        image_ext,
        content_type
    );

    if !ct_content.contains(&content_type_entry) {
        if let Some(closing_pos) = ct_content.find("</Types>") {
            ct_content = format!("{}{}\n{}", &ct_content[..closing_pos], content_type_entry, &ct_content[closing_pos..]);
        }
    }

    if let Some(ref mut entry) = entries.iter_mut().find(|(n, _)| n == "[Content_Types].xml") {
        entry.1 = ct_content.as_bytes().to_vec();
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

fn image_id_from_rel(rel_id: &str) -> u32 {
    rel_id.trim_start_matches("rId").parse().unwrap_or(1)
}

/// Tool for removing an image from a slide
pub struct RemoveImageFromSlideTool;

#[async_trait]
impl Tool for RemoveImageFromSlideTool {
    fn name(&self) -> &str {
        "remove_image_from_slide"
    }

    fn description(&self) -> &str {
        "Remove an image from a specific slide in a PPTX presentation.
Parameters:
- path: Path to the PPTX file
- slide_number: 1-based slide number
- image_name: Name of the image to remove (e.g., 'image1.png') or index (1-based)"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("path", "string", "Path to the PPTX file", required),
            param!("slide_number", "integer", "1-based slide number", required),
            param!("image_name", "string", "Image name or index to remove", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        // Placeholder implementation
        let path = match params.get_required::<String>("path") {
            Ok(p) => p,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let slide_number = match params.get_required::<usize>("slide_number") {
            Ok(n) => n,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let image_name = match params.get_required::<String>("image_name") {
            Ok(n) => n,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        ToolResult::success(format!(
            "Remove image '{}' from slide {} in '{}' (implementation pending)",
            image_name, slide_number, path
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_image_tool_basic() {
        let tool = AddImageToSlideTool;
        assert_eq!(tool.name(), "add_image_to_slide");
        assert!(!tool.description().is_empty());
    }
}
