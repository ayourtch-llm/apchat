pub fn generate_presentation_xml(slide_count: usize) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n");
    xml.push_str("<p:presentation xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">\n");
    xml.push_str("  <p:slideIdLst>\n");
    
    for i in 0..slide_count {
        xml.push_str(&format!("    <p:slideId id=\"{:#016x}\"><p:nvSlideIdPr><p:id>{}</p:id></p:nvSlideIdPr></p:slideId>\n", i + 1, i + 1));
    }
    
    xml.push_str("  </p:slideIdLst>\n");
    xml.push_str("  <p:slides>\n");
    
    for i in 0..slide_count {
        xml.push_str(&format!("    <p:slide r:id=\"rId{}\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"/>\n", i + 1));
    }
    
    xml.push_str("  </p:slides>\n");
    xml.push_str("  <p:slideLayouts/>\n");
    xml.push_str("  <p:slideMasters/>\n");
    xml.push_str("</p:presentation>\n");
    
    xml
}

pub fn generate_slide_xml(slide_number: usize, title: Option<&str>, content: &[String], slide_type: &str) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n");
    xml.push_str("<p:slide xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\n");
    xml.push_str("  <p:spTree>\n");
    xml.push_str("    <p:mv/>\n");
    xml.push_str("    <p:extLst><p:ext uri=\"{28A0092F-C50C-407E-A92D-4BBE6D82779E}\"><p16:unlockedPlaceholderShapes xmlns:p16=\"http://schemas.microsoft.com/office/powerpoint/2010/main\"/></p:ext></p:extLst>\n");
    
    // Title placeholder
    if let Some(t) = title {
        xml.push_str(&format!("    <p:ph type=\"title\"><p:sp><p:spPr><a:xfrm><a:off x=\"914400\" y=\"1778040\"/><a:ext cx=\"15240000\" cy=\"3810000\"/></a:xfrm></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr/><a:t>{}</a:t></a:r></a:p></p:txBody></p:sp></p:ph>\n", t));
    }
    
    // Content placeholder
    if slide_type == "content" {
        xml.push_str("    <p:ph type=\"body\"><p:sp><p:spPr><a:xfrm><a:off x=\"914400\" y=\"476250\"/><a:ext cx=\"15240000\" cy=\"8890500\"/></a:xfrm></p:spPr><p:txBody><a:bodyPr/><a:lstStyle>\n");
        for item in content {
            xml.push_str(&format!("        <a:p><a:pPr><a:defRPr/></a:pPr><a:r><a:rPr/><a:t>{}</a:t></a:r></a:p>\n", item));
        }
        xml.push_str("      </a:lstStyle></p:txBody></p:sp></p:ph>\n");
    }
    
    xml.push_str("  </p:spTree>\n");
    xml.push_str("</p:slide>\n");
    
    xml
}