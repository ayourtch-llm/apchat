//! Integration tests for the read_image tool

use apchat_tools::read_image::ReadImageTool;
use apchat_toolcore::{Tool, ToolParameters, ToolContext};
use std::collections::HashMap;
use std::io::Write;
use tempfile::tempdir;

#[tokio::test]
async fn test_read_image_tool_success() {
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
        ..Default::default()
    };

    let tool = ReadImageTool;
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), image_path.to_string());

    let result = tool.execute(params.into(), &context).await;
    assert!(result.is_success(), "Expected success, got: {:?}", result);
    
    // Check that result contains expected content
    let result_str = result.as_success().unwrap();
    assert!(result_str.contains("Image encoded successfully"));
    assert!(result_str.contains("PNG"));
    assert!(result_str.contains("<|image_start|>"));
    assert!(result_str.contains("<|image_end|>"));
}

#[tokio::test]
async fn test_read_image_tool_file_not_found() {
    let temp_dir = tempdir().unwrap();
    let context = ToolContext {
        work_dir: temp_dir.path().to_path_buf(),
        ..Default::default()
    };

    let tool = ReadImageTool;
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), "nonexistent.png".to_string());

    let result = tool.execute(params.into(), &context).await;
    assert!(result.is_error(), "Expected error for missing file");
    
    let result_str = result.as_error().unwrap();
    assert!(result_str.contains("not found"));
}

#[tokio::test]
async fn test_read_image_tool_too_large() {
    let temp_dir = tempdir().unwrap();
    let image_path = "large_image.png";
    let full_path = temp_dir.path().join(image_path);

    // Create a file larger than 50MB
    let mut file = std::fs::File::create(&full_path).unwrap();
    let large_data = vec![0u8; 51 * 1024 * 1024]; // 51 MB
    file.write_all(&large_data).unwrap();
    drop(file);

    let context = ToolContext {
        work_dir: temp_dir.path().to_path_buf(),
        ..Default::default()
    };

    let tool = ReadImageTool;
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), image_path.to_string());

    let result = tool.execute(params.into(), &context).await;
    assert!(result.is_error(), "Expected error for large file");
    
    let result_str = result.as_error().unwrap();
    assert!(result_str.contains("too large"));
    assert!(result_str.contains("50 MB"));
}

#[tokio::test]
async fn test_read_image_tool_custom_size_limit() {
    let temp_dir = tempdir().unwrap();
    let image_path = "test_image.png";
    let full_path = temp_dir.path().join(image_path);

    // Create a minimal valid PNG file
    let png_data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
        0x00, 0x00, 0x00, 0x0D,
        0x49, 0x48, 0x44, 0x52,
        0x00, 0x00, 0x00, 0x01,
        0x00, 0x00, 0x00, 0x01,
        0x08, 0x02,
        0x00, 0x00, 0x00, 0x00,
        0x90, 0x77, 0x53, 0xDE,
        0x00, 0x00, 0x00, 0x0A,
        0x49, 0x44, 0x41, 0x54,
        0x08, 0xD7, 0x63, 0xF8, 0xFF, 0xFF, 0x3F, 0x00,
        0x05, 0xFE, 0x02, 0xFE,
        0xDC, 0xBC, 0x69, 0x57,
        0x00, 0x00, 0x00, 0x00,
        0x49, 0x45, 0x4E, 0x44,
        0xAE, 0x42, 0x60, 0x82,
    ];

    let mut file = std::fs::File::create(&full_path).unwrap();
    file.write_all(&png_data).unwrap();
    drop(file);

    let context = ToolContext {
        work_dir: temp_dir.path().to_path_buf(),
        ..Default::default()
    };

    let tool = ReadImageTool;
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), image_path.to_string());
    params.insert("max_size_mb".to_string(), "1".to_string());

    let result = tool.execute(params.into(), &context).await;
    assert!(result.is_success(), "Expected success for small file with 1MB limit");
}

#[tokio::test]
async fn test_read_image_tool_jpeg_format() {
    let temp_dir = tempdir().unwrap();
    let image_path = "test_image.jpg";
    let full_path = temp_dir.path().join(image_path);

    // Create a minimal JPEG file (not a real JPEG, but enough to test format detection)
    let jpeg_data = vec![
        0xFF, 0xD8, // JPEG start
        0xFF, 0xE0, // APP0 marker
        0x00, 0x10, // Length
        0x4A, 0x46, 0x49, 0x46, // "JFIF"
        0x00, 0x01,
        0x01, 0x00,
        0x00, 0x01,
        0x01, 0x00,
        0x00,
        0xFF, 0xD9, // JPEG end
    ];

    let mut file = std::fs::File::create(&full_path).unwrap();
    file.write_all(&jpeg_data).unwrap();
    drop(file);

    let context = ToolContext {
        work_dir: temp_dir.path().to_path_buf(),
        ..Default::default()
    };

    let tool = ReadImageTool;
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), image_path.to_string());

    let result = tool.execute(params.into(), &context).await;
    assert!(result.is_success(), "Expected success for JPEG");
    
    let result_str = result.as_success().unwrap();
    assert!(result_str.contains("JPEG") || result_str.contains("JPG"));
}

#[tokio::test]
async fn test_read_image_tool_webp_format() {
    let temp_dir = tempdir().unwrap();
    let image_path = "test_image.webp";
    let full_path = temp_dir.path().join(image_path);

    // Create a minimal WebP file
    let webp_data = vec![
        0x52, 0x49, 0x46, 0x46, // "RIFF"
        0x1A, 0x00, 0x00, 0x00, // Length
        0x57, 0x45, 0x42, 0x50, // "WEBP"
        0x56, 0x50, 0x38, 0x20, // "VP8 "
        0x0E, 0x00, 0x00, 0x00, // Chunk length
        0x00, 0x00, 0x00, 0x00, // VP8 data
        0xFF, 0xFF, 0xFF, 0xFF, // Padding
    ];

    let mut file = std::fs::File::create(&full_path).unwrap();
    file.write_all(&webp_data).unwrap();
    drop(file);

    let context = ToolContext {
        work_dir: temp_dir.path().to_path_buf(),
        ..Default::default()
    };

    let tool = ReadImageTool;
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), image_path.to_string());

    let result = tool.execute(params.into(), &context).await;
    assert!(result.is_success(), "Expected success for WebP");
    
    let result_str = result.as_success().unwrap();
    assert!(result_str.contains("WEBP") || result_str.contains("WEBP"));
}