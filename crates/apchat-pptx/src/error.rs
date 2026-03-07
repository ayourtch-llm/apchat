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