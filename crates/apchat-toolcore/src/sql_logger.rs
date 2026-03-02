//! SQL-based logging for debugging tool argument parsing
//!
//! Captures HTTP requests/responses and parsing results to SQLite database.

use sqlx::{SqlitePool, Row, FromRow};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::Mutex;
use lazy_static::lazy_static;

/// Global SQL logger instance (lazy-initialized)
lazy_static! {
    static ref SQL_LOGGER: Arc<Mutex<Option<SqlLogger>>> = Arc::new(Mutex::new(None));
}

/// Initialize the SQL logger with a database path
pub async fn init_sql_logger(db_path: &str) -> Result<(), sqlx::Error> {
    // Create parent directory if it doesn't exist
    let path = std::path::Path::new(db_path);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    
    // Create the database file if it doesn't exist
    if !path.exists() {
        std::fs::File::create(path)?;
    }
    
    let pool = SqlitePool::connect(db_path).await?;
    
    // Create the database schema
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tool_parse_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            session_id TEXT,
            tool_name TEXT,
            http_request_body TEXT,
            http_response_body TEXT,
            parsed_arguments TEXT,
            parse_error TEXT,
            parse_success INTEGER NOT NULL,
            llm_provider TEXT,
            model_name TEXT,
            raw_llm_output TEXT
        )
        "#
    )
    .execute(&pool)
    .await?;

    // Create HTTP logs table (OpenTrace-compatible schema for calls)
    // Schema matches https://github.com/jmamda/OpenTrace exactly
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS calls (
            id                  TEXT PRIMARY KEY,
            timestamp           TEXT NOT NULL,
            provider            TEXT NOT NULL DEFAULT 'unknown',
            model               TEXT NOT NULL DEFAULT 'unknown',
            endpoint            TEXT NOT NULL DEFAULT '/v1/chat/completions',
            status_code         INTEGER NOT NULL DEFAULT 200,
            latency_ms          INTEGER NOT NULL DEFAULT 0,
            ttft_ms             INTEGER,
            input_tokens        INTEGER,
            output_tokens       INTEGER,
            cost_usd            REAL,
            request_body        TEXT,
            response_body       TEXT,
            error               TEXT,
            provider_request_id TEXT,
            trace_id            TEXT,
            parent_id           TEXT,
            prompt_hash         TEXT,
            tags                TEXT,
            agent_name          TEXT,
            workflow_id         TEXT,
            span_name           TEXT
        )
        "#
    )
    .execute(&pool)
    .await?;

    // Create indexes for common queries (matching OpenTrace)
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_calls_timestamp ON calls(timestamp);
        CREATE INDEX IF NOT EXISTS idx_calls_model ON calls(model);
        CREATE INDEX IF NOT EXISTS idx_calls_provider ON calls(provider);
        CREATE INDEX IF NOT EXISTS idx_calls_status ON calls(status_code);
        CREATE INDEX IF NOT EXISTS idx_calls_tags ON calls(tags);
        CREATE INDEX IF NOT EXISTS idx_calls_agent ON calls(agent_name);
        CREATE INDEX IF NOT EXISTS idx_calls_workflow ON calls(workflow_id);
        "#
    )
    .execute(&pool)
    .await?;

    let logger = SqlLogger {
        pool,
    };

    *SQL_LOGGER.lock().await = Some(logger);
    Ok(())
}

/// SQL Logger that captures tool parsing data
pub struct SqlLogger {
    pool: SqlitePool,
}

impl SqlLogger {
    /// Log a tool parsing attempt (success or failure)
    pub async fn log_tool_parse(
        &self,
        session_id: Option<String>,
        tool_name: Option<String>,
        http_request_body: Option<String>,
        http_response_body: Option<String>,
        parsed_arguments: Option<String>,
        parse_error: Option<String>,
        parse_success: bool,
        llm_provider: Option<String>,
        model_name: Option<String>,
        raw_llm_output: Option<String>,
    ) -> Result<u64, sqlx::Error> {
        let timestamp = Utc::now().to_rfc3339();

        let row = sqlx::query(
            r#"
            INSERT INTO tool_parse_logs 
            (timestamp, session_id, tool_name, http_request_body, http_response_body, 
             parsed_arguments, parse_error, parse_success, llm_provider, model_name, raw_llm_output)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&timestamp)
        .bind(session_id)
        .bind(tool_name)
        .bind(http_request_body)
        .bind(http_response_body)
        .bind(parsed_arguments)
        .bind(parse_error)
        .bind(parse_success as i32)
        .bind(llm_provider)
        .bind(model_name)
        .bind(raw_llm_output)
        .execute(&self.pool)
        .await?;

        Ok(row.last_insert_rowid() as u64)
    }

