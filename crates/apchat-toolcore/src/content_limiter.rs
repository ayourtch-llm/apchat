use std::path::PathBuf;
use std::fs;
use chrono::Local;
use uuid::Uuid;

/// Maximum content length before truncation (default: 20,000 characters)
pub const DEFAULT_MAX_CONTENT_LENGTH: usize = 20_000;

/// Content limiter configuration
#[derive(Debug, Clone)]
pub struct ContentLimiterConfig {
    pub max_content_length: usize,
    pub large_outputs_dir: PathBuf,
}

impl ContentLimiterConfig {
    pub fn new(work_dir: &PathBuf) -> Self {
        let large_outputs_dir = work_dir.join(".apchat-large-outputs");
        Self {
            max_content_length: DEFAULT_MAX_CONTENT_LENGTH,
            large_outputs_dir,
        }
    }

    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_content_length = max_length;
        self
    }
}

/// Content limiter for handling large tool outputs
#[derive(Debug, Clone)]
pub struct ContentLimiter {
    pub config: ContentLimiterConfig,
}

impl ContentLimiter {
    pub fn new(config: ContentLimiterConfig) -> Self {
        Self { config }
    }

    /// Check if content exceeds maximum length
    pub fn is_content_too_large(&self, content: &str) -> bool {
        content.len() > self.config.max_content_length
    }

    /// Save large content to file and return truncated version with note
    pub fn save_and_truncate(&self, content: String, tool_name: &str) -> (String, Option<String>, bool) {
        if !self.is_content_too_large(&content) {
            return (content, None, false);
        }

        // Create large outputs directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(&self.config.large_outputs_dir) {
            eprintln!("Warning: Failed to create large outputs directory: {}", e);
            return (content, None, false);
        }

        // Generate unique filename
        let timestamp = Local::now().format("%Y%m%d-%H%M%S");
        let filename = format!("{}-{}-{}.txt", tool_name, timestamp, Uuid::new_v4());
        let file_path = self.config.large_outputs_dir.join(&filename);

        // Write content to file
        if let Err(e) = fs::write(&file_path, &content) {
            eprintln!("Warning: Failed to write large output to file: {}", e);
            return (content, None, false);
        }

        // Create truncated content with note
        let truncated_content = format!("🚨 LARGE OUTPUT TRUNCATED 🚨
Output from '{}' exceeds maximum display length ({} chars). IMPORTANT: If a tool allows to limit the size of reply, like using max_read_lines parameter in read_file tool - do it.
Full output saved to: {}",
                                       tool_name,
                                       self.config.max_content_length,
                                       file_path.display());
        
        // Add note about how to inspect the output
        let note = Some(format!("\n💡 TO INSPECT FULL OUTPUT:
Use the `read_file` tool with the file path shown above, or manually open:
  {}",
                               file_path.display()));

        (truncated_content, note, true)
    }
}
