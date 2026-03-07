mod error;
pub mod xml;

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