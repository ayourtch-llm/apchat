//! Tool implementation for PDF reading

use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use crate::pdf_reader;
use async_trait::async_trait;
use std::collections::HashMap;

/// Tool for reading PDF files
pub struct ReadPdfTool;

#[async_trait]
impl Tool for ReadPdfTool {
    fn name(&self) -> &str {
        "read_pdf"
    }

    fn description(&self) -> &str {
        "Read and extract text from a PDF file. Returns the text content with pages clearly marked. Works best with text-based PDFs; scanned/image-based PDFs may return no text. Maximum file size: 50MB."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("file_path", "string", "Path to the PDF file relative to the work directory", required),
            param!("max_pages", "integer", "Maximum number of pages to extract (optional, extracts all pages by default)", optional),
            param!("start_page", "integer", "Starting page number to extract from (optional, defaults to 1)", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let file_path = match params.get_required::<String>("file_path") {
            Ok(path) => path,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let max_pages = params.get_optional::<usize>("max_pages").unwrap_or(None);
        let start_page = params.get_optional::<usize>("start_page").unwrap_or(None);

        match pdf_reader::extract_text_from_pdf(&context.work_dir, &file_path, max_pages, start_page).await {
            Ok(content) => {
                // Check if content is too large and needs truncation warning
                const MAX_SIZE: usize = 100_000; // ~100k chars
                if content.len() > MAX_SIZE {
                    let truncated = format!(
                        "{}\n\n[... Content truncated. The PDF is very large. Consider using max_pages parameter to read fewer pages. ...]",
                        &content[..MAX_SIZE]
                    );
                    ToolResult::success_with_truncation(truncated, file_path)
                } else {
                    ToolResult::success(content)
                }
            }
            Err(e) => ToolResult::error(format!("Failed to read PDF: {}", e)),
        }
    }
}
