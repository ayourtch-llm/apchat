use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use apchat_models::{ChatRequest, ModelColor};
use crate::{safe_truncate, get_logs_dir};

/// Log HTTP request details for debugging (console output)
pub fn log_request(url: &str, request: &ChatRequest, api_key: &str, verbose: bool) {
    if !verbose {
        return;
    }

    eprintln!("\n{}", "═".repeat(80).bright_cyan());
    eprintln!("{}", "🔍 HTTP REQUEST DEBUG".bright_cyan().bold());
    eprintln!("{}", "═".repeat(80).bright_cyan());

    // Parse URL to show host and port
    if let Ok(parsed_url) = reqwest::Url::parse(url) {
        eprintln!("{}: {}", "URL".bright_yellow(), url);
        eprintln!("{}: {}", "Host".bright_yellow(), parsed_url.host_str().unwrap_or("unknown"));
        eprintln!("{}: {}", "Port".bright_yellow(), parsed_url.port().map(|p| p.to_string()).unwrap_or_else(||
            if parsed_url.scheme() == "https" { "443 (default)".to_string() } else { "80 (default)".to_string() }
        ));
        eprintln!("{}: {}", "Scheme".bright_yellow(), parsed_url.scheme());
    } else {
        eprintln!("{}: {}", "URL".bright_yellow(), url);
    }

    eprintln!("\n{}", "Headers:".bright_yellow());
    eprintln!("  Content-Type: application/json");
    eprintln!("  Authorization: Bearer {}***", &api_key.chars().take(10).collect::<String>());

    eprintln!("\n{}", "Request Body:".bright_yellow());
    match serde_json::to_string_pretty(&request) {
        Ok(json) => {
            // Truncate very long requests for readability
            if json.chars().count() > 5000 {
                eprintln!("{}", safe_truncate(&json, 5000));
                eprintln!("\n{}", format!("... (truncated, total {} bytes)", json.len()).bright_black());
            } else {
                eprintln!("{}", json);
            }
        }
        Err(e) => eprintln!("{}", format!("Error serializing request: {}", e).red()),
    }

    eprintln!("{}", "═".repeat(80).bright_cyan());
    eprintln!();
}

/// Log HTTP request to file for persistent debugging
pub fn log_request_to_file(url: &str, request: &ChatRequest, model: &ModelColor, api_key: &str) -> Result<()> {
    // Use shared logs directory from utility function
    let logs_dir = get_logs_dir()?;

    // Generate timestamp for filename
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Create filename with timestamp and model name
    let model_name = model.as_str_default().replace('/', "-");
    let filename = format!("req-{}-{}.txt", timestamp, model_name);
    let file_path = logs_dir.join(filename.clone());

    // Build the log content
    let mut log_content = String::new();
    log_content.push_str(&format!("HTTP REQUEST LOG\n"));
    log_content.push_str(&format!("================\n\n"));
    log_content.push_str(&format!("Timestamp: {}\n", timestamp));
    log_content.push_str(&format!("Model: {}\n\n", model.as_str_default()));

    // Parse URL to show host and port
    if let Ok(parsed_url) = reqwest::Url::parse(url) {
        log_content.push_str(&format!("URL: {}\n", url));
        log_content.push_str(&format!("Host: {}\n", parsed_url.host_str().unwrap_or("unknown")));
        log_content.push_str(&format!("Port: {}\n",
            parsed_url.port().map(|p| p.to_string()).unwrap_or_else(||
                if parsed_url.scheme() == "https" { "443 (default)".to_string() } else { "80 (default)".to_string() }
            )
        ));
        log_content.push_str(&format!("Scheme: {}\n\n", parsed_url.scheme()));
    } else {
        log_content.push_str(&format!("URL: {}\n\n", url));
    }

    log_content.push_str("Headers:\n");
    log_content.push_str("  Content-Type: application/json\n");
    log_content.push_str(&format!("  Authorization: Bearer {}***\n\n", &api_key.chars().take(10).collect::<String>()));

    log_content.push_str("Request Body:\n");
    match serde_json::to_string_pretty(&request) {
        Ok(json) => {
            log_content.push_str(&json);
            log_content.push_str("\n");
        }
        Err(e) => {
            log_content.push_str(&format!("Error serializing request: {}\n", e));
        }
    }

    // Write to file
    fs::write(&file_path, log_content)
        .with_context(|| format!("Failed to write request log to {}", file_path.display()))?;

    // Print the filename to console
    // print_heart_yellow(&format!("{}", format!("📝 Request logged to: {}", filename).bright_blue()), true);

    Ok(())
}

