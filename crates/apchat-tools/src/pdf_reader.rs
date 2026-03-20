//! PDF reader tool for extracting text from PDF files

use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PdfError {
    #[error("PDF file not found: {0}")]
    FileNotFound(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("file exceeds maximum allowed size of {0} bytes")]
    FileTooLarge(usize),

    #[error("failed to parse PDF: {0}")]
    ParseError(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Lopdf(#[from] lopdf::Error),
}

const MAX_PDF_SIZE: usize = 50 * 1024 * 1024; // 50 MiB

/// Extract text from a PDF file
///
/// Returns the extracted text content from the PDF, with pages separated by form feed characters.
pub async fn extract_text_from_pdf(
    work_dir: &Path,
    file_path: impl AsRef<Path>,
    max_pages: Option<usize>,
    start_page: Option<usize>,
) -> Result<String> {
    let abs_path = work_dir.join(file_path.as_ref());

    // Check if the file exists
    if !abs_path.exists() {
        return Err(PdfError::FileNotFound(abs_path.display().to_string()).into());
    }

    // Canonicalize the path
    let canonical = abs_path.canonicalize().with_context(|| {
        format!("Failed to canonicalize path: {}", abs_path.display())
    })?;

    let work_canonical = work_dir.canonicalize().with_context(|| {
        format!("Failed to canonicalize workspace dir: {}", work_dir.display())
    })?;

    // Security check: ensure file is within workspace
    if !canonical.starts_with(&work_canonical) {
        return Err(PdfError::PermissionDenied(canonical.display().to_string()).into());
    }

    // Check if it's a directory
    if canonical.is_dir() {
        return Err(PdfError::FileNotFound(format!(
            "{} is a directory, not a file",
            canonical.display()
        ))
        .into());
    }

    // Size check
    let metadata = std::fs::metadata(&canonical)?;
    if metadata.len() > MAX_PDF_SIZE as u64 {
        return Err(PdfError::FileTooLarge(metadata.len() as usize).into());
    }

    // Load and parse the PDF
    let doc = lopdf::Document::load(&canonical)
        .map_err(|e| PdfError::ParseError(format!("Failed to load PDF: {}", e)))?;

    let num_pages = doc.get_pages().len();
    let max_pages = max_pages.unwrap_or(num_pages).min(num_pages);
    let start_page = start_page.unwrap_or(1).max(1);

    let mut text_content = String::new();

    for (page_num, page_id) in doc.get_pages().iter().skip(start_page.saturating_sub(1)).take(max_pages) {
        // Extract text from the page
        match extract_text_from_page(&doc, *page_id) {
            Ok(page_text) => {
                if !page_text.is_empty() {
                    text_content.push_str(&format!("--- Page {} ---\n", page_num));
                    text_content.push_str(&page_text);
                    text_content.push('\n');
                }
            }
            Err(e) => {
                // Continue on error, but note it
                text_content.push_str(&format!("--- Page {} (Error extracting text: {}) ---\n", page_num, e));
            }
        }
    }

    if text_content.trim().is_empty() {
        text_content = "[No extractable text found in PDF - it may be scanned or image-based]".to_string();
    }

    Ok(text_content)
}

/// Extract text from a specific page in the PDF
fn extract_text_from_page(doc: &lopdf::Document, page_id: (u32, u16)) -> Result<String> {
    use crate::pdf_content_parser::{decompress_stream_raw, extract_text_from_content_bytes};

    let mut text = String::new();

    // Get page object and build XObject map
    let page_obj = doc.get_object(lopdf::ObjectId::from(page_id))
        .context("Page object not found")?;

    let page_dict = page_obj.as_dict()
        .context("Page object is not a dictionary")?;

    // Build XObject map from resources
    let xobject_map = if let Ok(resources) = page_dict.get(b"Resources") {
        if let Ok(res_dict) = resources.as_dict() {
            if let Ok(xobjects) = res_dict.get(b"XObject") {
                if let Ok(xobj_dict) = xobjects.as_dict() {
                    let mut map = std::collections::HashMap::new();
                    for (name, obj_ref) in xobj_dict.iter() {
                        let name_str = String::from_utf8_lossy(name).to_string();
                        if let Ok(ref_id) = obj_ref.as_reference() {
                            map.insert(name_str, ref_id);
                        }
                    }
                    map
                } else {
                    std::collections::HashMap::new()
                }
            } else {
                std::collections::HashMap::new()
            }
        } else {
            std::collections::HashMap::new()
        }
    } else {
        std::collections::HashMap::new()
    };

    // Get all content streams for this page
    let content_ids = doc.get_page_contents(page_id);

    for content_id in content_ids {
        if let Ok(content_obj) = doc.get_object(content_id) {
            if let Ok(stream) = content_obj.as_stream() {
                // Use custom parser to work around lopdf's broken decode_content
                match decompress_stream_raw(stream) {
                    Ok(decompressed_bytes) => {
                        let page_text = extract_text_from_content_bytes(&decompressed_bytes, doc, &xobject_map);
                        text.push_str(&page_text);
                        // Don't add extra space here - TJ arrays and Tj operators handle spacing
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to decompress stream: {}", e);
                    }
                }
            }
        }
    }

    // Cleanup: normalize whitespace
    text = text.split_whitespace().collect::<Vec<_>>().join(" ");

    Ok(text)
}
