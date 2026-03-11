use apchat_toolcore::tool_context::ToolContext;
use std::io::{Cursor, Read, Write};

/// Slide size/aspect ratio options for presentations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SlideSize {
    /// 16:9 widescreen (default, PowerPoint standard)
    Screen16x9,
    /// 4:3 standard (older format)
    Screen4x3,
    /// 16:10 widescreen (some laptops)
    Screen16x10,
    /// Custom dimensions in EMU (English Metric Units)
    Custom { width: i64, height: i64 },
}

impl SlideSize {
    /// Get slide dimensions in EMU (English Metric Units)
    /// 1 inch = 914,400 EMU
    pub fn dimensions(&self) -> (i64, i64, &'static str) {
        match self {
            SlideSize::Screen16x9 => (9144000, 5143500, "screen16x9"), // 10" × 5.625"
            SlideSize::Screen4x3 => (9144000, 6858000, "screen4x3"),   // 10" × 7.5"
            SlideSize::Screen16x10 => (9144000, 5715000, "screen16x10"), // 10" × 6.25"
            SlideSize::Custom { width, height } => (*width, *height, "custom"),
        }
    }

    /// Parse slide size from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "4x3" | "4:3" | "standard" => SlideSize::Screen4x3,
            "16x10" | "16:10" | "widescreen16x10" => SlideSize::Screen16x10,
            "16x9" | "16:9" | "widescreen" | "widescreen16x9" => SlideSize::Screen16x9,
            _ => SlideSize::Screen16x9, // Default to 16:9
        }
    }
}

/// Create a presentation from a template, preserving theme and layouts
pub fn create_presentation_from_template(
    _title: &str,
    _author: &str,
    slides: &[ppt_rs::generator::SlideContent],
    is_title_slide: &[bool],
    template_path: &str,
    slide_size: SlideSize,
    context: &ToolContext,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // Read template
    let template_full_path = context.work_dir.join(template_path);
    if !template_full_path.exists() {
        return Err(format!("Template file not found: {}", template_path).into());
    }
    let template_data = std::fs::read(&template_full_path)?;
    let mut template_archive = zip::ZipArchive::new(Cursor::new(&template_data))?;

    // Create new archive
    let mut new_archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // Copy all template files except slides and [Content_Types].xml
    for i in 0..template_archive.len() {
        let mut entry = template_archive.by_index(i)?;
        let entry_name = entry.name().to_string();

        // Skip template slide XML files (we'll create new ones)
        if entry_name.starts_with("ppt/slides/slide") && entry_name.ends_with(".xml") {
            continue;
        }
        // Skip slide relationships
        if entry_name.starts_with("ppt/slides/_rels/slide") && entry_name.ends_with(".xml.rels") {
            continue;
        }
        // Skip [Content_Types].xml - we'll regenerate it
        if entry_name == "[Content_Types].xml" {
            continue;
        }
        // Skip presentation.xml and its relationships - we'll regenerate them
        if entry_name == "ppt/presentation.xml" || entry_name == "ppt/_rels/presentation.xml.rels" {
            continue;
        }

        new_archive.start_file(&entry_name, options.clone())?;
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;
        new_archive.write_all(&buf)?;
    }

    // Find appropriate slide layouts from template by reading slideLayouts XML
    let (title_layout_id, content_layout_id) = find_layout_ids(&mut template_archive)?;

    // Generate new presentation.xml with correct slide references
    let (width, height, size_type) = slide_size.dimensions();
    let mut pres_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:sldMasterIdLst>
<p:sldMasterId id="2147483648" r:id="rId1"/>
</p:sldMasterIdLst>
<p:sldIdLst>"#
    );

    for (idx, _) in slides.iter().enumerate() {
        pres_content.push_str(&format!(
            r#"<p:sldId id="{}" r:id="rId{}"/>"#,
            256 + idx,
            2 + idx
        ));
    }

    pres_content.push_str(
        &format!(r#"</p:sldIdLst><p:sldSz cx="{width}" cy="{height}" type="{size_type}"/><p:notesSz cx="6858000" cy="9144000"/></p:presentation>"#),
    );

    new_archive.start_file("ppt/presentation.xml", options.clone())?;
    new_archive.write_all(pres_content.as_bytes())?;

    // Update presentation relationships
    let mut rels_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
"#
    );

    for (idx, _) in slides.iter().enumerate() {
        rels_content.push_str(&format!(
            r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{}.xml"/>"#,
            2 + idx, idx + 1
        ));
        rels_content.push('\n');
    }

    rels_content.push_str("</Relationships>");

    new_archive.start_file("ppt/_rels/presentation.xml.rels", options.clone())?;
    new_archive.write_all(rels_content.as_bytes())?;

    // Create new slides using template layouts
    for (idx, slide) in slides.iter().enumerate() {
        let slide_num = idx + 1;
        // Determine layout: use title layout for title slides, content layout otherwise
        let layout_id = if is_title_slide.get(idx).copied().unwrap_or(false) {
            title_layout_id
        } else {
            content_layout_id
        };

        // Extract title and bullets from slide
        let title_text = &slide.title;
        let bullets: Vec<&str> = slide.bullets.iter().map(|s| s.text.as_str()).collect();

        // Build slide XML
        let slide_xml = build_slide_xml(idx, title_text, &bullets, layout_id, title_layout_id);

        new_archive.start_file(
            &format!("ppt/slides/slide{}.xml", slide_num),
            options.clone(),
        )?;
        new_archive.write_all(slide_xml.as_bytes())?;

        // Create slide relationships (referencing the layout)
        let slide_rels = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout{}.xml"/>
</Relationships>
"#,
            layout_id
        );

        new_archive.start_file(
            &format!("ppt/slides/_rels/slide{}.xml.rels", slide_num),
            options.clone(),
        )?;
        new_archive.write_all(slide_rels.as_bytes())?;
    }

    // Regenerate [Content_Types] to only include our new slides
    if let Ok(mut ct_entry) = template_archive.by_name("[Content_Types].xml") {
        let mut ct_content = String::new();
        ct_entry.read_to_string(&mut ct_content)?;

        // Remove all old slide references (slide1.xml through slideN.xml)
        use regex::Regex;
        let slide_re =
            Regex::new(r#"<Override PartName="/ppt/slides/slide\d+\.xml"[^>]*/>"#).unwrap();
        ct_content = slide_re.replace_all(&ct_content, "").to_string();

        // Add override entries for our new slides
        for (idx, _) in slides.iter().enumerate() {
            let slide_num = idx + 1;
            let slide_entry = format!(
                r#"<Override PartName="/ppt/slides/slide{}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#,
                slide_num
            );
            // Insert before closing tag
            ct_content = ct_content.replace("</Types>", &format!("{}\n</Types>", slide_entry));
        }

        new_archive.start_file("[Content_Types].xml", options.clone())?;
        new_archive.write_all(ct_content.as_bytes())?;
    }

    let cursor = new_archive.finish()?;
    Ok(cursor.into_inner())
}

