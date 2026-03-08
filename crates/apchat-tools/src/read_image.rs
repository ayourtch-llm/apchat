//! Tool implementation for image encoding and processing

use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

/// Tool for encoding images to base64 for use with multimodal models
pub struct ReadImageTool;

#[async_trait]
impl Tool for ReadImageTool {
    fn name(&self) -> &str {
        "read_image"
    }

    fn description(&self) -> &str {
        "Encode an image file to base64 for use with multimodal LLMs (Qwen3.5, Qwen3-VL). Supports JPEG, PNG, WebP, BMP formats. Returns base64-encoded image data in data: URI format. IMPORTANT: This tool produces large base64 output that consumes significant context window. ALWAYS use this tool inside a fresh subagent with a focused instruction to work on the image, rather than in the main conversation context."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("file_path", "string", "Path to the image file relative to the work directory", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let file_path = match params.get_required::<String>("file_path") {
            Ok(path) => path,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        // Construct full path
        let full_path = context.work_dir.join(&file_path);

        // Validate file exists
        if !full_path.exists() {
            return ToolResult::error(format!("Image file not found: {}", file_path));
        }

        // Determine file extension for MIME type
        let extension = full_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg")
            .to_lowercase();

        let mime_type = match extension.as_str() {
            "png" => "image/png",
            "jpeg" | "jpg" => "image/jpeg",
            "webp" => "image/webp",
            "bmp" => "image/bmp",
            "gif" => "image/gif",
            _ => "image/jpeg", // default
        };

        // Read file contents
        let image_data = match fs::read(&full_path) {
            Ok(data) => data,
            Err(e) => return ToolResult::error(format!("Failed to read image file: {}", e)),
        };

        // Encode to base64
        let base64_data = BASE64.encode(&image_data);

        // Return in data: URI format compatible with multimodal models
        // This format: data:image/jpeg;base64,<base64-data>
        // Can be used directly in image_url fields for multimodal APIs
        let result = format!(
            "data:{};base64,{}",
            mime_type, base64_data
        );

        ToolResult::success(result)
    }
}