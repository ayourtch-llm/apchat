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
        Ok(())
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