/// Build slide XML content
fn build_slide_xml(
    idx: usize,
    title_text: &str,
    bullets: &[&str],
    layout_id: i32,
    title_layout_id: i32,
) -> String {
    let mut slide_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld>
<p:sldId id="{}"/>
<p:spTree>
<p:nvGrpSpPr>
<p:cNvPr id="1" name=""/>
<p:cNvGrpSpPr/>
<p:nvPr/>
</p:nvGrpSpPr>
<p:grpSpPr>
<a:xfrm>
<a:off x="0" y="0"/>
<a:ext cx="0" cy="0"/>
<a:chOff x="0" y="0"/>
<a:chExt cx="0" cy="0"/>
</a:xfrm>
</p:grpSpPr>
"#,
        256 + idx
    );

    // Add title placeholder
    let ph_type = if layout_id == title_layout_id {
        "ctrTitle"
    } else {
        "title"
    };
    slide_content.push_str(&format!(
        r#"<p:sp>
<p:nvSpPr>
<p:cNvPr id="2" name="Title"/>
<p:cNvSpPr txBox="1"/>
<p:nvPr><p:ph type="{}"/></p:nvPr>
</p:nvSpPr>
<p:spPr/>
<p:txBody>
<a:bodyPr/>
<a:lstStyle/>
<a:p>
<a:pPr algn="ctr"/>
<a:r>
<a:rPr lang="en-US" sz="4000" b="1"/>
<a:t>{}</a:t>
</a:r>
</a:p>
</p:txBody>
</p:sp>
"#,
        ph_type, title_text
    ));

    // Add subtitle or bullets
    if layout_id == title_layout_id && !bullets.is_empty() {
        // Add subtitle for title slides
        slide_content.push_str(&format!(
            r#"<p:sp>
<p:nvSpPr>
<p:cNvPr id="3" name="Subtitle"/>
<p:cNvSpPr txBox="1"/>
<p:nvPr><p:ph type="subTitle"/></p:nvPr>
</p:nvSpPr>
<p:spPr/>
<p:txBody>
<a:bodyPr/>
<a:lstStyle/>
<a:p>
<a:pPr algn="ctr"/>
<a:r>
<a:rPr lang="en-US" sz="1800"/>
<a:t>{}</a:t>
</a:r>
</a:p>
</p:txBody>
</p:sp>
"#,
            bullets[0]
        ));
    } else if !bullets.is_empty() {
        // Add content placeholder with bullets
        slide_content.push_str(
            r#"<p:sp>
<p:nvSpPr>
<p:cNvPr id="3" name="Content"/>
<p:cNvSpPr txBox="1"/>
<p:nvPr><p:ph type="body"/></p:nvPr>
</p:nvSpPr>
<p:spPr/>
<p:txBody>
<a:bodyPr lIns="914400" tIns="914400" rIns="914400" bIns="914400"/>
<a:lstStyle/>
"#,
        );

        for bullet in bullets {
            slide_content.push_str(&format!(
                r#"<a:p>
<a:pPr lvl="0"/>
<a:r>
<a:rPr lang="en-US" sz="1800"/>
<a:t>{}</a:t>
</a:r>
</a:p>
"#,
                bullet
            ));
        }

        slide_content.push_str(
            r#"</p:txBody>
</p:sp>
"#,
        );
    }

    slide_content.push_str(
        r#"</p:spTree>
</p:cSld>
<p:clrMapOvr>
<a:masterClrMapping/>
</p:clrMapOvr>
</p:sld>
"#,
    );

    slide_content
}

