# apchat-pptx Crate Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use `executing-plans` to implement this plan task-by-task.

**Goal:** Create a pure Rust library for generating PPTX presentations with theme support, integrated as tools into APChat.

**Architecture:** New crate `apchat-pptx` implementing PPTX format (ZIP of XML files) with builder pattern API. Theme system for consistent styling. Tools registered in APChat when `--pptx-tools` flag is used.

**Tech Stack:** Rust, `zip` crate, `quick-xml` for XML generation, existing APChat tooling framework.

---

## Phase 1: Core Infrastructure

### Task 1: Create Crate Structure

**Files:**
- Create: `crates/apchat-pptx/Cargo.toml`
- Create: `crates/apchat-pptx/src/lib.rs`
- Create: `crates/apchat-pptx/src/error.rs`

**Step 1: Write Cargo.toml**

```toml
[package]
name = "apchat-pptx"
version = "0.1.0"
edition = "2021"
description = "Pure Rust PPTX presentation creation library"
license = "MIT"

[dependencies]
zip = "0.6"
quick-xml = { version = "0.31", features = ["serialize"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4"] }
thiserror = "1.0"

[dev-dependencies]
tempfile = "3.0"
```

**Step 2: Write error.rs**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PptxError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    
    #[error("XML error: {0}")]
    Xml(#[from] quick_xml::Error),
    
    #[error("Invalid presentation: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, PptxError>;
```

**Step 3: Write lib.rs (initial)**

```rust
mod error;

pub use error::{PptxError, Result};

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
```

**Step 4: Verify crate compiles**

Run: `cd crates/apchat-pptx && cargo check`
Expected: SUCCESS

**Step 5: Commit**

```bash
git add crates/apchat-pptx
git commit -m "feat: create apchat-pptx crate with basic structure"
```

---

### Task 2: Add Builder Pattern to Presentation

**Files:**
- Modify: `crates/apchat-pptx/src/lib.rs`
- Create: `crates/apchat-pptx/tests/basic_presentation.rs`

**Step 1: Write failing test**

```rust
use apchat_pptx::{Presentation, Theme};

#[test]
fn test_presentation_builder() {
    let ppt = Presentation::new()
        .title("Test Presentation")
        .author("Test Author")
        .theme(Theme::corporate_blue());
    
    assert_eq!(ppt.get_title(), Some("Test Presentation"));
    assert_eq!(ppt.get_author(), Some("Test Author"));
}
```

**Step 2: Run test to verify it fails**

Run: `cd crates/apchat-pptx && cargo test test_presentation_builder`
Expected: FAIL (methods don't exist)

**Step 3: Implement builder pattern**

Modify `lib.rs`:
```rust
impl Presentation {
    // ... existing new() ...
    
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
}
```

**Step 4: Run test to verify it passes**

Run: `cd crates/apchat-pptx && cargo test test_presentation_builder`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/apchat-pptx
git commit -m "feat: add builder pattern to Presentation"
```

---

### Task 3: Implement Slide Addition Methods

**Files:**
- Modify: `crates/apchat-pptx/src/lib.rs`
- Modify: `crates/apchat-pptx/tests/basic_presentation.rs`

**Step 1: Write failing tests**

Add to test file:
```rust
#[test]
fn test_add_title_slide() {
    let mut ppt = Presentation::new();
    ppt.add_title_slide("Main Title", "Subtitle");
    assert_eq!(ppt.slides_count(), 1);
}

#[test]
fn test_add_content_slide() {
    let mut ppt = Presentation::new();
    ppt.add_content_slide("Slide Title", vec!["Bullet 1", "Bullet 2"]);
    assert_eq!(ppt.slides_count(), 1);
}
```

**Step 2: Run tests to verify they fail**

Run: `cd crates/apchat-pptx && cargo test slide`
Expected: FAIL

**Step 3: Implement slide methods**

Modify `lib.rs`:
```rust
impl Presentation {
    // ... existing methods ...
    
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
}
```

**Step 4: Run tests to verify they pass**

Run: `cd crates/apchat-pptx && cargo test slide`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/apchat-pptx
git commit -m "feat: implement slide addition methods"
```

---

## Phase 2: PPTX XML Generation

### Task 4: Create XML Module

**Files:**
- Create: `crates/apchat-pptx/src/xml.rs`
- Create: `crates/apchat-pptx/tests/xml_generation.rs`

**Step 1: Write failing test**

```rust
use apchat_pptx::xml::generate_presentation_xml;

#[test]
fn test_presentation_xml() {
    let xml = generate_presentation_xml(5);
    assert!(xml.contains("<p:presentation"));
    assert!(xml.contains("slideIdLst"));
}
```

**Step 2: Run test to verify it fails**

Run: `cd crates/apchat-pptx && cargo test test_presentation_xml`
Expected: FAIL

**Step 3: Implement XML generation**

Create `xml.rs`:
```rust
pub fn generate_presentation_xml(slide_count: usize) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n");
    xml.push_str("<p:presentation xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">\n");
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
```

**Step 4: Run test to verify it passes**

Run: `cd crates/apchat-pptx && cargo test test_presentation_xml`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/apchat-pptx/src/xml.rs crates/apchat-pptx/tests/xml_generation.rs
git commit -m "feat: implement PPTX XML generation"
```

---

### Task 5: Implement ZIP Packaging and Save

**Files:**
- Modify: `crates/apchat-pptx/src/lib.rs`
- Modify: `crates/apchat-pptx/src/xml.rs`
- Modify: `crates/apchat-pptx/tests/basic_presentation.rs`

**Step 1: Write failing test**

```rust
use apchat_pptx::Presentation;
use std::fs;

#[test]
fn test_save_presentation() {
    let mut ppt = Presentation::new()
        .title("Test")
        .author("Test Author");
    
    ppt.add_title_slide("Title", "Subtitle");
    ppt.add_content_slide("Content", vec!["Bullet 1", "Bullet 2"]);
    
    let path = "/tmp/test_presentation.pptx";
    ppt.save(path).unwrap();
    
    assert!(fs::metadata(path).is_ok());
    
    // Cleanup
    fs::remove_file(path).ok();
}
```

**Step 2: Run test to verify it fails**

Run: `cd crates/apchat-pptx && cargo test test_save_presentation`
Expected: FAIL

**Step 3: Implement save method**

Modify `lib.rs` to add save logic with ZIP creation:
```rust
use std::fs::File;
use std::io::Cursor;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

mod xml;

impl Presentation {
    pub fn save(&self, path: &str) -> Result<()> {
        let file = File::create(path)?;
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        
        let opts = SimpleFileOptions::default()
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
    
    fn add_presentation_xml(&self, zip: &mut ZipWriter<Cursor<Vec<u8>>>, opts: SimpleFileOptions) -> Result<()> {
        let xml = xml::generate_presentation_xml(self.slides.len());
        zip.start_file("ppt/presentation.xml", opts)?;
        zip.write_all(xml.as_bytes())?;
        Ok(())
    }
    
    fn add_slides_xml(&self, zip: &mut ZipWriter<Cursor<Vec<u8>>>, opts: SimpleFileOptions) -> Result<()> {
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
    
    fn add_content_types(&self, zip: &mut ZipWriter<Cursor<Vec<u8>>>, opts: SimpleFileOptions) -> Result<()> {
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
    
    fn add_core_properties(&self, zip: &mut ZipWriter<Cursor<Vec<u8>>>, opts: SimpleFileOptions) -> Result<()> {
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
    
    fn add_app_properties(&self, zip: &mut ZipWriter<Cursor<Vec<u8>>>, opts: SimpleFileOptions) -> Result<()> {
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
```

**Step 4: Run test to verify it passes**

Run: `cd crates/apchat-pptx && cargo test test_save_presentation`
Expected: PASS

**Step 5: Verify file is valid PPTX**

Run: `file /tmp/test_presentation.pptx`
Expected: Should show ZIP archive

**Step 6: Commit**

```bash
git add crates/apchat-pptx
git commit -m "feat: implement PPTX save with ZIP packaging"
```

---

## Phase 3: APChat Integration

### Task 6: Add PPTX Tools to APChat

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `src/main.rs` or appropriate CLI entry point
- Create: `src/tools/pptx_tools.rs`

**Step 1: Add dependency to workspace**

Modify root `Cargo.toml`:
```toml
[workspace.members]
# ... existing members ...
members = [
    ".",
    "crates/apchat-pptx",
]
```

Add to main Cargo.toml dependencies:
```toml
apchat-pptx = { path = "crates/apchat-pptx" }
```

**Step 2: Create pptx_tools.rs**

```rust
use apchat_pptx::Presentation;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreatePresentationArgs {
    path: String,
    title: String,
    author: Option<String>,
    slides: Vec<SlideArgs>,
}

#[derive(Deserialize)]
pub struct SlideArgs {
    r#type: String,  // "title" or "content"
    title: String,
    subtitle: Option<String>,
    bullets: Option<Vec<String>>,
}

pub fn create_presentation(args: CreatePresentationArgs) -> Result<String, String> {
    let mut ppt = Presentation::new()
        .title(&args.title);
    
    if let Some(author) = args.author {
        ppt = ppt.author(&author);
    }
    
    for slide in args.slides {
        match slide.r#type.as_str() {
            "title" => {
                let subtitle = slide.subtitle.unwrap_or_else(|| "Subtitle".to_string());
                ppt.add_title_slide(&slide.title, &subtitle);
            }
            "content" => {
                let bullets = slide.bullets.unwrap_or_default();
                ppt.add_content_slide(&slide.title, bullets.into_iter().map(|s| s.as_str()).collect());
            }
            _ => return Err(format!("Unknown slide type: {}", slide.r#type)),
        }
    }
    
    ppt.save(&args.path)
        .map_err(|e| e.to_string())?;
    
    Ok(format!("Created presentation '{}' with {} slides", args.path, args.slides.len()))
}
```

**Step 3: Register tool in APChat**

Modify appropriate file (likely `src/main.rs` or `src/tools/mod.rs`):
```rust
#[cfg(feature = "pptx-tools")]
mod pptx_tools;

#[cfg(feature = "pptx-tools")]
use pptx_tools::create_presentation;

// In tool registration:
#[cfg(feature = "pptx-tools")]
tools.register("create_presentation", create_presentation);
```

**Step 4: Add CLI flag support**

Add to args parsing:
```rust
#[argh(subcommand)]
pub struct PptxTools {
    #[argh(option)]
    enabled: bool,
}

// Or simpler:
#[argh(switch)]
pptx_tools: bool,
```

**Step 5: Test integration**

Run: `cargo run -- --pptx-tools --help`
Expected: Should show PPTX tools available

**Step 6: Commit**

```bash
git add Cargo.toml src/
git commit -m "feat: integrate apchat-pptx tools into APChat"
```

---

## Phase 4: Testing & Documentation

### Task 7: Add Integration Tests

**Files:**
- Create: `crates/apchat-pptx/tests/integration.rs`

**Step 1: Write comprehensive integration test**

```rust
use apchat_pptx::{Presentation, Theme};
use std::fs;

#[test]
fn test_full_presentation_workflow() {
    let mut ppt = Presentation::new()
        .title("Q3 Executive Review")
        .author("APChat AI")
        .theme(Theme::corporate_blue());
    
    ppt.add_title_slide("Strategic Insights", "Q3 2024");
    ppt.add_content_slide("Key Metrics", vec![
        "Revenue: +23% YoY",
        "User growth: 1.2M new users",
        "NPS: 72 (up 8 points)"
    ]);
    ppt.add_content_slide("Market Analysis", vec![
        "Market share: 34%",
        "Growth rate: 15% CAGR",
        "Competitive advantage maintained"
    ]);
    
    let path = "/tmp/integration_test.pptx";
    ppt.save(path).unwrap();
    
    let metadata = fs::metadata(path).unwrap();
    assert!(metadata.len() > 0);
    
    fs::remove_file(path).ok();
}
```

**Step 2: Run integration tests**

Run: `cd crates/apchat-pptx && cargo test --test integration`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/apchat-pptx/tests/integration.rs
git commit -m "test: add integration tests"
```

### Task 8: Add Examples and Documentation

**Files:**
- Create: `crates/apchat-pptx/examples/create_basic.rs`
- Create: `crates/apchat-pptx/README.md`

**Step 1: Write example**

```rust
use apchat_pptx::{Presentation, Theme};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ppt = Presentation::new()
        .title("My Presentation")
        .author("My Name")
        .theme(Theme::corporate_blue());
    
    ppt.add_title_slide("Welcome", "An Example Presentation");
    
    ppt.add_content_slide("Features", vec![
        "Pure Rust implementation",
        "Theme support",
        "Easy to use API"
    ]);
    
    ppt.add_content_slide("Thanks!", vec![
        "Questions?",
        "Contact: example@email.com"
    ]);
    
    ppt.save("examples/output.pptx")?;
    println!("Created examples/output.pptx");
    
    Ok(())
}
```

**Step 2: Write README**

```markdown
# apchat-pptx

Pure Rust library for creating PPTX presentations.

## Features

- Create presentations from scratch
- Title and content slides
- Theme support (corporate blue, etc.)
- Builder pattern API

## Usage

```rust
use apchat_pptx::{Presentation, Theme};

let mut ppt = Presentation::new()
    .title("My Presentation")
    .author("My Name")
    .theme(Theme::corporate_blue());

ppt.add_title_slide("Welcome", "Subtitle");
ppt.add_content_slide("Content", vec!["Bullet 1", "Bullet 2"]);

ppt.save("output.pptx")?;
```

## APChat Integration

Enable with `--pptx-tools` flag to use PPTX creation tools in APChat.
```

**Step 3: Run example**

Run: `cd crates/apchat-pptx && cargo run --example create_basic`
Expected: Creates `examples/output.pptx`

**Step 4: Commit**

```bash
git add crates/apchat-pptx/examples crates/apchat-pptx/README.md
git commit -m "docs: add examples and README"
```

---

## Verification Checklist

- [ ] Crate compiles: `cargo check --all`
- [ ] All tests pass: `cargo test --all`
- [ ] Example runs: `cargo run --example create_basic -p apchat-pptx`
- [ ] APChat builds with feature: `cargo build --features pptx-tools`
- [ ] Generated PPTX opens in PowerPoint/LibreOffice