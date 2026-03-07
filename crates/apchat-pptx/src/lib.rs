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