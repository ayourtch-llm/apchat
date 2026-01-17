// Database connection and operations

use anyhow::{Context, Result};
use serde_json::Value;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{SqlitePool, Row};
use std::path::PathBuf;

use super::memory::Memory;

/// Get the default memory database path
/// Can be overridden by APCHAT_MEMORY_DB_PATH environment variable
pub fn get_memory_db_path() -> PathBuf {
    if let Ok(custom_path) = std::env::var("APCHAT_MEMORY_DB_PATH") {
        return PathBuf::from(custom_path);
    }
    
    let mut path = if let Some(mut base) = dirs::home_dir() {
        base.push(".okaychat");
        base.push("memory.sqlite");
        base
    } else {
        // Fallback to current directory
        PathBuf::from("memory.sqlite")
    };
    
    // Ensure directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    
    path
}

/// Create a database connection pool
pub async fn connect_pool(db_path: &PathBuf) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .busy_timeout(std::time::Duration::from_secs(5));
    
    SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(options)
        .await
        .context("Failed to connect to database")
}

/// Initialize the database (create tables if not exist)
pub async fn init_db(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS memories (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            conversation_id TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            metadata TEXT
        );
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create memories table")?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_memories_user ON memories(user_id);
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create user index")?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_memories_conversation ON memories(conversation_id);
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create conversation index")?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_memories_timestamp ON memories(timestamp);
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create timestamp index")?;

    Ok(())
}

/// Delete a memory from the database
/// Returns true if memory was deleted, false if not found
pub async fn delete_memory(pool: &SqlitePool, memory_id: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM memories WHERE id = ?")
        .bind(memory_id)
        .execute(pool)
        .await
        .context("Failed to delete memory")?;

    Ok(result.rows_affected() > 0)
}

/// List memories with filtering and pagination
pub async fn list_memories(
    pool: &SqlitePool,
    user_id: Option<&str>,
    conversation_id: Option<&str>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<Memory>> {
    let mut memos = Vec::new();

    // Build the query dynamically based on filters
    let mut query_builder = String::from(
        "SELECT id, user_id, conversation_id, content, timestamp, metadata FROM memories"
    );

    let mut conditions = Vec::new();
    let mut binds = Vec::new();

    if let Some(user) = user_id {
        conditions.push("user_id = ?");
        binds.push(user.to_string());
    }

    if let Some(conv) = conversation_id {
        conditions.push("conversation_id = ?");
        binds.push(conv.to_string());
    }

    if !conditions.is_empty() {
        query_builder.push_str(" WHERE ");
        query_builder.push_str(&conditions.join(" AND "));
    }

    query_builder.push_str(" ORDER BY timestamp DESC");

    if let Some(lim) = limit {
        query_builder.push_str(" LIMIT ?");
        binds.push(lim.to_string());
    }

    if let Some(off) = offset {
        if limit.is_some() {
            // Offset comes after limit in SQLite
            query_builder.push_str(" OFFSET ?");
            binds.push(off.to_string());
        } else {
            // SQLite doesn't support OFFSET without LIMIT
            query_builder.push_str(" LIMIT ? OFFSET ?");
            binds.push((usize::MAX).to_string()); // Use max limit
            binds.push(off.to_string());
        }
    }

    let query = sqlx::query(&query_builder);

    let mut query = binds.into_iter().fold(query, |q, bind| q.bind(bind));

    let rows = query.fetch_all(pool).await?;

    for row in rows {
        memos.push(Memory {
            id: row.get(0),
            user_id: row.get(1),
            conversation_id: row.get(2),
            content: row.get(3),
            timestamp: row.get(4),
            metadata: row.get(5),
        });
    }

    Ok(memos)
}
