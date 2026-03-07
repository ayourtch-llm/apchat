use apchat_toolcore::tool_context::ToolContext;
use std::io::{Cursor, Read, Write};

/// Create a presentation from a template, preserving theme and layouts
pub fn create_presentation_from_template(
    _title: &str,
    _author: &str,
    slides: &[ppt_rs::generator::SlideContent],
    is_title_slide: &[bool],
    template_path: &str,
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
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

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

    // Find appropriate slide layouts from template
    // Typically: layout 1 = content slide, layout 2 = title slide (based on Cisco template)
    let title_layout_id = 2;
    let content_layout_id = 1;

    // Generate new presentation.xml with correct slide references
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
        r#"</p:sldIdLst>
<p:sldSz cx="9144000" cy="5143500" type="screen16x9"/>
<p:notesSz cx="6858000" cy="9144000"/>
</p:presentation>"#
    );
    
    new_archive.start_file("ppt/presentation.xml", options.clone())?;
    new_archive.write_all(pres_content.as_bytes())?;

    // Update presentation relationships
    let mut rels_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
"#);
    
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
        
        new_archive.start_file(&format!("ppt/slides/slide{}.xml", slide_num), options.clone())?;
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
        
        new_archive.start_file(&format!("ppt/slides/_rels/slide{}.xml.rels", slide_num), options.clone())?;
        new_archive.write_all(slide_rels.as_bytes())?;
    }

    // Regenerate [Content_Types] to only include our new slides
    if let Ok(mut ct_entry) = template_archive.by_name("[Content_Types].xml") {
        let mut ct_content = String::new();
        ct_entry.read_to_string(&mut ct_content)?;
        
        // Remove all old slide references (slide1.xml through slideN.xml)
        use regex::Regex;
        let slide_re = Regex::new(r#"<Override PartName="/ppt/slides/slide\d+\.xml"[^>]*/>"#).unwrap();
        ct_content = slide_re.replace_all(&ct_content, "").to_string();
        
        // Add override entries for our new slides
        for (idx, _) in slides.iter().enumerate() {
            let slide_num = idx + 1;
            let slide_entry = format!(r#"<Override PartName="/ppt/slides/slide{}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#, slide_num);
            // Insert before closing tag
            ct_content = ct_content.replace(
                "</Types>",
                &format!("{}\n</Types>", slide_entry)
            );
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
    let ph_type = if layout_id == title_layout_id { "ctrTitle" } else { "title" };
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
        ph_type,
        title_text
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
"#
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
"#
        );
    }
    
    slide_content.push_str(
        r#"</p:spTree>
</p:cSld>
<p:clrMapOvr>
<a:masterClrMapping/>
</p:clrMapOvr>
</p:sld>
"#
    );
    
    slide_content
}