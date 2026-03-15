use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use colored::Colorize;
use std::io::Write as IoWrite;
use apchat_vty::print_heart_red;

/// Tool for searching the web using a SearXNG instance
pub struct SearxngSearchTool {
    base_url: String,
}

impl SearxngSearchTool {
    pub fn new(base_url: String) -> Self {
        // Strip trailing slash for consistency
        let base_url = base_url.trim_end_matches('/').to_string();
        Self { base_url }
    }
}

#[async_trait]
impl Tool for SearxngSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web using a SearXNG metasearch engine instance. Returns a list of search results with titles, URLs, and snippets. Use this to find information on the internet. WARNING: ALWAYS include a 30 seconds sleep after using this tool to avoid getting it banned and broken."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("query", "string", "The search query string", required),
            param!("categories", "string", "Comma-separated search categories (e.g., 'general', 'images', 'news', 'science', 'it')", optional),
            param!("engines", "string", "Comma-separated list of search engines to use (e.g., 'google,duckduckgo,brave')", optional),
            param!("language", "string", "Search language code (e.g., 'en', 'de', 'fr'). Default: 'auto'", optional),
            param!("time_range", "string", "Time range filter: 'day', 'week', 'month', 'year'", optional),
            param!("max_results", "number", "Maximum number of results to return (default: 10, max: 50)", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let query = match params.get_required::<String>("query") {
            Ok(q) => q,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        if query.trim().is_empty() {
            return ToolResult::error("Search query cannot be empty".to_string());
        }

        let categories = params.get_optional::<String>("categories")
            .ok()
            .flatten();

        let engines = params.get_optional::<String>("engines")
            .ok()
            .flatten();

        let language = params.get_optional::<String>("language")
            .ok()
            .flatten();

        let time_range = params.get_optional::<String>("time_range")
            .ok()
            .flatten();

        let max_results = params.get_optional::<u64>("max_results")
            .ok()
            .flatten()
            .unwrap_or(10)
            .min(50) as usize;

        // Build the search URL with proper parameter encoding
        let mut search_url = match reqwest::Url::parse(&format!("{}/search", self.base_url)) {
            Ok(u) => u,
            Err(e) => return ToolResult::error(format!("Invalid SearXNG base URL: {}", e)),
        };

        {
            let mut params = search_url.query_pairs_mut();
            params.append_pair("q", &query);
            params.append_pair("format", "json");
            if let Some(ref cats) = categories {
                params.append_pair("categories", cats);
            }
            if let Some(ref eng) = engines {
                params.append_pair("engines", eng);
            }
            if let Some(ref lang) = language {
                params.append_pair("language", lang);
            }
            if let Some(ref tr) = time_range {
                if matches!(tr.as_str(), "day" | "week" | "month" | "year") {
                    params.append_pair("time_range", tr);
                } else {
                    return ToolResult::error(format!(
                        "Invalid time_range '{}'. Must be 'day', 'week', 'month', or 'year'", tr
                    ));
                }
            }
        }

        let url = search_url.as_str();

        // Check permission
        print_heart_red(&format!("{} {} ", "Web Search:".yellow(), query.cyan()), false);
        std::io::stdout().flush().ok();

        let approved = if context.non_interactive {
            true
        } else {
            match context.check_permission_async(
                apchat_policy::ActionType::NetworkRequest,
                &url,
                "Search? (y/N):"
            ).await {
                Ok((approved, _)) => approved,
                Err(e) => return ToolResult::error(format!("Permission check failed: {}", e)),
            }
        };

        if !approved {
            return ToolResult::error("Search cancelled by user or policy".to_string());
        }

        print_heart_red(&format!("{} {}", "Searching:".green(), query.cyan()), true);

        // Execute search request
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
        {
            Ok(client) => client,
            Err(e) => return ToolResult::error(format!("Failed to create HTTP client: {}", e)),
        };

        let response = match client.get(url).send().await {
            Ok(response) => response,
            Err(e) => {
                if e.is_timeout() {
                    return ToolResult::error(format!("Search request timed out after 30 seconds"));
                } else if e.is_connect() {
                    return ToolResult::error(format!("Failed to connect to SearXNG at {}: {}", self.base_url, e));
                } else {
                    return ToolResult::error(format!("Search request failed: {}", e));
                }
            }
        };

        let status = response.status();
        if !status.is_success() {
            return ToolResult::error(format!(
                "SearXNG returned HTTP {} {}\nURL: {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown"),
                self.base_url,
            ));
        }

        let body = match response.text().await {
            Ok(body) => body,
            Err(e) => return ToolResult::error(format!("Failed to read response: {}", e)),
        };

        // Parse the JSON response
        let json: serde_json::Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(e) => return ToolResult::error(format!("Failed to parse SearXNG response: {}", e)),
        };

        // Extract results
        let results = match json.get("results").and_then(|r| r.as_array()) {
            Some(results) => results,
            None => return ToolResult::success("No results found.".to_string()),
        };

        if results.is_empty() {
            return ToolResult::success("No results found.".to_string());
        }

        // Format results
        let mut output = format!("Search results for: {}\n\n", query);
        for (i, result) in results.iter().take(max_results).enumerate() {
            let title = result.get("title").and_then(|v| v.as_str()).unwrap_or("(no title)");
            let url = result.get("url").and_then(|v| v.as_str()).unwrap_or("(no url)");
            let content = result.get("content").and_then(|v| v.as_str()).unwrap_or("");

            output.push_str(&format!("{}. {}\n", i + 1, title));
            output.push_str(&format!("   URL: {}\n", url));
            if !content.is_empty() {
                output.push_str(&format!("   {}\n", content));
            }
            output.push('\n');
        }

        let total = results.len();
        let shown = total.min(max_results);
        output.push_str(&format!("Showing {} of {} results.", shown, total));

        // Wait 30-60 seconds to avoid hammering search engines
        // Using a simple pseudo-random based on current timestamp
        let wait_seconds = 30 + (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() % 31);
        print_heart_red(&format!("\n⏳ Waiting {} seconds before returning results (to avoid search engine rate limiting)...", wait_seconds), true);
        tokio::time::sleep(std::time::Duration::from_secs(wait_seconds)).await;

        ToolResult::success(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_searxng_tool_name() {
        let tool = SearxngSearchTool::new("http://example.com".to_string());
        assert_eq!(tool.name(), "web_search");
    }

    #[test]
    fn test_searxng_tool_parameters() {
        let tool = SearxngSearchTool::new("http://example.com".to_string());
        let params = tool.parameters();
        assert!(params.contains_key("query"));
        assert!(params.get("query").unwrap().required);
        assert!(params.contains_key("categories"));
        assert!(!params.get("categories").unwrap().required);
        assert!(params.contains_key("max_results"));
    }

    #[test]
    fn test_base_url_trailing_slash_stripped() {
        let tool = SearxngSearchTool::new("http://example.com/".to_string());
        assert_eq!(tool.base_url, "http://example.com");

        let tool = SearxngSearchTool::new("http://example.com///".to_string());
        assert_eq!(tool.base_url, "http://example.com");
    }
}
