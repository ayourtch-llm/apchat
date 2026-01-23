use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use colored::Colorize;
use std::io::Write as IoWrite;
use apchat_vty::print_heart_red;

/// Maximum allowed response size (50MB)
const MAX_RESPONSE_SIZE: usize = 50 * 1024 * 1024;

/// Default response size limit (10MB)
const DEFAULT_MAX_SIZE: usize = 10 * 1024 * 1024;

/// Maximum allowed timeout (120 seconds)
const MAX_TIMEOUT: u64 = 120;

/// Default timeout (30 seconds)
const DEFAULT_TIMEOUT: u64 = 30;

/// Tool for fetching URLs via HTTP GET
pub struct FetchUrlTool;

#[async_trait]
impl Tool for FetchUrlTool {
    fn name(&self) -> &str {
        "fetch_url"
    }

    fn description(&self) -> &str {
        "Fetch content from a URL via HTTP GET request. Prefer 'markdown' format for HTML pages - it converts to clean, readable markdown. Use 'raw' only if you need unprocessed content, as it may include large amounts of JavaScript and CSS."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("url", "string", "The URL to fetch", required),
            param!("format", "string", "Response format: 'auto' (default), 'raw', 'json', or 'markdown'", optional),
            param!("headers", "object", "Custom HTTP headers as key-value pairs", optional),
            param!("timeout", "number", "Request timeout in seconds (default: 30, max: 120)", optional),
            param!("max_size", "number", "Maximum response size in bytes (default: 10MB, max: 50MB)", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        // Parse URL parameter
        let url = match params.get_required::<String>("url") {
            Ok(url) => url,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        // Parse optional format parameter
        let format = params.get_optional::<String>("format")
            .ok()
            .flatten()
            .unwrap_or_else(|| "auto".to_string());

        // Validate format
        if !matches!(format.as_str(), "auto" | "raw" | "json" | "markdown") {
            return ToolResult::error(format!("Invalid format '{}'. Must be 'auto', 'raw', 'json', or 'markdown'", format));
        }

        // Parse optional timeout parameter
        let timeout_secs = params.get_optional::<u64>("timeout")
            .ok()
            .flatten()
            .unwrap_or(DEFAULT_TIMEOUT);

        if timeout_secs > MAX_TIMEOUT {
            return ToolResult::error(format!("Timeout {} exceeds maximum of {} seconds", timeout_secs, MAX_TIMEOUT));
        }

        // Parse optional max_size parameter
        let max_size = params.get_optional::<usize>("max_size")
            .ok()
            .flatten()
            .unwrap_or(DEFAULT_MAX_SIZE);

        if max_size > MAX_RESPONSE_SIZE {
            return ToolResult::error(format!("max_size {} exceeds maximum of {} bytes", max_size, MAX_RESPONSE_SIZE));
        }

        // Parse optional headers parameter
        let custom_headers = params.get_optional::<HashMap<String, String>>("headers")
            .ok()
            .flatten()
            .unwrap_or_default();

        // Validate and sanitize URL
        if let Err(e) = validate_url(&url) {
            return ToolResult::error(e);
        }

        // Check permission using policy system
        print_heart_red(&format!("{} {} ", "Fetch URL:".yellow(), url.cyan()), false);
        std::io::stdout().flush().ok();

        // In non-interactive mode, skip confirmation (already approved via web UI)
        // In interactive mode, check permission using policy system
        let approved = if context.non_interactive {
            true
        } else {
            let (result, _) = match context.check_permission_async(
                apchat_policy::ActionType::NetworkRequest,
                &url,
                "Fetch? (y/N):"
            ).await {
                Ok((approved, reason)) => (approved, reason),
                Err(e) => return ToolResult::error(format!("Permission check failed: {}", e)),
            };
            result
        };

        if !approved {
            return ToolResult::error("Request cancelled by user or policy".to_string());
        }

        print_heart_red(&format!("{} {}", "Fetching:".green(), url.cyan()), true);

        // Build HTTP client with timeout
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
        {
            Ok(client) => client,
            Err(e) => return ToolResult::error(format!("Failed to create HTTP client: {}", e)),
        };

        // Build request with custom headers
        let mut request = client.get(&url);
        for (key, value) in custom_headers {
            request = request.header(&key, &value);
        }

        // Execute request
        let response = match request.send().await {
            Ok(response) => response,
            Err(e) => {
                if e.is_timeout() {
                    return ToolResult::error(format!("Request timeout after {} seconds\nURL: {}", timeout_secs, url));
                } else if e.is_connect() {
                    return ToolResult::error(format!("Connection error: {}\nURL: {}", e, url));
                } else {
                    return ToolResult::error(format!("Network error: {}\nURL: {}", e, url));
                }
            }
        };

        let status = response.status();
        let content_type = response.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        // Get response body with size limit
        let body_bytes = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => return ToolResult::error(format!("Failed to read response body: {}", e)),
        };

        if body_bytes.len() > max_size {
            return ToolResult::error(format!(
                "Response too large (exceeded {} bytes)\nURL: {}\nActual size: {} bytes",
                max_size, url, body_bytes.len()
            ));
        }

        let body = String::from_utf8_lossy(&body_bytes).to_string();

        // Format response based on format parameter
        let formatted_body = match format.as_str() {
            "raw" => body,
            "json" => format_as_json(&body, &content_type),
            "markdown" => format_as_markdown(&body, &content_type),
            "auto" => auto_format(&body, &content_type),
            _ => body, // Should not reach here due to validation above
        };

        // Build response
        let result = if status.is_success() {
            format!(
                "Success: HTTP {}\nContent-Type: {}\nSize: {} bytes\n\n{}",
                status.as_u16(),
                content_type,
                body_bytes.len(),
                formatted_body
            )
        } else {
            format!(
                "Error: HTTP {} {}\nURL: {}\nContent-Type: {}\n\n{}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown"),
                url,
                content_type,
                formatted_body
            )
        };

        ToolResult::success(result)
    }
}