    /// Get recent failed parses
    pub async fn get_recent_failures(&self, limit: u32) -> Result<Vec<ToolParseLog>, sqlx::Error> {
        let rows = sqlx::query_as::<_, ToolParseLog>(
            r#"
            SELECT * FROM tool_parse_logs 
            WHERE parse_success = 0 
            ORDER BY timestamp DESC 
            LIMIT ?
            "#
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Get recent successes
    pub async fn get_recent_successes(&self, limit: u32) -> Result<Vec<ToolParseLog>, sqlx::Error> {
        let rows = sqlx::query_as::<_, ToolParseLog>(
            r#"
            SELECT * FROM tool_parse_logs 
            WHERE parse_success = 1 
            ORDER BY timestamp DESC 
            LIMIT ?
            "#
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Get all logs within a time range
    pub async fn get_logs_by_time_range(
        &self,
        start_time: &str,
        end_time: &str,
    ) -> Result<Vec<ToolParseLog>, sqlx::Error> {
        let rows = sqlx::query_as::<_, ToolParseLog>(
            r#"
            SELECT * FROM tool_parse_logs 
            WHERE timestamp BETWEEN ? AND ?
            ORDER BY timestamp DESC
            "#
        )
        .bind(start_time)
        .bind(end_time)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Search logs by tool name
    pub async fn search_by_tool(&self, tool_name: &str) -> Result<Vec<ToolParseLog>, sqlx::Error> {
        let rows = sqlx::query_as::<_, ToolParseLog>(
            r#"
            SELECT * FROM tool_parse_logs 
            WHERE tool_name LIKE ?
            ORDER BY timestamp DESC
            "#
        )
        .bind(format!("%{}%", tool_name))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Get statistics
    pub async fn get_statistics(&self) -> Result<(u64, u64), sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total,
                SUM(CASE WHEN parse_success = 1 THEN 1 ELSE 0 END) as successes
            FROM tool_parse_logs
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        let total: i64 = row.get("total");
        let successes: Option<i64> = row.get("successes");

        Ok((total as u64, successes.unwrap_or(0) as u64))
    }

    /// Clear all logs
    pub async fn clear_logs(&self) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM tool_parse_logs")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM calls")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Log an HTTP request/response (OpenTrace-compatible)
    pub async fn log_http(
        &self,
        id: String,
        timestamp: &str,
        provider: &str,
        model: &str,
        endpoint: &str,
        status_code: u16,
        latency_ms: u64,
        ttft_ms: Option<u64>,
        input_tokens: Option<i64>,
        output_tokens: Option<i64>,
        cost_usd: Option<f64>,
        request_body: Option<String>,
        response_body: Option<String>,
        error: Option<String>,
        provider_request_id: Option<String>,
        trace_id: Option<String>,
        parent_id: Option<String>,
        prompt_hash: Option<String>,
        tags: Option<String>,
        agent_name: Option<String>,
        workflow_id: Option<String>,
        span_name: Option<String>,
    ) -> Result<u64, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT OR REPLACE INTO calls 
            (id, timestamp, provider, model, endpoint, status_code, latency_ms, ttft_ms,
             input_tokens, output_tokens, cost_usd, request_body, response_body, error,
             provider_request_id, trace_id, parent_id, prompt_hash, tags, agent_name,
             workflow_id, span_name)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&id)
        .bind(timestamp)
        .bind(provider)
        .bind(model)
        .bind(endpoint)
        .bind(status_code as i64)
        .bind(latency_ms as i64)
        .bind(ttft_ms.map(|v| v as i64))
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(cost_usd)
        .bind(request_body)
        .bind(response_body)
        .bind(error)
        .bind(provider_request_id)
        .bind(trace_id)
        .bind(parent_id)
        .bind(prompt_hash)
        .bind(tags)
        .bind(agent_name)
        .bind(workflow_id)
        .bind(span_name)
        .execute(&self.pool)
        .await?;

        Ok(row.rows_affected())
    }

    /// Get recent HTTP logs (OpenTrace-compatible)
    pub async fn get_recent_calls(&self, limit: u32) -> Result<Vec<CallLog>, sqlx::Error> {
        let rows = sqlx::query_as::<_, CallLog>(
            r#"
            SELECT * FROM calls 
            ORDER BY timestamp DESC 
            LIMIT ?
            "#
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Get calls by status code
    pub async fn get_calls_by_status(&self, status: u16) -> Result<Vec<CallLog>, sqlx::Error> {
        let rows = sqlx::query_as::<_, CallLog>(
            r#"
            SELECT * FROM calls 
            WHERE status_code = ?
            ORDER BY timestamp DESC
            "#
        )
        .bind(status as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Get calls by model
    pub async fn get_calls_by_model(&self, model: &str) -> Result<Vec<CallLog>, sqlx::Error> {
        let rows = sqlx::query_as::<_, CallLog>(
            r#"
            SELECT * FROM calls 
            WHERE model LIKE ?
            ORDER BY timestamp DESC
            "#
        )
        .bind(format!("%{}%", model))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Get calls by agent
    pub async fn get_calls_by_agent(&self, agent: &str) -> Result<Vec<CallLog>, sqlx::Error> {
        let rows = sqlx::query_as::<_, CallLog>(
            r#"
            SELECT * FROM calls 
            WHERE agent_name LIKE ?
            ORDER BY timestamp DESC
            "#
        )
        .bind(format!("%{}%", agent))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Get statistics (OpenTrace-compatible)
    pub async fn get_call_statistics(&self) -> Result<(i64, i64, i64, f64, f64, i64), sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_calls,
                COALESCE(SUM(input_tokens), 0) as total_input,
                COALESCE(SUM(output_tokens), 0) as total_output,
                COALESCE(SUM(cost_usd), 0.0) as total_cost,
                COALESCE(AVG(latency_ms), 0.0) as avg_latency,
                COALESCE(SUM(CASE WHEN status_code >= 400 THEN 1 ELSE 0 END), 0) as error_count
            FROM calls
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        let total_calls: i64 = row.get("total_calls");
        let total_input: i64 = row.get("total_input");
        let total_output: i64 = row.get("total_output");
        let total_cost: f64 = row.get("total_cost");
        let avg_latency: f64 = row.get("avg_latency");
        let error_count: i64 = row.get("error_count");

        Ok((total_calls, total_input, total_output, total_cost, avg_latency, error_count))
    }
}

/// Represents a tool parse log entry
#[derive(Debug, Clone)]
pub struct ToolParseLog {
    pub id: i64,
    pub timestamp: String,
    pub session_id: Option<String>,
    pub tool_name: Option<String>,
    pub http_request_body: Option<String>,
    pub http_response_body: Option<String>,
    pub parsed_arguments: Option<String>,
    pub parse_error: Option<String>,
    pub parse_success: i64,
    pub llm_provider: Option<String>,
    pub model_name: Option<String>,
    pub raw_llm_output: Option<String>,
}

impl FromRow<'_, sqlx::sqlite::SqliteRow> for ToolParseLog {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(ToolParseLog {
            id: row.get("id"),
            timestamp: row.get("timestamp"),
            session_id: row.try_get("session_id").ok(),
            tool_name: row.try_get("tool_name").ok(),
            http_request_body: row.try_get("http_request_body").ok(),
            http_response_body: row.try_get("http_response_body").ok(),
            parsed_arguments: row.try_get("parsed_arguments").ok(),
            parse_error: row.try_get("parse_error").ok(),
            parse_success: row.get("parse_success"),
            llm_provider: row.try_get("llm_provider").ok(),
            model_name: row.try_get("model_name").ok(),
            raw_llm_output: row.try_get("raw_llm_output").ok(),
        })
    }
}

/// Represents a call log entry (OpenTrace-compatible)
#[derive(Debug, Clone)]
pub struct CallLog {
    pub id: String,  // UUID
    pub timestamp: String,
    pub provider: String,
    pub model: String,
    pub endpoint: String,
    pub status_code: i64,
    pub latency_ms: i64,
    pub ttft_ms: Option<i64>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cost_usd: Option<f64>,
    pub request_body: Option<String>,
    pub response_body: Option<String>,
    pub error: Option<String>,
    pub provider_request_id: Option<String>,
    pub trace_id: Option<String>,
    pub parent_id: Option<String>,
    pub prompt_hash: Option<String>,
    pub tags: Option<String>,
    pub agent_name: Option<String>,
    pub workflow_id: Option<String>,
    pub span_name: Option<String>,
}

impl FromRow<'_, sqlx::sqlite::SqliteRow> for CallLog {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(CallLog {
            id: row.get("id"),
            timestamp: row.get("timestamp"),
            provider: row.get("provider"),
            model: row.get("model"),
            endpoint: row.get("endpoint"),
            status_code: row.get("status_code"),
            latency_ms: row.get("latency_ms"),
            ttft_ms: row.try_get("ttft_ms").ok(),
            input_tokens: row.try_get("input_tokens").ok(),
            output_tokens: row.try_get("output_tokens").ok(),
            cost_usd: row.try_get("cost_usd").ok(),
            request_body: row.try_get("request_body").ok(),
            response_body: row.try_get("response_body").ok(),
            error: row.try_get("error").ok(),
            provider_request_id: row.try_get("provider_request_id").ok(),
            trace_id: row.try_get("trace_id").ok(),
            parent_id: row.try_get("parent_id").ok(),
            prompt_hash: row.try_get("prompt_hash").ok(),
            tags: row.try_get("tags").ok(),
            agent_name: row.try_get("agent_name").ok(),
            workflow_id: row.try_get("workflow_id").ok(),
            span_name: row.try_get("span_name").ok(),
        })
    }
}

