//! Handout master part
//!
//! Represents the handout master (ppt/handoutMasters/handoutMaster1.xml).

use super::base::{Part, PartType, ContentType};
use crate::exc::PptxError;

/// Handout layout type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HandoutLayout {
    #[default]
    SlidesPerPage1,
    SlidesPerPage2,
    SlidesPerPage3,
    SlidesPerPage4,
    SlidesPerPage6,
    SlidesPerPage9,
    Outline,
}

impl HandoutLayout {
    pub fn slides_per_page(&self) -> u32 {
        match self {
            HandoutLayout::SlidesPerPage1 => 1,
            HandoutLayout::SlidesPerPage2 => 2,
            HandoutLayout::SlidesPerPage3 => 3,
            HandoutLayout::SlidesPerPage4 => 4,
            HandoutLayout::SlidesPerPage6 => 6,
            HandoutLayout::SlidesPerPage9 => 9,
            HandoutLayout::Outline => 0,
        }
    }
}

/// Handout master part
#[derive(Debug, Clone)]
pub struct HandoutMasterPart {
    path: String,
    layout: HandoutLayout,
    show_header: bool,
    show_footer: bool,
    show_date: bool,
    show_page_number: bool,
    header_text: Option<String>,
    footer_text: Option<String>,
    xml_content: Option<String>,
}

impl HandoutMasterPart {
    pub fn new() -> Self {
        HandoutMasterPart {
            path: "ppt/handoutMasters/handoutMaster1.xml".to_string(),
            layout: HandoutLayout::default(),
            show_header: true,
            show_footer: true,
            show_date: true,
            show_page_number: true,
            header_text: None,
            footer_text: None,
            xml_content: None,
        }
    }

    pub fn layout(mut self, layout: HandoutLayout) -> Self {
        self.layout = layout;
        self
    }

    pub fn header(mut self, text: impl Into<String>) -> Self {
        self.header_text = Some(text.into());
        self
    }

    pub fn footer(mut self, text: impl Into<String>) -> Self {
        self.footer_text = Some(text.into());
        self
    }

    pub fn hide_header(mut self) -> Self {
        self.show_header = false;
        self
    }

    pub fn hide_footer(mut self) -> Self {
        self.show_footer = false;
        self
    }

    pub fn hide_date(mut self) -> Self {
        self.show_date = false;
        self
    }

    pub fn hide_page_number(mut self) -> Self {
        self.show_page_number = false;
        self
    }

    fn generate_xml(&self) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:handoutMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
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
    </p:spTree>
  </p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  <p:hf hdr="{}" ftr="{}" dt="{}" sldNum="{}"/>
</p:handoutMaster>"#,
            if self.show_header { "1" } else { "0" },
            if self.show_footer { "1" } else { "0" },
            if self.show_date { "1" } else { "0" },
            if self.show_page_number { "1" } else { "0" }
        )
    }
}

impl Default for HandoutMasterPart {
    fn default() -> Self {
        Self::new()
    }
}

impl Part for HandoutMasterPart {
    fn path(&self) -> &str {
        &self.path
    }

    fn part_type(&self) -> PartType {
        PartType::SlideMaster // Similar handling
    }

    fn content_type(&self) -> ContentType {
        ContentType::Xml
    }

    fn to_xml(&self) -> Result<String, PptxError> {
        if let Some(ref xml) = self.xml_content {
            return Ok(xml.clone());
        }
        Ok(self.generate_xml())
    }

    fn from_xml(xml: &str) -> Result<Self, PptxError> {
        let mut part = HandoutMasterPart::new();
        part.xml_content = Some(xml.to_string());
        Ok(part)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handout_master_new() {
        let master = HandoutMasterPart::new();
        assert_eq!(master.path(), "ppt/handoutMasters/handoutMaster1.xml");
    }

    #[test]
    fn test_handout_layout() {
        assert_eq!(HandoutLayout::SlidesPerPage3.slides_per_page(), 3);
        assert_eq!(HandoutLayout::SlidesPerPage6.slides_per_page(), 6);
    }

    #[test]
    fn test_handout_master_builder() {
        let master = HandoutMasterPart::new()
            .layout(HandoutLayout::SlidesPerPage4)
            .header("My Presentation")
            .footer("Confidential")
            .hide_date();
        assert!(!master.show_date);
        assert!(master.show_header);
    }

    #[test]
    fn test_handout_master_to_xml() {
        let master = HandoutMasterPart::new();
        let xml = master.to_xml().unwrap();
        assert!(xml.contains("p:handoutMaster"));
        assert!(xml.contains("p:hf"));
    }
}