/// Log HTTP response to file for persistent debugging
pub fn log_response_to_file(
    status: &reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
    body: &str,
    request_timestamp: u64,
    model: &ModelColor,
) -> Result<()> {
    // Use shared logs directory from utility function
    let logs_dir = get_logs_dir()?;

    // Create filename with timestamp and model name to match request file
    let model_name = model.as_str_default().replace('/', "-");
    let filename = format!("resp-{}-{}.txt", request_timestamp, model_name);
    let file_path = logs_dir.join(filename.clone());

    // Build the log content
    let mut log_content = String::new();
    log_content.push_str(&format!("HTTP RESPONSE LOG\n"));
    log_content.push_str(&format!("=================\n\n"));
    log_content.push_str(&format!("Timestamp: {}\n", request_timestamp));
    log_content.push_str(&format!("Model: {}\n\n", model.as_str_default()));

    // Log status information
    log_content.push_str(&format!("Status: {} {}\n\n",
        status.as_u16(),
        status.canonical_reason().unwrap_or("Unknown")
    ));

    // Log headers
    log_content.push_str("Headers:\n");
    for (name, value) in headers.iter() {
        if let Ok(val_str) = value.to_str() {
            log_content.push_str(&format!("  {}: {}\n", name.as_str(), val_str));
        }
    }

    log_content.push_str("\nResponse Body:\n");
    // Try to pretty-print JSON, fall back to raw text
    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(body) {
        match serde_json::to_string_pretty(&json_val) {
            Ok(pretty) => {
                log_content.push_str(&pretty);
                log_content.push_str("\n");
            }
            Err(_) => {
                log_content.push_str(body);
                log_content.push_str("\n");
            }
        }
    } else {
        // Not JSON, show raw
        log_content.push_str(body);
        log_content.push_str("\n");
    }

    // Add metadata at the end
    log_content.push_str(&format!("\n---\n"));
    log_content.push_str(&format!("Response Size: {} bytes\n", body.len()));
    log_content.push_str(&format!("Content-Type: {}\n",
        headers.get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown")
    ));

    // Write to file
    fs::write(&file_path, log_content)
        .with_context(|| format!("Failed to write response log to {}", file_path.display()))?;

    // Print the filename to console
    // print_heart_yellow(&format!("{}", format!("📄 Response logged to: {}", filename).bright_blue()), true);

    Ok(())
}

/// Log pure raw response to file without any transformation or massage
pub fn log_raw_response_to_file(
    raw_response: &str,
    request_timestamp: u64,
    model: &ModelColor,
) -> Result<()> {
    // Use shared logs directory from utility function
    let logs_dir = get_logs_dir()?;

    // Create filename for raw response with timestamp and model name
    let model_name = model.as_str_default().replace('/', "-");
    let filename = format!("resp-raw-{}-{}.txt", request_timestamp, model_name);
    let file_path = logs_dir.join(filename.clone());

    // Write the pure raw response without any modification
    fs::write(&file_path, raw_response)
        .with_context(|| format!("Failed to write raw response log to {}", file_path.display()))?;

    // Print the filename to console
    // print_heart_yellow(&format!("{}", format!("📄 Raw response logged to: {}", filename).bright_blue()), true);

    Ok(())
}

/// Log HTTP response details for debugging (console output)
pub fn log_response(status: &reqwest::StatusCode, headers: &reqwest::header::HeaderMap, body: &str, verbose: bool) {
    if !verbose {
        return;
    }

    eprintln!("\n{}", "═".repeat(80).bright_green());
    eprintln!("{}", "📥 HTTP RESPONSE DEBUG".bright_green().bold());
    eprintln!("{}", "═".repeat(80).bright_green());

    eprintln!("{}: {} {}",
        "Status".bright_yellow(),
        status.as_u16(),
        status.canonical_reason().unwrap_or("Unknown")
    );

    eprintln!("\n{}", "Headers:".bright_yellow());
    for (name, value) in headers.iter() {
        if let Ok(val_str) = value.to_str() {
            eprintln!("  {}: {}", name.as_str().bright_white(), val_str);
        }
    }

    eprintln!("\n{}", "Response Body:".bright_yellow());
    // Try to pretty-print JSON, fall back to raw text
    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(body) {
        match serde_json::to_string_pretty(&json_val) {
            Ok(pretty) => {
                if pretty.chars().count() > 5000 {
                    eprintln!("{}", safe_truncate(&pretty, 5000));
                    eprintln!("\n{}", format!("... (truncated, total {} bytes)", pretty.len()).bright_black());
                } else {
                    eprintln!("{}", pretty);
                }
            }
            Err(_) => eprintln!("{}", body),
        }
    } else {
        // Not JSON, show raw
        if body.chars().count() > 5000 {
            eprintln!("{}", safe_truncate(body, 5000));
            eprintln!("\n{}", format!("... (truncated, total {} bytes)", body.len()).bright_black());
        } else {
            eprintln!("{}", body);
        }
    }

    eprintln!("{}", "═".repeat(80).bright_green());
    eprintln!();
}