/// Convenience function to log HTTP request/response (thread-safe, OpenTrace-compatible)
pub async fn log_http(
    id: String,
    timestamp: &str,
    provider: &str,
    model: &str,
    endpoint: &str,
    status_code: u16,
    latency_ms: u64,
    ttft_ms: Option<u64>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cost_usd: Option<f64>,
    request_body: Option<String>,
    response_body: Option<String>,
    error: Option<String>,
    provider_request_id: Option<String>,
    trace_id: Option<String>,
    parent_id: Option<String>,
    prompt_hash: Option<String>,
    tags: Option<String>,
    agent_name: Option<String>,
    workflow_id: Option<String>,
    span_name: Option<String>,
) -> Result<u64, sqlx::Error> {
    let logger_guard = SQL_LOGGER.lock().await;
    if let Some(logger) = logger_guard.as_ref() {
        logger.log_http(
            id,
            timestamp,
            provider,
            model,
            endpoint,
            status_code,
            latency_ms,
            ttft_ms,
            input_tokens,
            output_tokens,
            cost_usd,
            request_body,
            response_body,
            error,
            provider_request_id,
            trace_id,
            parent_id,
            prompt_hash,
            tags,
            agent_name,
            workflow_id,
            span_name,
        )
        .await
    } else {
        // Logger not initialized, silently ignore
        Ok(0)
    }
}