/// Discover slide layout IDs from template by examining slideLayout files
fn find_layout_ids(
    template_archive: &mut zip::ZipArchive<Cursor<&Vec<u8>>>,
) -> Result<(i32, i32), Box<dyn std::error::Error + Send + Sync>> {
    let mut title_layout_id = 2; // Default fallback
    let mut content_layout_id = 1; // Default fallback

    // Read presentation.xml to get layout references
    let mut pres_layout_ids = Vec::new();
    if let Ok(mut entry) = template_archive.by_name("ppt/presentation.xml") {
        let mut xml = String::new();
        entry.read_to_string(&mut xml)?;

        // Extract layout IDs from presentation.xml relationships
        let layout_re =
            regex::Regex::new(r#"<p:sldLayoutIdLst[^>]*>(.*?)</p:sldLayoutIdLst>"#).unwrap();
        if let Some(caps) = layout_re.captures(&xml) {
            if let Some(layouts) = caps.get(1) {
                let id_re =
                    regex::Regex::new(r#"<p:sldLayoutId id="(\d+)"[^>]*type="([^"]*)""#).unwrap();
                for cap in id_re.captures_iter(layouts.as_str()) {
                    if let (Some(id_match), Some(type_match)) = (cap.get(1), cap.get(2)) {
                        if let Ok(id) = id_match.as_str().parse::<i32>() {
                            let layout_type = type_match.as_str();
                            pres_layout_ids.push((id, layout_type.to_string()));
                        }
                    }
                }
            }
        }
    }

    // Find title and content layout IDs from discovered layouts
    for (id, layout_type) in &pres_layout_ids {
        if layout_type == "title" || layout_type == "ctrTitle" {
            title_layout_id = *id;
        } else if layout_type == "body" || layout_type == "obj" {
            content_layout_id = *id;
        }
    }

    // If not found in presentation.xml, try reading individual layout files
    if title_layout_id == 2 && content_layout_id == 1 {
        // First, collect all layout file names and numbers
        let mut layout_files: Vec<(String, i32)> = Vec::new();
        for i in 0..template_archive.len() {
            let entry = template_archive.by_index(i)?;
            let entry_name = entry.name().to_string();

            if entry_name.starts_with("ppt/slideLayouts/slideLayout")
                && entry_name.ends_with(".xml")
            {
                // Extract layout number from filename
                if let Some(num_str) = entry_name
                    .strip_prefix("ppt/slideLayouts/slideLayout")
                    .and_then(|s| s.strip_suffix(".xml"))
                {
                    if let Ok(num) = num_str.parse::<i32>() {
                        layout_files.push((entry_name, num));
                    }
                }
            }
        }

        // Now read each layout file separately
        for (entry_name, num) in layout_files {
            let mut layout_xml = String::new();
            let mut layout_entry = template_archive.by_name(&entry_name)?;
            layout_entry.read_to_string(&mut layout_xml)?;

            // Check for title placeholder
            if layout_xml.contains(r#"type="title""#) || layout_xml.contains(r#"type="ctrTitle""#) {
                title_layout_id = num;
            } else if layout_xml.contains(r#"type="body""#) {
                content_layout_id = num;
            }
        }
    }

    Ok((title_layout_id, content_layout_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slide_size_dimensions() {
        // Test 16:9
        let (w, h, t) = SlideSize::Screen16x9.dimensions();
        assert_eq!(w, 9144000);
        assert_eq!(h, 5143500);
        assert_eq!(t, "screen16x9");

        // Test 4:3
        let (w, h, t) = SlideSize::Screen4x3.dimensions();
        assert_eq!(w, 9144000);
        assert_eq!(h, 6858000);
        assert_eq!(t, "screen4x3");

        // Test 16:10
        let (w, h, t) = SlideSize::Screen16x10.dimensions();
        assert_eq!(w, 9144000);
        assert_eq!(h, 5715000);
        assert_eq!(t, "screen16x10");
    }

    #[test]
    fn test_slide_size_from_str() {
        assert_eq!(SlideSize::from_str("16x9"), SlideSize::Screen16x9);
        assert_eq!(SlideSize::from_str("16:9"), SlideSize::Screen16x9);
        assert_eq!(SlideSize::from_str("widescreen"), SlideSize::Screen16x9);
        assert_eq!(SlideSize::from_str("4x3"), SlideSize::Screen4x3);
        assert_eq!(SlideSize::from_str("4:3"), SlideSize::Screen4x3);
        assert_eq!(SlideSize::from_str("standard"), SlideSize::Screen4x3);
        assert_eq!(SlideSize::from_str("16x10"), SlideSize::Screen16x10);
        assert_eq!(SlideSize::from_str("unknown"), SlideSize::Screen16x9); // Default
    }
}
