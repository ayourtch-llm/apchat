# fetch_url Tool Design

**Date:** 2026-01-03
**Status:** Approved

## Overview

Add a `fetch_url` tool to enable HTTP GET requests for documentation, API exploration, web scraping, and general web content retrieval.

## Use Cases

- Fetching API documentation and README files
- Making GET requests to REST APIs for data retrieval
- Downloading web content for research
- Accessing public data sources

## Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `url` | string | Yes | - | The URL to fetch |
| `format` | string | No | "auto" | Response format: "auto", "raw", "json", or "text" |
| `headers` | object | No | {} | Custom HTTP headers as key-value pairs |
| `timeout` | number | No | 30 | Request timeout in seconds (max 120) |
| `max_size` | number | No | 10485760 | Maximum response size in bytes (10MB default, 50MB max) |

### Format Options

- **`auto`**: Auto-detect content type and format appropriately
  - JSON → Pretty-print with 2-space indent
  - HTML → Extract text content (strip tags)
  - Text → Return as-is
  - Binary → Return error
- **`raw`**: Return raw response body unchanged
- **`json`**: Parse and pretty-print as JSON
- **`text`**: Return as plain text

## Security Measures

### URL Validation
- Block dangerous schemes: `file://`, `javascript:`, `data:`
- Block private IP ranges:
  - 127.0.0.0/8 (localhost)
  - 10.0.0.0/8 (private)
  - 172.16.0.0/12 (private)
  - 192.168.0.0/16 (private)
  - localhost hostname

### Permission System
- Use existing policy system via `ToolContext::check_permission()`
- Action type: `ActionType::NetworkRequest` (new variant)
- Prompt format: `Fetch URL? (y/N):`
- Show URL in prompt for user awareness

### Size & Timeout Protection
- Stream response body in chunks
- Enforce `max_size` during download
- Use tokio timeout wrapper
- Abort on limit exceeded

## Response Format

### Success Response
```
Success: HTTP 200
Content-Type: application/json
Size: 1,234 bytes

{
  "formatted": "response"
}
```

### Error Response
```
Error: HTTP 404 Not Found
URL: https://example.com/not-found
```

### Timeout Error
```
Error: Request timeout after 30 seconds
URL: https://slow-server.com
```

### Size Limit Error
```
Error: Response too large (exceeded 10,485,760 bytes)
URL: https://large-file.com
Partial response available: [first 10MB]
```

## Implementation Plan

### File Structure
- **New file**: `crates/apchat-tools/src/web.rs`
- **Tool struct**: `FetchUrlTool` implementing `Tool` trait
- **Update**: `crates/apchat-tools/src/lib.rs` to export module

### Dependencies
- `reqwest` (already available)
- `serde_json` (already available)
- `tokio` with timeout feature (already available)
- HTML text extraction: simple regex or `scraper` crate

### Error Handling

| Error Type | Message Format |
|------------|----------------|
| Network error | `Network error: {details}` |
| Invalid URL | `Invalid URL: {details}` |
| Blocked URL | `URL blocked for security: {reason}` |
| Permission denied | `Request cancelled by user` |
| Parse error | `Failed to parse as JSON: {error}` |
| Timeout | `Request timeout after {timeout} seconds` |
| Size exceeded | `Response too large (exceeded {max_size} bytes)` |

### Policy Integration

**Add to** `crates/apchat-policy/src/lib.rs`:
```rust
pub enum ActionType {
    // ... existing variants
    NetworkRequest,
}
```

**Tool permission check**:
```rust
context.check_permission(
    apchat_policy::ActionType::NetworkRequest,
    &url,
    &format!("Fetch URL? (y/N):")
)
```

### Registration

Register tool in `apchat-main/src/config/mod.rs` (or wherever tools are registered):
```rust
registry.register(Box::new(FetchUrlTool));
```

## Auto-Detection Logic

1. Check `Content-Type` header
   - `application/json` → Parse and pretty-print
   - `text/html` → Extract text, note HTML source
   - `text/*` → Return as-is
   - Binary types → Error
2. If Content-Type missing/ambiguous:
   - Attempt JSON parse
   - Fallback to text

## Testing Considerations

- Test with various content types (JSON, HTML, plain text)
- Test timeout handling
- Test size limit enforcement
- Test permission denial
- Test blocked URLs (localhost, private IPs)
- Test invalid URLs
- Test custom headers
- Test all format options

## Future Enhancements (Out of Scope)

- POST/PUT/DELETE methods
- Request body support
- Cookie handling
- Redirect following configuration
- Proxy support
- Certificate validation options