/// Convenience function to get recent calls
pub async fn get_recent_calls(limit: u32) -> Result<Vec<CallLog>, sqlx::Error> {
    let logger_guard = SQL_LOGGER.lock().await;
    if let Some(logger) = logger_guard.as_ref() {
        logger.get_recent_calls(limit).await
    } else {
        Ok(Vec::new())
    }
}

/// Convenience function to get calls by status code
pub async fn get_calls_by_status(status: u16) -> Result<Vec<CallLog>, sqlx::Error> {
    let logger_guard = SQL_LOGGER.lock().await;
    if let Some(logger) = logger_guard.as_ref() {
        logger.get_calls_by_status(status).await
    } else {
        Ok(Vec::new())
    }
}

/// Convenience function to get calls by model
pub async fn get_calls_by_model(model: &str) -> Result<Vec<CallLog>, sqlx::Error> {
    let logger_guard = SQL_LOGGER.lock().await;
    if let Some(logger) = logger_guard.as_ref() {
        logger.get_calls_by_model(model).await
    } else {
        Ok(Vec::new())
    }
}

/// Convenience function to get calls by agent
pub async fn get_calls_by_agent(agent: &str) -> Result<Vec<CallLog>, sqlx::Error> {
    let logger_guard = SQL_LOGGER.lock().await;
    if let Some(logger) = logger_guard.as_ref() {
        logger.get_calls_by_agent(agent).await
    } else {
        Ok(Vec::new())
    }
}

/// Convenience function to get call statistics
pub async fn get_call_statistics() -> Result<(i64, i64, i64, f64, f64, i64), sqlx::Error> {
    let logger_guard = SQL_LOGGER.lock().await;
    if let Some(logger) = logger_guard.as_ref() {
        logger.get_call_statistics().await
    } else {
        Ok((0, 0, 0, 0.0, 0.0, 0))
    }
}

