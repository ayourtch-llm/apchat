//! Embedded font part
//!
//! Represents fonts embedded in the presentation for consistent rendering.

use super::base::{Part, PartType, ContentType};
use crate::exc::PptxError;

/// Font embedding type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontEmbedType {
    #[default]
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

impl FontEmbedType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FontEmbedType::Regular => "regular",
            FontEmbedType::Bold => "bold",
            FontEmbedType::Italic => "italic",
            FontEmbedType::BoldItalic => "boldItalic",
        }
    }
}

/// Embedded font part (ppt/fonts/fontN.fntdata)
#[derive(Debug, Clone)]
pub struct EmbeddedFontPart {
    path: String,
    font_number: usize,
    font_name: String,
    embed_type: FontEmbedType,
    data: Vec<u8>,
    charset: Option<String>,
    pitch_family: Option<u8>,
}

impl EmbeddedFontPart {
    /// Create a new embedded font part
    pub fn new(font_number: usize, font_name: impl Into<String>, data: Vec<u8>) -> Self {
        EmbeddedFontPart {
            path: format!("ppt/fonts/font{}.fntdata", font_number),
            font_number,
            font_name: font_name.into(),
            embed_type: FontEmbedType::default(),
            data,
            charset: None,
            pitch_family: None,
        }
    }

    /// Set embed type
    pub fn embed_type(mut self, embed_type: FontEmbedType) -> Self {
        self.embed_type = embed_type;
        self
    }

    /// Set charset
    pub fn charset(mut self, charset: impl Into<String>) -> Self {
        self.charset = Some(charset.into());
        self
    }

    /// Set pitch family
    pub fn pitch_family(mut self, pitch_family: u8) -> Self {
        self.pitch_family = Some(pitch_family);
        self
    }

    /// Get font number
    pub fn font_number(&self) -> usize {
        self.font_number
    }

    /// Get font name
    pub fn font_name(&self) -> &str {
        &self.font_name
    }

    /// Get font data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get embed type
    pub fn get_embed_type(&self) -> FontEmbedType {
        self.embed_type
    }

    /// Generate font reference XML for presentation.xml
    pub fn to_font_ref_xml(&self) -> String {
        let charset_attr = self.charset.as_ref()
            .map(|c| format!(r#" charset="{}""#, c))
            .unwrap_or_default();
        let pitch_attr = self.pitch_family
            .map(|p| format!(r#" pitchFamily="{}""#, p))
            .unwrap_or_default();

        format!(
            r#"<p:embeddedFont>
  <p:font typeface="{}"{}{}>
    <p:{}/>
  </p:font>
  <p:{}><a:extLst><a:ext uri="{{28A0092B-C50C-407E-A947-70E740481C1C}}"><a14:useLocalDpi xmlns:a14="http://schemas.microsoft.com/office/drawing/2010/main" val="0"/></a:ext></a:extLst></p:{}>
</p:embeddedFont>"#,
            self.font_name,
            charset_attr,
            pitch_attr,
            self.embed_type.as_str(),
            self.embed_type.as_str(),
            self.embed_type.as_str()
        )
    }
}

impl Part for EmbeddedFontPart {
    fn path(&self) -> &str {
        &self.path
    }

    fn part_type(&self) -> PartType {
        PartType::Image // Fonts are handled similarly to images (binary)
    }

    fn content_type(&self) -> ContentType {
        ContentType::Xml // Actually binary font data
    }

    fn to_xml(&self) -> Result<String, PptxError> {
        // Fonts are binary, not XML
        Err(PptxError::InvalidOperation("Embedded fonts are binary, not XML".to_string()))
    }

    fn from_xml(_xml: &str) -> Result<Self, PptxError> {
        Err(PptxError::InvalidOperation("Embedded fonts cannot be created from XML".to_string()))
    }
}

/// Font collection for managing embedded fonts
#[derive(Debug, Clone, Default)]
pub struct EmbeddedFontCollection {
    fonts: Vec<EmbeddedFontPart>,
}

impl EmbeddedFontCollection {
    pub fn new() -> Self {
        EmbeddedFontCollection::default()
    }

    /// Add a font
    pub fn add(&mut self, font_name: impl Into<String>, data: Vec<u8>) -> &mut EmbeddedFontPart {
        let font_number = self.fonts.len() + 1;
        self.fonts.push(EmbeddedFontPart::new(font_number, font_name, data));
        self.fonts.last_mut().unwrap()
    }

    /// Add a font with specific embed type
    pub fn add_with_type(&mut self, font_name: impl Into<String>, data: Vec<u8>, embed_type: FontEmbedType) -> &mut EmbeddedFontPart {
        let font_number = self.fonts.len() + 1;
        let mut font = EmbeddedFontPart::new(font_number, font_name, data);
        font.embed_type = embed_type;
        self.fonts.push(font);
        self.fonts.last_mut().unwrap()
    }

    /// Get all fonts
    pub fn fonts(&self) -> &[EmbeddedFontPart] {
        &self.fonts
    }

    /// Get font count
    pub fn len(&self) -> usize {
        self.fonts.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
    }

    /// Generate embedded fonts XML for presentation.xml
    pub fn to_xml(&self) -> String {
        if self.fonts.is_empty() {
            return String::new();
        }

        let fonts_xml: String = self.fonts.iter()
            .map(|f| f.to_font_ref_xml())
            .collect::<Vec<_>>()
            .join("\n");

        format!("<p:embeddedFontLst>\n{}\n</p:embeddedFontLst>", fonts_xml)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_font_new() {
        let font = EmbeddedFontPart::new(1, "Arial", vec![0, 1, 2]);
        assert_eq!(font.font_number(), 1);
        assert_eq!(font.font_name(), "Arial");
        assert_eq!(font.path(), "ppt/fonts/font1.fntdata");
    }

    #[test]
    fn test_embedded_font_builder() {
        let font = EmbeddedFontPart::new(1, "Times New Roman", vec![])
            .embed_type(FontEmbedType::Bold)
            .charset("00")
            .pitch_family(18);
        assert_eq!(font.get_embed_type(), FontEmbedType::Bold);
    }

    #[test]
    fn test_font_embed_type() {
        assert_eq!(FontEmbedType::Regular.as_str(), "regular");
        assert_eq!(FontEmbedType::BoldItalic.as_str(), "boldItalic");
    }

    #[test]
    fn test_font_collection() {
        let mut collection = EmbeddedFontCollection::new();
        collection.add("Arial", vec![0, 1, 2]);
        collection.add("Times New Roman", vec![3, 4, 5]);
        assert_eq!(collection.len(), 2);
    }

    #[test]
    fn test_font_collection_to_xml() {
        let mut collection = EmbeddedFontCollection::new();
        collection.add("Arial", vec![]);
        let xml = collection.to_xml();
        assert!(xml.contains("p:embeddedFontLst"));
        assert!(xml.contains("Arial"));
    }
}