/// Validate URL and check for security issues
fn validate_url(url: &str) -> Result<(), String> {
    // Parse URL
    let parsed = match reqwest::Url::parse(url) {
        Ok(u) => u,
        Err(e) => return Err(format!("Invalid URL: {}", e)),
    };

    // Check scheme
    let scheme = parsed.scheme();
    if !matches!(scheme, "http" | "https") {
        return Err(format!("URL blocked for security: unsupported scheme '{}' (only http/https allowed)", scheme));
    }

    // Check for dangerous schemes that might bypass scheme check
    let url_lower = url.to_lowercase();
    if url_lower.starts_with("file://")
        || url_lower.starts_with("javascript:")
        || url_lower.starts_with("data:") {
        return Err("URL blocked for security: dangerous scheme detected".to_string());
    }

    // Check for private IP addresses and localhost
    if let Some(host) = parsed.host_str() {
        if is_private_or_localhost(host) {
            return Err(format!("URL blocked for security: private IP or localhost not allowed ({})", host));
        }
    }

    Ok(())
}

/// Check if a host is a private IP address or localhost
fn is_private_or_localhost(host: &str) -> bool {
    // Check for localhost names
    if host == "localhost" || host == "127.0.0.1" || host == "::1" {
        return true;
    }

    // Try to parse as IP address
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        match ip {
            std::net::IpAddr::V4(ipv4) => {
                let octets = ipv4.octets();
                // Check private ranges
                // 10.0.0.0/8
                if octets[0] == 10 {
                    return true;
                }
                // 172.16.0.0/12
                if octets[0] == 172 && (octets[1] >= 16 && octets[1] <= 31) {
                    return true;
                }
                // 192.168.0.0/16
                if octets[0] == 192 && octets[1] == 168 {
                    return true;
                }
                // 127.0.0.0/8
                if octets[0] == 127 {
                    return true;
                }
            }
            std::net::IpAddr::V6(ipv6) => {
                // Check for ::1 (localhost)
                if ipv6.is_loopback() {
                    return true;
                }
                // Check for private IPv6 ranges (fc00::/7)
                let segments = ipv6.segments();
                if segments[0] >= 0xfc00 && segments[0] <= 0xfdff {
                    return true;
                }
            }
        }
    }

    false
}

