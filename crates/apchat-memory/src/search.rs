// Search functionality for memories

use anyhow::Result;
use sqlx::{SqlitePool, Row};

use super::memory::Memory;

/// Search memories by keyword
pub async fn search_memories(
    pool: &SqlitePool,
    query: &str,
    user_id: Option<&str>,
    conversation_id: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<Memory>> {
    let mut memos = Vec::new();
    
    let query_str = format!("%{}%", query);
    
    let query = if let (Some(user), Some(conv)) = (user_id, conversation_id) {
        sqlx::query(
            r#"
            SELECT id, user_id, conversation_id, content, timestamp, metadata 
            FROM memories 
            WHERE content LIKE ? 
            AND user_id = ? 
            AND conversation_id = ? 
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(&query_str)
        .bind(user)
        .bind(conv)
        .bind(limit.unwrap_or(50) as i64)
    } else if let Some(user) = user_id {
        sqlx::query(
            r#"
            SELECT id, user_id, conversation_id, content, timestamp, metadata 
            FROM memories 
            WHERE content LIKE ? 
            AND user_id = ? 
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(&query_str)
        .bind(user)
        .bind(limit.unwrap_or(50) as i64)
    } else if let Some(conv) = conversation_id {
        sqlx::query(
            r#"
            SELECT id, user_id, conversation_id, content, timestamp, metadata 
            FROM memories 
            WHERE content LIKE ? 
            AND conversation_id = ? 
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(&query_str)
        .bind(conv)
        .bind(limit.unwrap_or(50) as i64)
    } else {
        sqlx::query(
            r#"
            SELECT id, user_id, conversation_id, content, timestamp, metadata 
            FROM memories 
            WHERE content LIKE ? 
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(&query_str)
        .bind(limit.unwrap_or(50) as i64)
    };
    
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
