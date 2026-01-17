// Memory tools implementation

use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use chrono::{Utc, DateTime};
use sqlx::Row;

use crate::memory::{Memory, connect_pool, init_db, get_memory_db_path, search_memories};

/// Tool for storing a new memory
pub struct StoreMemoryTool;

#[async_trait]
impl Tool for StoreMemoryTool {
    fn name(&self) -> &str {
        "store_memory"
    }

    fn description(&self) -> &str {
        "Store a new memory for a conversation. Memories can be retrieved later for context or search."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("user_id", "string", "ID of the user creating the memory", required),
            param!("conversation_id", "string", "ID of the conversation this memory belongs to", required),
            param!("content", "string", "The content/text to store in the memory", required),
            param!("metadata", "string", "Optional metadata as JSON string", optional, ""),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        // Validate and extract parameters
        let user_id = match params.get_required::<String>("user_id") {
            Ok(user_id) => user_id,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let conversation_id = match params.get_required::<String>("conversation_id") {
            Ok(conversation_id) => conversation_id,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let content = match params.get_required::<String>("content") {
            Ok(content) => content,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let metadata = params.get_optional::<String>("metadata")
            .unwrap_or(None);

        // Validate inputs
        if user_id.trim().is_empty() {
            return ToolResult::error("user_id cannot be empty".to_string());
        }

        if conversation_id.trim().is_empty() {
            return ToolResult::error("conversation_id cannot be empty".to_string());
        }

        if content.trim().is_empty() {
            return ToolResult::error("content cannot be empty".to_string());
        }

        if content.len() > 100000 {
            return ToolResult::error("content cannot exceed 100,000 characters".to_string());
        }

        // Get current timestamp
        let timestamp = Utc::now().timestamp();

        // Create the memory
        let memory = Memory::new(
            user_id,
            conversation_id,
            content,
            timestamp,
            metadata,
        );

        // Initialize database connection
        let db_path = get_memory_db_path();
        let pool = match connect_pool(&db_path).await {
            Ok(pool) => pool,
            Err(e) => return ToolResult::error(format!("Failed to connect to database: {}", e)),
        };

        // Initialize database (create tables if not exist)
        if let Err(e) = init_db(&pool).await {
            return ToolResult::error(format!("Failed to initialize database: {}", e));
        }

        // Store memory in database
        let result = sqlx::query(
            r#"
            INSERT INTO memories (
                id, user_id, conversation_id, content, timestamp, metadata
            ) VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&memory.id)
        .bind(&memory.user_id)
        .bind(&memory.conversation_id)
        .bind(&memory.content)
        .bind(memory.timestamp)
        .bind(memory.metadata)
        .execute(&pool)
        .await;

        match result {
            Ok(_) => {
                let response_json = serde_json::json!({
                    "message": "Memory stored successfully",
                    "memory_id": memory.id,
                    "timestamp": memory.timestamp,
                    "user_id": memory.user_id,
                    "conversation_id": memory.conversation_id,
                }).to_string();
                ToolResult::success(response_json)
            }
            Err(e) => ToolResult::error(format!("Failed to store memory: {}", e)),
        }
    }
}

/// Tool for querying memories
pub struct QueryMemoryTool;

#[async_trait]
impl Tool for QueryMemoryTool {
    fn name(&self) -> &str {
        "query_memory"
    }

    fn description(&self) -> &str {
        "Search and retrieve memories based on keywords, filters, and time ranges. Memories are returned sorted by timestamp (newest first)."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("user_id", "string", "ID of the user to filter memories by", required),
            param!("query", "string", "Search term to find in memory content", optional, ""),
            param!("limit", "integer", "Maximum number of memories to return (default: 50, max: 1000)", optional, "50"),
            param!("conversation_id", "string", "ID of the conversation to filter by", optional, ""),
            param!("after_timestamp", "integer", "Only return memories created after this Unix timestamp", optional, ""),
            param!("before_timestamp", "integer", "Only return memories created before this Unix timestamp", optional, ""),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        // Validate and extract required parameter
        let user_id = match params.get_required::<String>("user_id") {
            Ok(user_id) => user_id,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        // Extract optional parameters
        let query = params.get_optional::<String>("query").unwrap_or(None);
        let limit_param = params.get_optional::<i64>("limit").unwrap_or(None);
        let conversation_id = params.get_optional::<String>("conversation_id").unwrap_or(None);
        let after_timestamp = params.get_optional::<i64>("after_timestamp").unwrap_or(None);
        let before_timestamp = params.get_optional::<i64>("before_timestamp").unwrap_or(None);

        // Validate inputs
        if user_id.trim().is_empty() {
            return ToolResult::error("user_id cannot be empty".to_string());
        }

        // Validate limit
        let limit = match limit_param {
            Some(lim) => {
                if lim < 1 {
                    return ToolResult::error("limit must be at least 1".to_string());
                }
                if lim > 1000 {
                    return ToolResult::error("limit cannot exceed 1000".to_string());
                }
                lim as usize
            }
            None => 50, // Default limit
        };

        // Initialize database connection
        let db_path = get_memory_db_path();
        let pool = match connect_pool(&db_path).await {
            Ok(pool) => pool,
            Err(e) => return ToolResult::error(format!("Failed to connect to database: {}", e)),
        };

        // Initialize database (create tables if not exist)
        if let Err(e) = init_db(&pool).await {
            return ToolResult::error(format!("Failed to initialize database: {}", e));
        }

        // Perform the search
        let memories = match query {
            Some(search_query) => {
                if search_query.trim().is_empty() {
                    return ToolResult::error("query cannot be empty if provided".to_string());
                }
                search_memories(
                    &pool,
                    &search_query,
                    Some(&user_id),
                    conversation_id.as_deref(),
                    Some(limit),
                ).await
            }
            None => {
                // When no query is provided, return all memories for the user
                // Apply conversation and timestamp filters
                let query_str;
                let query;
                
                match (conversation_id.as_deref(), after_timestamp, before_timestamp) {
                    (Some(conv), Some(after), Some(before)) => {
                        query_str = "SELECT id, user_id, conversation_id, content, timestamp, metadata FROM memories WHERE user_id = ? AND conversation_id = ? AND timestamp > ? AND timestamp < ? ORDER BY timestamp DESC LIMIT ?";
                        query = sqlx::query(query_str)
                            .bind(&user_id)
                            .bind(conv)
                            .bind(after)
                            .bind(before)
                            .bind(limit as i64);
                    }
                    (Some(conv), Some(after), None) => {
                        query_str = "SELECT id, user_id, conversation_id, content, timestamp, metadata FROM memories WHERE user_id = ? AND conversation_id = ? AND timestamp > ? ORDER BY timestamp DESC LIMIT ?";
                        query = sqlx::query(query_str)
                            .bind(&user_id)
                            .bind(conv)
                            .bind(after)
                            .bind(limit as i64);
                    }
                    (Some(conv), None, Some(before)) => {
                        query_str = "SELECT id, user_id, conversation_id, content, timestamp, metadata FROM memories WHERE user_id = ? AND conversation_id = ? AND timestamp < ? ORDER BY timestamp DESC LIMIT ?";
                        query = sqlx::query(query_str)
                            .bind(&user_id)
                            .bind(conv)
                            .bind(before)
                            .bind(limit as i64);
                    }
                    (Some(conv), None, None) => {
                        query_str = "SELECT id, user_id, conversation_id, content, timestamp, metadata FROM memories WHERE user_id = ? AND conversation_id = ? ORDER BY timestamp DESC LIMIT ?";
                        query = sqlx::query(query_str)
                            .bind(&user_id)
                            .bind(conv)
                            .bind(limit as i64);
                    }
                    (None, Some(after), Some(before)) => {
                        query_str = "SELECT id, user_id, conversation_id, content, timestamp, metadata FROM memories WHERE user_id = ? AND timestamp > ? AND timestamp < ? ORDER BY timestamp DESC LIMIT ?";
                        query = sqlx::query(query_str)
                            .bind(&user_id)
                            .bind(after)
                            .bind(before)
                            .bind(limit as i64);
                    }
                    (None, Some(after), None) => {
                        query_str = "SELECT id, user_id, conversation_id, content, timestamp, metadata FROM memories WHERE user_id = ? AND timestamp > ? ORDER BY timestamp DESC LIMIT ?";
                        query = sqlx::query(query_str)
                            .bind(&user_id)
                            .bind(after)
                            .bind(limit as i64);
                    }
                    (None, None, Some(before)) => {
                        query_str = "SELECT id, user_id, conversation_id, content, timestamp, metadata FROM memories WHERE user_id = ? AND timestamp < ? ORDER BY timestamp DESC LIMIT ?";
                        query = sqlx::query(query_str)
                            .bind(&user_id)
                            .bind(before)
                            .bind(limit as i64);
                    }
                    (None, None, None) => {
                        query_str = "SELECT id, user_id, conversation_id, content, timestamp, metadata FROM memories WHERE user_id = ? ORDER BY timestamp DESC LIMIT ?";
                        query = sqlx::query(query_str)
                            .bind(&user_id)
                            .bind(limit as i64);
                    }
                }
                
                let rows = match query.fetch_all(&pool).await {
                    Ok(rows) => rows,
                    Err(e) => return ToolResult::error(format!("Failed to query memories: {}", e)),
                };
                
                let mut memos = Vec::new();
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
        };

        match memories {
            Ok(memos) => {
                // Format results
                let mut results = Vec::new();
                for memo in memos {
                    results.push(serde_json::json!({
                        "memory_id": memo.id,
                        "user_id": memo.user_id,
                        "conversation_id": memo.conversation_id,
                        "content": memo.content,
                        "timestamp": memo.timestamp,
                        "metadata": memo.metadata,
                        "formatted_timestamp": DateTime::from_timestamp(memo.timestamp, 0)
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_else(|| "invalid timestamp".to_string()),
                    }));
                }
                
                let response_json = serde_json::json!({
                    "count": results.len(),
                    "memories": results,
                }).to_string();
                
                ToolResult::success(response_json)
            }
            Err(e) => ToolResult::error(format!("Failed to search memories: {}", e)),
        }
    }
}

/// Tool for updating existing memories
pub struct UpdateMemoryTool;

#[async_trait]
impl Tool for UpdateMemoryTool {
    fn name(&self) -> &str {
        "update_memory"
    }

    fn description(&self) -> &str {
        "Update an existing memory. Only the owner of the memory can update it. You can update the content and/or metadata."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("memory_id", "string", "ID of the memory to update", required),
            param!("user_id", "string", "ID of the user who owns the memory", required),
            param!("content", "string", "New content for the memory (optional)", optional, ""),
            param!("metadata", "string", "New metadata as JSON string (optional)", optional, ""),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        // Validate and extract parameters
        let memory_id = match params.get_required::<String>("memory_id") {
            Ok(memory_id) => memory_id,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let user_id = match params.get_required::<String>("user_id") {
            Ok(user_id) => user_id,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let content = params.get_optional::<String>("content").unwrap_or(None);
        let metadata = params.get_optional::<String>("metadata").unwrap_or(None);

        // Validate memory_id
        if memory_id.trim().is_empty() {
            return ToolResult::error("memory_id cannot be empty".to_string());
        }

        // Validate user_id
        if user_id.trim().is_empty() {
            return ToolResult::error("user_id cannot be empty".to_string());
        }

        // Validate content if provided
        if let Some(ref c) = content {
            if c.trim().is_empty() {
                return ToolResult::error("content cannot be empty if provided".to_string());
            }
            if c.len() > 100000 {
                return ToolResult::error("content cannot exceed 100,000 characters".to_string());
            }
        }

        // Initialize database connection
        let db_path = get_memory_db_path();
        let pool = match connect_pool(&db_path).await {
            Ok(pool) => pool,
            Err(e) => return ToolResult::error(format!("Failed to connect to database: {}", e)),
        };

        // Initialize database (create tables if not exist)
        if let Err(e) = init_db(&pool).await {
            return ToolResult::error(format!("Failed to initialize database: {}", e));
        }

        // First, verify that the memory exists and belongs to the user
        let memory_check = sqlx::query(
            "SELECT id, user_id FROM memories WHERE id = ?"
        )
        .bind(&memory_id)
        .fetch_optional(&pool)
        .await;

        let (memory_exists, memory_user_id) = match memory_check {
            Ok(Some(row)) => {
                let db_id: String = row.get(0);
                let db_user_id: String = row.get(1);
                (true, db_user_id)
            }
            Ok(None) => (false, String::new()),
            Err(e) => return ToolResult::error(format!("Failed to check memory: {}", e)),
        };

        if !memory_exists {
            return ToolResult::error(format!("Memory with ID '{}' not found", memory_id));
        }

        // Validate that the user owns the memory
        if memory_user_id != user_id {
            return ToolResult::error(
                "You can only update memories that belong to you".to_string()
            );
        }

        // Build the update query based on what fields are provided
        let mut query_str = String::from("UPDATE memories SET ");
        let mut set_clauses = Vec::new();

        if content.is_some() {
            set_clauses.push("content = ?".to_string());
        }

        if metadata.is_some() {
            set_clauses.push("metadata = ?".to_string());
        }

        // Update the timestamp to mark when the memory was last updated
        set_clauses.push("timestamp = ?".to_string());

        query_str.push_str(&set_clauses.join(", "));
        query_str.push_str(" WHERE id = ?");

        // Execute the update
        let mut query = sqlx::query(&query_str);

        // Bind parameters in order
        if let Some(ref c) = content {
            query = query.bind(c);
        }

        if let Some(ref m) = metadata {
            query = query.bind(m);
        }

        query = query
            .bind(Utc::now().timestamp())
            .bind(&memory_id);

        let result = query.execute(&pool).await;

        match result {
            Ok(_) => {
                // Fetch the updated memory to return it
                let updated_memory = sqlx::query(
                    "SELECT id, user_id, conversation_id, content, timestamp, metadata FROM memories WHERE id = ?"
                )
                .bind(&memory_id)
                .fetch_one(&pool)
                .await;

                match updated_memory {
                    Ok(row) => {
                        let memory = Memory {
                            id: row.get(0),
                            user_id: row.get(1),
                            conversation_id: row.get(2),
                            content: row.get(3),
                            timestamp: row.get(4),
                            metadata: row.get(5),
                        };

                        let response_json = serde_json::json!({
                            "message": "Memory updated successfully",
                            "memory_id": memory.id,
                            "user_id": memory.user_id,
                            "conversation_id": memory.conversation_id,
                            "content": memory.content,
                            "timestamp": memory.timestamp,
                            "metadata": memory.metadata,
                            "formatted_timestamp": DateTime::from_timestamp(memory.timestamp, 0)
                                .map(|dt| dt.to_rfc3339())
                                .unwrap_or_else(|| "invalid timestamp".to_string()),
                        }).to_string();

                        ToolResult::success(response_json)
                    }
                    Err(e) => ToolResult::error(format!("Failed to retrieve updated memory: {}", e)),
                }
            }
            Err(e) => ToolResult::error(format!("Failed to update memory: {}", e)),
        }
    }
}