/// Auto-detect format based on content-type and content
fn auto_format(body: &str, content_type: &str) -> String {
    if content_type.contains("application/json") || content_type.contains("json") {
        format_as_json(body, content_type)
    } else if content_type.contains("text/html") || content_type.contains("html") {
        // Convert HTML to markdown for better readability
        html2md::parse_html(body)
    } else if content_type.starts_with("text/") {
        body.to_string()
    } else if content_type.contains("image/") || content_type.contains("video/") || content_type.contains("audio/") {
        format!("Binary content not supported (Content-Type: {})\nUse format='raw' to get raw bytes", content_type)
    } else {
        // Try to parse as JSON, fallback to text
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(body) {
            match serde_json::to_string_pretty(&json_value) {
                Ok(pretty) => pretty,
                Err(_) => body.to_string(),
            }
        } else {
            body.to_string()
        }
    }
}

/// Format response as JSON (pretty-print)
fn format_as_json(body: &str, _content_type: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(body) {
        Ok(json_value) => {
            match serde_json::to_string_pretty(&json_value) {
                Ok(pretty) => pretty,
                Err(e) => format!("Failed to format JSON: {}\n\nRaw:\n{}", e, body),
            }
        }
        Err(e) => format!("Failed to parse as JSON: {}\n\nRaw:\n{}", e, body),
    }
}

/// Format response as markdown (convert HTML to markdown if detected)
fn format_as_markdown(body: &str, content_type: &str) -> String {
    if content_type.contains("html") || body.trim_start().starts_with("<!DOCTYPE") || body.trim_start().starts_with("<html") {
        // Convert HTML to markdown
        html2md::parse_html(body)
    } else if content_type.contains("json") {
        // JSON doesn't need markdown conversion, just return as-is
        format_as_json(body, content_type)
    } else {
        // Plain text, return as-is
        body.to_string()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url_valid() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://api.example.com/data").is_ok());
    }

    #[test]
    fn test_validate_url_invalid_scheme() {
        assert!(validate_url("file:///etc/passwd").is_err());
        assert!(validate_url("javascript:alert(1)").is_err());
        assert!(validate_url("data:text/html,<script>alert(1)</script>").is_err());
    }

    #[test]
    fn test_validate_url_localhost() {
        assert!(validate_url("http://localhost:8080").is_err());
        assert!(validate_url("http://127.0.0.1").is_err());
    }

    #[test]
    fn test_validate_url_private_ips() {
        assert!(validate_url("http://10.0.0.1").is_err());
        assert!(validate_url("http://172.16.0.1").is_err());
        assert!(validate_url("http://192.168.1.1").is_err());
    }

    #[test]
    fn test_is_private_or_localhost() {
        assert!(is_private_or_localhost("localhost"));
        assert!(is_private_or_localhost("127.0.0.1"));
        assert!(is_private_or_localhost("10.0.0.1"));
        assert!(is_private_or_localhost("172.16.0.1"));
        assert!(is_private_or_localhost("192.168.1.1"));
        assert!(!is_private_or_localhost("8.8.8.8"));
        assert!(!is_private_or_localhost("example.com"));
    }

    #[test]
    fn test_format_as_markdown() {
        let html = "<html><body><h1>Title</h1><p>This is a <strong>paragraph</strong> with <a href='http://example.com'>a link</a>.</p></body></html>";
        let markdown = format_as_markdown(html, "text/html");

        // Check that markdown conversion happened
        assert!(markdown.contains("# Title") || markdown.contains("Title"));
        assert!(markdown.contains("paragraph"));
        assert!(!markdown.contains("<h1>"));
        assert!(!markdown.contains("<p>"));
        assert!(!markdown.contains("<strong>"));
    }

    #[test]
    fn test_format_as_markdown_with_json() {
        let json = r#"{"key": "value"}"#;
        let result = format_as_markdown(json, "application/json");

        // Should format as JSON, not try to convert as HTML
        assert!(result.contains("key"));
        assert!(result.contains("value"));
    }

    #[test]
    fn test_format_as_markdown_with_plain_text() {
        let text = "Just plain text";
        let result = format_as_markdown(text, "text/plain");

        // Should return as-is
        assert_eq!(result, text);
    }
}
