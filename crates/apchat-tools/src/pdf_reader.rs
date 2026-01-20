//! PDF reader tool for extracting text from PDF files

use anyhow::{Context, Result};
use std::path::Path;
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

    let mut text_content = String::new();

    for (page_num, page_id) in doc.get_pages().iter().take(max_pages) {
        // Extract text from the page
        match extract_text_from_page(&doc, *page_id) {
            Ok(page_text) => {
                text_content.push_str(&format!("--- Page {} ---\n", page_num));
                text_content.push_str(&page_text);
                text_content.push('\n');
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
    let mut text = String::new();

    // Get all content streams for this page
    let content_ids = doc.get_page_contents(page_id);

    for content_id in content_ids {
        if let Ok(content_obj) = doc.get_object(content_id) {
            if let Ok(stream) = content_obj.as_stream() {
                // Decompress and decode the content stream
                if let Ok(decoded_content) = stream.decode_content() {
                    let operations = decoded_content.operations;

                    for op in operations {
                        // Look for text show operations (Tj, TJ, ', ")
                        // operator is a Vec<u8>, so we compare slices
                        let op_name: &[u8] = op.operator.as_bytes();
                        if op_name == b"Tj" || op_name == b"'" {
                            // Tj: show single string
                            if let Some(args) = op.operands.get(0) {
                                if let Ok(bytes) = args.as_str() {
                                    if let Ok(s) = std::str::from_utf8(bytes) {
                                        text.push_str(s);
                                    }
                                }
                            }
                        } else if op_name == b"TJ" {
                            // TJ: show array of strings with spacing adjustments
                            for arg in &op.operands {
                                if let Ok(arr) = arg.as_array() {
                                    for item in arr {
                                        if let Ok(bytes) = item.as_str() {
                                            if let Ok(s) = std::str::from_utf8(bytes) {
                                                text.push_str(s);
                                            }
                                        }
                                    }
                                }
                            }
                        } else if op_name == b"\"" {
                            // \": move to next line and show text
                            if op.operands.len() >= 3 {
                                if let Ok(bytes) = op.operands[2].as_str() {
                                    if let Ok(s) = std::str::from_utf8(bytes) {
                                        text.push(' ');
                                        text.push_str(s);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Basic cleanup: add spaces where appropriate
    text = text.replace("  ", " ");

    Ok(text)
}
