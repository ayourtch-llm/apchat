use apchat_toolcore::content_limiter::{ContentLimiter, ContentLimiterConfig};
use std::path::PathBuf;
use tempfile::TempDir;
use std::fs;

#[cfg(test)]
mod content_limiter_tests {
    use super::*;

    #[test]
    fn test_content_limiter_config_default() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().to_path_buf();
        
        let config = ContentLimiterConfig::new(&work_dir);
        
        assert_eq!(config.max_content_length, 20_000);
        // The directory is only created when save_and_truncate is called
        assert_eq!(config.large_outputs_dir, work_dir.join(".apchat-large-outputs"));
    }

    #[test]
    fn test_content_limiter_config_custom_max_length() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().to_path_buf();
        
        let config = ContentLimiterConfig::new(&work_dir)
            .with_max_length(1000);
        
        assert_eq!(config.max_content_length, 1000);
    }

    #[test]
    fn test_content_limiter_is_content_too_large() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().to_path_buf();
        
        let config = ContentLimiterConfig::new(&work_dir);
        let limiter = ContentLimiter::new(config);
        
        // Small content
        assert!(!limiter.is_content_too_large("Small text"));
        
        // Content exactly at limit
        let small_content = "a".repeat(20_000);
        assert!(!limiter.is_content_too_large(&small_content));
        
        // Content over limit
        let large_content = "a".repeat(20_001);
        assert!(limiter.is_content_too_large(&large_content));
    }

    #[test]
    fn test_content_limiter_save_and_truncate() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().to_path_buf();
        
        let config = ContentLimiterConfig::new(&work_dir);
        let limiter = ContentLimiter::new(config);
        
        // Small content - should not truncate
        let small_content = "Small text".to_string();
        let (result_content, note, was_truncated) = limiter.save_and_truncate(small_content.clone(), "test_tool");
        
        assert_eq!(result_content, small_content);
        assert!(note.is_none());
        assert!(!was_truncated);
    }

    #[test]
    fn test_content_limiter_save_and_truncate_large_content() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().to_path_buf();
        
        let config = ContentLimiterConfig::new(&work_dir);
        let limiter = ContentLimiter::new(config);
        
        // Large content - should truncate and save to file
        let large_content = "a".repeat(25_000);
        let (result_content, note, was_truncated) = limiter.save_and_truncate(large_content.clone(), "test_tool");
        
        assert!(result_content.contains("🚨 LARGE OUTPUT TRUNCATED 🚨"));
        assert!(result_content.contains("test_tool"));
        assert!(note.is_some());
        assert!(was_truncated);
        
        // Verify the file was created
        let note_text = note.unwrap();
        assert!(note_text.contains("💡 TO INSPECT FULL OUTPUT:"));
        assert!(note_text.contains("read_file"));
        
        // Parse the file path from the note
        let file_path = note_text.split("  ").last().unwrap().trim();
        assert!(fs::exists(file_path).unwrap());
        
        // Verify the content was saved correctly
        let saved_content = fs::read_to_string(file_path).unwrap();
        assert_eq!(saved_content, large_content);
    }

    #[test]
    fn test_content_limiter_truncation_message_format() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().to_path_buf();
        
        let config = ContentLimiterConfig::new(&work_dir);
        let limiter = ContentLimiter::new(config);
        
        let large_content = "a".repeat(25_000);
        let (result_content, note, _) = limiter.save_and_truncate(large_content, "my_tool");
        
        assert!(result_content.contains("🚨 LARGE OUTPUT TRUNCATED 🚨"));
        assert!(result_content.contains("my_tool"));
        assert!(result_content.contains("exceeds maximum display length"));
        
        let note_text = note.unwrap();
        assert!(note_text.contains("💡 TO INSPECT FULL OUTPUT:"));
        assert!(note_text.contains("read_file"));
    }

    #[test]
    fn test_content_limiter_custom_max_length() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().to_path_buf();
        
        let config = ContentLimiterConfig::new(&work_dir)
            .with_max_length(100);
        let limiter = ContentLimiter::new(config);
        
        // Small content
        assert!(!limiter.is_content_too_large("Small"));
        
        // Content at custom limit
        let at_limit = "a".repeat(100);
        assert!(!limiter.is_content_too_large(&at_limit));
        
        // Content over custom limit
        let over_limit = "a".repeat(101);
        assert!(limiter.is_content_too_large(&over_limit));
    }

    #[test]
    fn test_content_limiter_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().to_path_buf();
        
        // Remove the large outputs directory if it exists
        let large_dir = work_dir.join(".apchat-large-outputs");
        if large_dir.exists() {
            fs::remove_dir_all(&large_dir).unwrap();
        }
        
        let config = ContentLimiterConfig::new(&work_dir);
        let limiter = ContentLimiter::new(config);
        
        // This should create the directory
        let large_content = "a".repeat(25_000);
        limiter.save_and_truncate(large_content, "test_tool");
        
        assert!(large_dir.exists());
        assert!(large_dir.is_dir());
    }

    #[test]
    fn test_content_limiter_error_handling() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().to_path_buf();
        
        // Create an invalid config with a directory that can't be created
        // (this is a bit tricky to test, but we can at least verify the function
        // handles errors gracefully)
        let config = ContentLimiterConfig::new(&work_dir);
        let limiter = ContentLimiter::new(config);
        
        // Even with a very large content that might cause issues,
        // the function should handle it gracefully
        let very_large_content = "a".repeat(1_000_000);
        let (result_content, _note, _was_truncated) = limiter.save_and_truncate(very_large_content, "test_tool");
        
        // Should still return some result (either truncated or original)
        assert!(!result_content.is_empty());
    }
}