/// Convenience function to log tool parsing (thread-safe)
pub async fn log_tool_parse(
    session_id: Option<String>,
    tool_name: Option<String>,
    http_request_body: Option<String>,
    http_response_body: Option<String>,
    parsed_arguments: Option<String>,
    parse_error: Option<String>,
    parse_success: bool,
    llm_provider: Option<String>,
    model_name: Option<String>,
    raw_llm_output: Option<String>,
) -> Result<u64, sqlx::Error> {
    let logger_guard = SQL_LOGGER.lock().await;
    if let Some(logger) = logger_guard.as_ref() {
        logger.log_tool_parse(
            session_id,
            tool_name,
            http_request_body,
            http_response_body,
            parsed_arguments,
            parse_error,
            parse_success,
            llm_provider,
            model_name,
            raw_llm_output,
        )
        .await
    } else {
        // Logger not initialized, silently ignore
        Ok(0)
    }
}

/// Convenience function to get recent failures
pub async fn get_recent_failures(limit: u32) -> Result<Vec<ToolParseLog>, sqlx::Error> {
    let logger_guard = SQL_LOGGER.lock().await;
    if let Some(logger) = logger_guard.as_ref() {
        logger.get_recent_failures(limit).await
    } else {
        Ok(Vec::new())
    }
}

/// Convenience function to get statistics
pub async fn get_statistics() -> Result<(u64, u64), sqlx::Error> {
    let logger_guard = SQL_LOGGER.lock().await;
    if let Some(logger) = logger_guard.as_ref() {
        logger.get_statistics().await
    } else {
        Ok((0, 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[serial_test::serial]
    async fn test_sql_logger_initialization() {
        // Use in-memory database for tests
        let db_path = "sqlite::memory:";
        let result = init_sql_logger(db_path).await;
        assert!(result.is_ok(), "Failed to initialize SQL logger: {:?}", result);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_log_tool_parse_success() {
        let db_path = "sqlite::memory:";
        init_sql_logger(db_path).await.unwrap();

        let log_id = log_tool_parse(
            Some("session_123".to_string()),
            Some("read_file".to_string()),
            Some(r#"{"messages": [...]}"#.to_string()),
            Some(r#"{...}"#.to_string()),
            Some(r#"{"file_path": "test.txt"}"#.to_string()),
            None,
            true,
            Some("groq".to_string()),
            Some("llama3-70b".to_string()),
            None,
        )
        .await
        .unwrap();

        assert!(log_id > 0);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_log_tool_parse_failure() {
        let db_path = "sqlite::memory:";
        init_sql_logger(db_path).await.unwrap();

        let log_id = log_tool_parse(
            Some("session_456".to_string()),
            Some("write_file".to_string()),
            Some(r#"{"messages": [...]}"#.to_string()),
            Some(r#"{...}"#.to_string()),
            None,
            Some("Failed to parse tool arguments: invalid JSON".to_string()),
            false,
            Some("anthropic".to_string()),
            Some("claude-3-5-sonnet".to_string()),
            Some("Raw LLM output here".to_string()),
        )
        .await
        .unwrap();

        assert!(log_id > 0);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_recent_failures() {
        let db_path = "sqlite::memory:";
        init_sql_logger(db_path).await.unwrap();

        // Log some failures
        log_tool_parse(None, None, None, None, None, Some("Error 1".to_string()), false, None, None, None).await.unwrap();
        log_tool_parse(None, None, None, None, None, Some("Error 2".to_string()), false, None, None, None).await.unwrap();

        let failures = get_recent_failures(10).await.unwrap();
        assert_eq!(failures.len(), 2);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_statistics() {
        let db_path = "sqlite::memory:";
        init_sql_logger(db_path).await.unwrap();

        // Log mixed results
        log_tool_parse(None, None, None, None, Some("args".to_string()), None, true, None, None, None).await.unwrap();
        log_tool_parse(None, None, None, None, Some("args".to_string()), None, true, None, None, None).await.unwrap();
        log_tool_parse(None, None, None, None, None, Some("error".to_string()), false, None, None, None).await.unwrap();

        let (total, successes) = get_statistics().await.unwrap();
        assert_eq!(total, 3);
        assert_eq!(successes, 2);
    }
}
