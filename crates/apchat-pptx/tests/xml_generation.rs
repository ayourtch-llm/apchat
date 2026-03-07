use apchat_pptx::xml::generate_presentation_xml;

#[test]
fn test_presentation_xml() {
    let xml = generate_presentation_xml(5);
    assert!(xml.contains("<p:presentation"));
    assert!(xml.contains("slideIdLst"));
}