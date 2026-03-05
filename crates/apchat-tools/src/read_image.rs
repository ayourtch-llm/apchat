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
        "Encode an image file to base64 for use with multimodal LLMs (Qwen3.5, Qwen3-VL). Supports JPEG, PNG, WebP, BMP formats. Returns base64-encoded image data that can be included in multimodal prompts. Maximum file size: 50MB."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("file_path", "string", "Path to the image file relative to the work directory", required),
            param!("max_size_mb", "integer", "Maximum file size in MB (default: 50)", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let file_path = match params.get_required::<String>("file_path") {
            Ok(path) => path,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let max_size_mb = params.get_optional::<usize>("max_size_mb").unwrap_or(Some(50));
        let max_size_bytes = max_size_mb.unwrap_or(50) as u64 * 1024 * 1024;

        // Construct full path
        let full_path = context.work_dir.join(&file_path);

        // Validate file exists
        if !full_path.exists() {
            return ToolResult::error(format!("Image file not found: {}", file_path));
        }

        // Check file size
        let metadata = match fs::metadata(&full_path) {
            Ok(m) => m,
            Err(e) => return ToolResult::error(format!("Failed to read file metadata: {}", e)),
        };

        if metadata.len() > max_size_bytes {
            return ToolResult::error(format!(
                "Image too large: {} bytes (max {} MB)",
                metadata.len(),
                max_size_mb.unwrap_or(50)
            ));
        }

        // Read file contents
        let image_data = match fs::read(&full_path) {
            Ok(data) => data,
            Err(e) => return ToolResult::error(format!("Failed to read image file: {}", e)),
        };

        // Encode to base64
        let base64_data = BASE64.encode(&image_data);

        // Detect file format from extension
        let format = file_path
            .split('.')
            .last()
            .unwrap_or("unknown")
            .to_uppercase();

        // Return structured result with metadata
        let result = format!(
            "Image encoded successfully:\n\
             File: {}\n\
             Format: {}\n\
             Size: {} bytes\n\
             Base64 data:\n{}\n\
             \n\
             **Usage with Qwen3.5/Qwen3-VL:**\n\
             Include this base64 data in your multimodal prompt using the format:\n\
             ```\n\
             <image>{base64_data}</image>\n\
             ```\n\
             Or with Qwen3.5 special tokens:\n\
             ```\n\
             <|image_start|>\n\
             {base64_data}\n\
             <|image_end|>\n\
             ```",
            file_path,
            format,
            image_data.len(),
            base64_data
        );

        ToolResult::success(result)
    }
}

/// Generate a Qwen3.5-compatible multimodal prompt from image data
pub fn generate_qwen35_prompt(base64_image: &str, prompt_text: &str) -> String {
    format!(
        "<|image_start|>\n{}\n<|image_end|>\n{}",
        base64_image, prompt_text
    )
}

/// Validate image file format
pub fn is_valid_image_format(file_path: &str) -> bool {
    let ext = file_path
        .split('.')
        .last()
        .unwrap_or("")
        .to_lowercase();

    matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp" | "bmp" | "gif")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_valid_image_extensions() {
        assert!(is_valid_image_format("test.jpg"));
        assert!(is_valid_image_format("test.JPEG"));
        assert!(is_valid_image_format("test.png"));
        assert!(is_valid_image_format("test.webp"));
        assert!(is_valid_image_format("test.bmp"));
        assert!(!is_valid_image_format("test.pdf"));
        assert!(!is_valid_image_format("test.txt"));
    }

    #[test]
    fn test_qwen35_prompt_generation() {
        let base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
        let prompt = "What is in this image?";

        let result = generate_qwen35_prompt(base64, prompt);
        assert!(result.contains("<|image_start|>"));
        assert!(result.contains("<|image_end|>"));
        assert!(result.contains(prompt));
    }

    #[tokio::test]
    async fn test_read_image_tool() {
        let temp_dir = tempdir().unwrap();
        let image_path = "test_image.png";
        let full_path = temp_dir.path().join(image_path);

        // Create a minimal valid PNG file (1x1 pixel)
        let png_data = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, // IHDR chunk length
            0x49, 0x48, 0x44, 0x52, // IHDR
            0x00, 0x00, 0x00, 0x01, // width
            0x00, 0x00, 0x00, 0x01, // height
            0x08, 0x02, // bit depth, color type
            0x00, 0x00, 0x00, 0x00, // compression, filter, interlace
            0x90, 0x77, 0x53, 0xDE, // CRC
            0x00, 0x00, 0x00, 0x0A, // IDAT chunk length
            0x49, 0x44, 0x41, 0x54, // IDAT
            0x08, 0xD7, 0x63, 0xF8, 0xFF, 0xFF, 0x3F, 0x00,
            0x05, 0xFE, 0x02, 0xFE, // compressed data
            0xDC, 0xBC, 0x69, 0x57, // CRC
            0x00, 0x00, 0x00, 0x00, // IEND chunk length
            0x49, 0x45, 0x4E, 0x44, // IEND
            0xAE, 0x42, 0x60, 0x82, // CRC
        ];

        let mut file = std::fs::File::create(&full_path).unwrap();
        file.write_all(&png_data).unwrap();
        drop(file);

        let context = ToolContext {
            work_dir: temp_dir.path().to_path_buf(),
            // Add other required fields
            ..Default::default()
        };

        let tool = ReadImageTool;
        let mut params = HashMap::new();
        params.insert("file_path".to_string(), image_path.to_string());

        let result = tool.execute(params.into(), &context).await;
        assert!(result.is_success());
    }
}