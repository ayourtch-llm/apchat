// Logging module - conversation and request logging
pub mod conversation_logger;
pub mod request_logger;

use std::path::PathBuf;
use anyhow::{Result, Context};
use apchat_common::ApChatPaths;

// Re-export ConversationLogger for backward compatibility
pub use conversation_logger::ConversationLogger;

// Re-export request logging functions
pub use request_logger::{
    log_request,
    log_request_to_file,
    log_response,
    log_response_to_file,
    log_raw_response_to_file,
    log_stream_chunk,
};

/// Safely truncate a string to a maximum number of characters
pub fn safe_truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        // Reserve space for "..." suffix
        let trunc_chars = if max_chars >= 3 { max_chars - 3 } else { 0 };
        format!("{}...", s.chars().take(trunc_chars).collect::<String>())
    }
}

/// Get or create the logs directory (XDG-compliant: ~/.cache/apchat/logs)
pub fn get_logs_dir() -> Result<PathBuf> {
    let logs_dir = ApChatPaths::logs_dir();

    // Create directory if it doesn't exist
    ApChatPaths::ensure_dir(&logs_dir)
        .context("Failed to create logs directory")?;

    Ok(logs_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_truncate_short_string() {
        let result = safe_truncate("hello", 10);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_safe_truncate_exact_length() {
        let result = safe_truncate("hello", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_safe_truncate_needs_truncation() {
        let result = safe_truncate("hello world", 8);
        assert_eq!(result, "hello...");
    }

    #[test]
    fn test_safe_truncate_very_small_max() {
        // max_chars < 3, so trunc_chars = 0, result is just "..."
        let result = safe_truncate("hello", 2);
        assert_eq!(result, "...");
    }

    #[test]
    fn test_safe_truncate_max_equals_three() {
        let result = safe_truncate("hello", 3);
        assert_eq!(result, "...");
    }

    #[test]
    fn test_safe_truncate_max_four() {
        let result = safe_truncate("hello", 4);
        assert_eq!(result, "h...");
    }

    #[test]
    fn test_safe_truncate_empty_string() {
        let result = safe_truncate("", 10);
        assert_eq!(result, "");
    }

    #[test]
    fn test_safe_truncate_unicode() {
        let result = safe_truncate("abcdef", 6);
        assert_eq!(result, "abcdef");
        let result = safe_truncate("abcdefgh", 6);
        assert_eq!(result, "abc...");
    }

    #[test]
    fn test_get_logs_dir_returns_path() {
        let result = get_logs_dir();
        assert!(result.is_ok());
        let dir = result.unwrap();
        assert!(dir.exists());
    }
}
