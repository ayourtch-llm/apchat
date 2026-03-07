mod error;
pub mod xml;

pub use error::{PptxError, Result};

use std::fs::File;
use std::io::{Cursor, Write};
use zip::write::FileOptions;
use zip::ZipWriter;

pub struct Presentation {
    slides: Vec<Slide>,
    title: Option<String>,
    author: Option<String>,
    theme: Theme,
}

impl Presentation {
    pub fn new() -> Self {
        Self {
            slides: Vec::new(),
            title: None,
            author: None,
            theme: Theme::default(),
        }
    }
    
    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }
    
    pub fn author(mut self, author: &str) -> Self {
        self.author = Some(author.to_string());
        self
    }
    
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
    
    pub fn get_title(&self) -> Option<&str> {
        self.title.as_deref()
    }
    
    pub fn get_author(&self) -> Option<&str> {
        self.author.as_deref()
    }
    
    pub fn slides_count(&self) -> usize {
        self.slides.len()
    }
    
    pub fn add_title_slide(&mut self, title: &str, subtitle: &str) {
        self.slides.push(Slide {
            slide_type: SlideType::Title,
            title: Some(title.to_string()),
            content: vec![subtitle.to_string()],
        });
    }
    
    pub fn add_content_slide(&mut self, title: &str, bullets: Vec<&str>) {
        self.slides.push(Slide {
            slide_type: SlideType::Content,
            title: Some(title.to_string()),
            content: bullets.into_iter().map(|s| s.to_string()).collect(),
        });
    }
    
    pub fn save(&self, path: &str) -> Result<()> {
        let mut file = File::create(path)?;
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        
        let opts = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        
        // Add required PPTX files
        self.add_presentation_xml(&mut zip, opts)?;
        self.add_slides_xml(&mut zip, opts)?;
        self.add_content_types(&mut zip, opts)?;
        self.add_core_properties(&mut zip, opts)?;
        self.add_app_properties(&mut zip, opts)?;
        
        let cursor = zip.finish()?;
        let data = cursor.into_inner();
        std::io::Write::write_all(&mut file, &data)?;
        
        Ok(())
    }
    
    fn add_presentation_xml(&self, zip: &mut ZipWriter<Cursor<Vec<u8>>>, opts: FileOptions) -> Result<()> {
        let xml = xml::generate_presentation_xml(self.slides.len());
        zip.start_file("ppt/presentation.xml", opts)?;
        zip.write_all(xml.as_bytes())?;
        Ok(())
    }
    
    fn add_slides_xml(&self, zip: &mut ZipWriter<Cursor<Vec<u8>>>, opts: FileOptions) -> Result<()> {
        for (i, slide) in self.slides.iter().enumerate() {
            let xml = xml::generate_slide_xml(
                i + 1,
                slide.title.as_deref(),
                &slide.content,
                match slide.slide_type {
                    SlideType::Title => "title",
                    SlideType::Content => "content",
                }
            );
            zip.start_file(format!("ppt/slides/slide{}.xml", i + 1), opts)?;
            zip.write_all(xml.as_bytes())?;
        }
        Ok(())
    }
    
    fn add_content_types(&self, zip: &mut ZipWriter<Cursor<Vec<u8>>>, opts: FileOptions) -> Result<()> {
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  {}
</Types>"#,
            (1..=self.slides.len())
                .map(|i| format!("  <Override PartName=\"/ppt/slides/slide{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>", i))
                .collect::<Vec<_>>()
                .join("\n")
        );
        zip.start_file("[Content_Types].xml", opts)?;
        zip.write_all(xml.as_bytes())?;
        Ok(())
    }
    
    fn add_core_properties(&self, zip: &mut ZipWriter<Cursor<Vec<u8>>>, opts: FileOptions) -> Result<()> {
        let author = self.author.as_deref().unwrap_or("Unknown");
        let title = self.title.as_deref().unwrap_or("Untitled");
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <dc:creator>{}</dc:creator>
  <cp:lastModifiedBy>{}</cp:lastModifiedBy>
  <dc:title>{}</dc:title>
  <dcmitype:imeType>application/vnd.openxmlformats-officedocument.presentationml.presentation</dcmitype:imeType>
</cp:coreProperties>"#,
            author, author, title
        );
        zip.start_file("_docProps/core.xml", opts)?;
        zip.write_all(xml.as_bytes())?;
        Ok(())
    }
    
    fn add_app_properties(&self, zip: &mut ZipWriter<Cursor<Vec<u8>>>, opts: FileOptions) -> Result<()> {
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">
  <TotalTime>0</TotalTime>
  <Slides>{}</Slides>
  <Notes>{}</Notes>
  <HiddenSlides>0</HiddenSlides>
  <MMClips>0</MMClips>
  <ScaleCrop>0</ScaleCrop>
  <HeadingPairs>
    <vt:vector size="2" baseType="variant">
      <vt:variant><vt:lpstr>Presentation Name</vt:lpstr></vt:variant>
      <vt:variant><vt:bstr>{}</vt:bstr></vt:variant>
    </vt:vector>
  </HeadingPairs>
  <TitlesOfParts>
    <vt:vector size="1" baseType="lpstr">
      <vt:lpstr>{}</vt:lpstr>
    </vt:vector>
  </TitlesOfParts>
  <Company/>
  <Manager/>
</Properties>"#,
            self.slides.len(),
            self.slides.len(),
            self.title.as_deref().unwrap_or("Untitled"),
            self.title.as_deref().unwrap_or("Untitled")
        );
        zip.start_file("_docProps/app.xml", opts)?;
        zip.write_all(xml.as_bytes())?;
        Ok(())
    }
}

pub struct Slide {
    slide_type: SlideType,
    title: Option<String>,
    content: Vec<String>,
}

pub enum SlideType {
    Title,
    Content,
}

#[derive(Clone)]
pub struct Theme {
    primary_color: String,
    font_family: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary_color: "#2E5090".to_string(),
            font_family: "Calibri".to_string(),
        }
    }
}

impl Theme {
    pub fn corporate_blue() -> Self {
        Self {
            primary_color: "#2E5090".to_string(),
            font_family: "Calibri".to_string(),
        }
    }
}