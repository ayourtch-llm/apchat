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
