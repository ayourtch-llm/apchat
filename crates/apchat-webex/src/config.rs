// Configuration module for Webex bot
// Handles loading of Webex access token from environment or config file

use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};

use apchat_common::ApChatPaths;

/// Load Webex access token from environment variable or ~/.okaychat/env file
///
/// Priority:
/// 1. WEBEX_APCHAT_SECRET environment variable
/// 2. WEBEX_APCHAT_SECRET in ~/.okaychat/env file
///
/// Returns error if token not found in either location
pub fn load_webex_secret() -> Result<String> {
    // First, try environment variable
    if let Ok(token) = std::env::var("WEBEX_APCHAT_SECRET") {
        if !token.trim().is_empty() {
            return Ok(token.trim().to_string());
        }
    }

    // Second, try ~/.okaychat/env file
    if let Ok(token) = load_from_okaychat_env() {
        return Ok(token);
    }

    // Not found in either location
    Err(anyhow::anyhow!(
        "WEBEX_APCHAT_SECRET not found. Set it via environment variable or add it to ~/.okaychat/env:\n\
         \n\
         Option 1 - Environment variable:\n\
         export WEBEX_APCHAT_SECRET=\"your_webex_bot_token\"\n\
         \n\
         Option 2 - Config file (~/.okaychat/env):\n\
         WEBEX_APCHAT_SECRET=your_webex_bot_token"
    ))
}

/// Load Webex secret from ~/.okaychat/env file
fn load_from_okaychat_env() -> Result<String> {
    let env_file = ApChatPaths::env_file();

    if !env_file.exists() {
        return Err(anyhow::anyhow!("~/.okaychat/env file not found"));
    }

    let file = File::open(&env_file)
        .with_context(|| format!("Failed to open {}", env_file.display()))?;

    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line.context("Failed to read line from env file")?;
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Look for WEBEX_APCHAT_SECRET=value
        if let Some(value) = parse_env_line(trimmed, "WEBEX_APCHAT_SECRET") {
            return Ok(value);
        }
    }

    Err(anyhow::anyhow!("WEBEX_APCHAT_SECRET not found in ~/.okaychat/env"))
}

/// Parse a line from env file in format KEY=value
/// Strips quotes from value if present
fn parse_env_line(line: &str, key: &str) -> Option<String> {
    let parts: Vec<&str> = line.splitn(2, '=').collect();

    if parts.len() != 2 {
        return None;
    }

    let (line_key, line_value) = (parts[0].trim(), parts[1].trim());

    if line_key != key {
        return None;
    }

    // Strip quotes if present
    let value = if (line_value.starts_with('"') && line_value.ends_with('"'))
        || (line_value.starts_with('\'') && line_value.ends_with('\''))
    {
        &line_value[1..line_value.len() - 1]
    } else {
        line_value
    };

    Some(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_env_line_simple() {
        let result = parse_env_line("WEBEX_APCHAT_SECRET=abc123", "WEBEX_APCHAT_SECRET");
        assert_eq!(result, Some("abc123".to_string()));
    }

    #[test]
    fn test_parse_env_line_with_double_quotes() {
        let result = parse_env_line("WEBEX_APCHAT_SECRET=\"abc123\"", "WEBEX_APCHAT_SECRET");
        assert_eq!(result, Some("abc123".to_string()));
    }

    #[test]
    fn test_parse_env_line_with_single_quotes() {
        let result = parse_env_line("WEBEX_APCHAT_SECRET='abc123'", "WEBEX_APCHAT_SECRET");
        assert_eq!(result, Some("abc123".to_string()));
    }

    #[test]
    fn test_parse_env_line_wrong_key() {
        let result = parse_env_line("OTHER_KEY=abc123", "WEBEX_APCHAT_SECRET");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_env_line_no_equals() {
        let result = parse_env_line("WEBEX_APCHAT_SECRET", "WEBEX_APCHAT_SECRET");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_env_line_with_spaces() {
        let result = parse_env_line("  WEBEX_APCHAT_SECRET = abc123  ", "WEBEX_APCHAT_SECRET");
        assert_eq!(result, Some("abc123".to_string()));
    }
}