/// Log streaming chunk for debugging (console output)
pub fn log_stream_chunk(chunk_num: usize, data: &str, verbose: bool) {
    if !verbose {
        return;
    }

    eprintln!("{}", format!("📦 Stream Chunk #{}: {}", chunk_num,
        if data.chars().count() > 200 {
            format!("{}... ({} bytes)", safe_truncate(data, 200), data.len())
        } else {
            data.to_string()
        }
    ).bright_black());
}

#[cfg(test)]
mod tests {
    use super::*;
    use apchat_models::requests::ChatRequest;
    use apchat_models::types::{Message, ContentPart};

    fn make_test_request() -> ChatRequest {
        ChatRequest {
            model: "test-model".to_string(),
            stream: None,
            tool_choice: "auto".to_string(),
            tools: vec![],
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: vec![ContentPart::Text("Hello".to_string())],
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                    reasoning: None,
                },
            ],
        }
    }

    // --- log_request tests (console output, verbose gate) ---

    #[test]
    fn test_log_request_not_verbose_does_nothing() {
        // Should not panic when verbose=false
        let request = make_test_request();
        log_request("https://api.example.com/v1/chat", &request, "sk-test1234567890", false);
    }

    #[test]
    fn test_log_request_verbose_does_not_panic() {
        let request = make_test_request();
        log_request("https://api.example.com/v1/chat", &request, "sk-test1234567890", true);
    }

    #[test]
    fn test_log_request_with_invalid_url() {
        // Invalid URL should not panic, just print URL as-is
        let request = make_test_request();
        log_request("not-a-url", &request, "sk-test", true);
    }

    // --- log_response tests (console output, verbose gate) ---

    #[test]
    fn test_log_response_not_verbose_does_nothing() {
        let status = reqwest::StatusCode::OK;
        let headers = reqwest::header::HeaderMap::new();
        log_response(&status, &headers, "{}", false);
    }

    #[test]
    fn test_log_response_verbose_json_body() {
        let status = reqwest::StatusCode::OK;
        let headers = reqwest::header::HeaderMap::new();
        log_response(&status, &headers, "{\"key\": \"value\"}", true);
    }

    #[test]
    fn test_log_response_verbose_non_json_body() {
        let status = reqwest::StatusCode::INTERNAL_SERVER_ERROR;
        let headers = reqwest::header::HeaderMap::new();
        log_response(&status, &headers, "plain text error", true);
    }

    #[test]
    fn test_log_response_verbose_long_body_truncated() {
        let status = reqwest::StatusCode::OK;
        let headers = reqwest::header::HeaderMap::new();
        let long_body = "x".repeat(6000);
        log_response(&status, &headers, &long_body, true);
    }

    // --- log_stream_chunk tests ---

    #[test]
    fn test_log_stream_chunk_not_verbose() {
        log_stream_chunk(1, "some data", false);
    }

    #[test]
    fn test_log_stream_chunk_verbose_short() {
        log_stream_chunk(1, "short data", true);
    }

    #[test]
    fn test_log_stream_chunk_verbose_long() {
        let long = "a".repeat(300);
        log_stream_chunk(42, &long, true);
    }

    // --- log_request_to_file tests ---

    #[test]
    fn test_log_request_to_file_creates_file() {
        let request = make_test_request();
        let result = log_request_to_file(
            "https://api.example.com/v1/chat",
            &request,
            &ModelColor::BluModel,
            "sk-test1234567890",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_log_request_to_file_with_invalid_url() {
        let request = make_test_request();
        let result = log_request_to_file(
            "not-a-url",
            &request,
            &ModelColor::GrnModel,
            "sk-short",
        );
        // Should succeed even with invalid URL
        assert!(result.is_ok());
    }

    // --- log_response_to_file tests ---

    #[test]
    fn test_log_response_to_file_creates_file() {
        let status = reqwest::StatusCode::OK;
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());

        let result = log_response_to_file(
            &status,
            &headers,
            "{\"choices\": []}",
            1234567890,
            &ModelColor::BluModel,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_log_response_to_file_non_json_body() {
        let status = reqwest::StatusCode::BAD_REQUEST;
        let headers = reqwest::header::HeaderMap::new();

        let result = log_response_to_file(
            &status,
            &headers,
            "not json at all",
            9999999999,
            &ModelColor::RedModel,
        );
        assert!(result.is_ok());
    }

    // --- log_raw_response_to_file tests ---

    #[test]
    fn test_log_raw_response_to_file_creates_file() {
        let result = log_raw_response_to_file(
            "raw response data here",
            1234567890,
            &ModelColor::GrnModel,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_log_raw_response_to_file_empty_content() {
        let result = log_raw_response_to_file(
            "",
            1111111111,
            &ModelColor::BluModel,
        );
        assert!(result.is_ok());
    }
}
