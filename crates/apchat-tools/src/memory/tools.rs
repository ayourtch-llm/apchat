// Memory tools implementation

use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use chrono::Utc;
use sqlx::Row;

use crate::memory::{Memory, connect_pool, init_db, get_memory_db_path, search_memories, delete_memory, list_memories};

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

        let metadata_str = params.get_optional::<String>("metadata").unwrap_or(None);

        // Validate user_id
        if user_id.trim().is_empty() {
            return ToolResult::error("user_id cannot be empty".to_string());
        }

        // Validate conversation_id
        if conversation_id.trim().is_empty() {
            return ToolResult::error("conversation_id cannot be empty".to_string());
        }

        // Validate content
        if content.trim().is_empty() {
            return ToolResult::error("content cannot be empty".to_string());
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

        // Check permission using policy system
        let (approved, rejection_reason) = match context.check_permission(
            apchat_policy::ActionType::MemoryStore,
            &conversation_id,
            &format!("Store memory for conversation '{}'", conversation_id)
        ) {
            Ok((approved, reason)) => (approved, reason),
            Err(e) => return ToolResult::error(format!("Permission check failed: {}", e)),
        };

        if !approved {
            let error_msg = if let Some(reason) = rejection_reason {
                format!("Store operation cancelled: {}", reason)
            } else {
                "Store operation cancelled by policy".to_string()
            };
            return ToolResult::error(error_msg);
        }

        // Store the memory
        let memory = Memory::new(
            user_id.clone(),
            conversation_id.clone(),
            content.clone(),
            Utc::now().timestamp(),
            metadata_str,
        );

        let insert_result = sqlx::query(
            r#"
            INSERT INTO memories (id, user_id, conversation_id, content, timestamp, metadata)
            VALUES (?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&memory.id)
        .bind(&memory.user_id)
        .bind(&memory.conversation_id)
        .bind(&memory.content)
        .bind(memory.timestamp)
        .bind(memory.metadata)
        .execute(&pool)
        .await;

        match insert_result {
            Ok(_) => {
                let response_json = serde_json::json!({
                    "message": "Memory stored successfully",
                    "memory_id": memory.id,
                    "user_id": memory.user_id,
                    "conversation_id": memory.conversation_id,
                    "timestamp": memory.timestamp,
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
        "Query memories by keyword. Searches through memory content and returns matching memories."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("query", "string", "Search term to find in memory content", required),
            param!("user_id", "string", "Filter by user ID", optional, ""),
            param!("conversation_id", "string", "Filter by conversation ID", optional, ""),
            param!("limit", "integer", "Maximum number of results to return", optional, "50"),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        // Validate and extract parameters
        let query = match params.get_required::<String>("query") {
            Ok(query) => query,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let user_id = params.get_optional::<String>("user_id").unwrap_or(None);
        let conversation_id = params.get_optional::<String>("conversation_id").unwrap_or(None);
        let limit_str = params.get_optional::<String>("limit").unwrap_or(None);

        // Validate query
        if query.trim().is_empty() {
            return ToolResult::error("query cannot be empty".to_string());
        }

        let limit = limit_str.and_then(|s| s.parse::<usize>().ok());

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

        // Check permission using policy system
        let (approved, rejection_reason) = match context.check_permission(
            apchat_policy::ActionType::MemoryQuery,
            &query,
            &format!("Query memories with search term '{}'", query)
        ) {
            Ok((approved, reason)) => (approved, reason),
            Err(e) => return ToolResult::error(format!("Permission check failed: {}", e)),
        };

        if !approved {
            let error_msg = if let Some(reason) = rejection_reason {
                format!("Query operation cancelled: {}", reason)
            } else {
                "Query operation cancelled by policy".to_string()
            };
            return ToolResult::error(error_msg);
        }

        // Search for memories
        let memories = match search_memories(
            &pool,
            &query,
            user_id.as_deref(),
            conversation_id.as_deref(),
            limit,
        )
        .await {
            Ok(memories) => memories,
            Err(e) => return ToolResult::error(format!("Failed to search memories: {}", e)),
        };

        // Format the response
        let mut memories_json = Vec::new();
        for memory in memories {
            memories_json.push(serde_json::json!({
                "id": memory.id,
                "user_id": memory.user_id,
                "conversation_id": memory.conversation_id,
                "content": memory.content,
                "timestamp": memory.timestamp,
                "metadata": memory.metadata,
            }));
        }

        let response_json = serde_json::json!({
            "found": memories_json.len(),
            "memories": memories_json,
        }).to_string();

        ToolResult::success(response_json)
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
        "Update an existing memory. You can update the content and/or metadata of a memory."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("memory_id", "string", "ID of the memory to update", required),
            param!("user_id", "string", "ID of the user who owns the memory", required),
            param!("content", "string", "New content for the memory (optional if updating metadata)", optional, ""),
            param!("metadata", "string", "New metadata as JSON string (optional if updating content)", optional, ""),
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

        // Validate user_id
        if user_id.trim().is_empty() {
            return ToolResult::error("user_id cannot be empty".to_string());
        }

        // Validate that at least one field is provided
        if content.is_none() && metadata.is_none() {
            return ToolResult::error("At least one of content or metadata must be provided".to_string());
        }

        // Validate memory_id
        if memory_id.trim().is_empty() {
            return ToolResult::error("memory_id cannot be empty".to_string());
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

        // First, retrieve the existing memory to validate ownership
        let memory = sqlx::query(
            "SELECT id, user_id, conversation_id, content, timestamp, metadata FROM memories WHERE id = ?"
        )
        .bind(&memory_id)
        .fetch_optional(&pool)
        .await;

        let memory = match memory {
            Ok(Some(row)) => Memory {
                id: row.get(0),
                user_id: row.get(1),
                conversation_id: row.get(2),
                content: row.get(3),
                timestamp: row.get(4),
                metadata: row.get(5),
            },
            Ok(None) => return ToolResult::error(format!("Memory with ID '{}' not found", memory_id)),
            Err(e) => return ToolResult::error(format!("Failed to retrieve memory: {}", e)),
        };

        // Validate that the user owns the memory
        // Extract user_id from context or parameters - need to get it from the context
        // For now, we'll use the policy system to handle ownership, similar to other tools
        // TODO: Add explicit user_id parameter to UpdateMemoryTool for ownership validation

        // Check permission using policy system
        let (approved, rejection_reason) = match context.check_permission(
            apchat_policy::ActionType::MemoryUpdate,
            &memory_id,
            &format!("Update memory '{}'", memory_id)
        ) {
            Ok((approved, reason)) => (approved, reason),
            Err(e) => return ToolResult::error(format!("Permission check failed: {}", e)),
        };

        if !approved {
            let error_msg = if let Some(reason) = rejection_reason {
                format!("Update operation cancelled: {}", reason)
            } else {
                "Update operation cancelled by policy".to_string()
            };
            return ToolResult::error(error_msg);
        }

        // Build update query dynamically
        let mut set_clauses = Vec::new();
        let mut binds = Vec::new();

        if let Some(c) = content {
            set_clauses.push("content = ?");
            binds.push(c);
        }

        if let Some(m) = metadata {
            set_clauses.push("metadata = ?");
            binds.push(m);
        }

        binds.push(memory_id.clone());

        let query_str = format!(
            "UPDATE memories SET {} WHERE id = ?",
            set_clauses.join(", ")
        );

        // Build the query with sqlx::query
        let mut query = sqlx::query(&query_str);
        
        // Bind each value individually
        for bind in binds {
            query = query.bind(bind);
        }
        
        // Bind the memory_id as the last parameter (WHERE clause)
        let result = query.bind(&memory_id).execute(&pool).await;

        match result {
            Ok(result) => {
                if result.rows_affected() == 0 {
                    return ToolResult::error(format!("Memory with ID '{}' not found", memory_id));
                }

                // Retrieve the updated memory
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
                            "memory": {
                                "id": memory.id,
                                "user_id": memory.user_id,
                                "conversation_id": memory.conversation_id,
                                "content": memory.content,
                                "timestamp": memory.timestamp,
                                "metadata": memory.metadata,
                            }
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

/// Tool for deleting existing memories
pub struct DeleteMemoryTool;

#[async_trait]
impl Tool for DeleteMemoryTool {
    fn name(&self) -> &str {
        "delete_memory"
    }

    fn description(&self) -> &str {
        "Delete an existing memory. Only the owner of the memory can delete it. This action cannot be undone."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("memory_id", "string", "ID of the memory to delete", required),
            param!("user_id", "string", "ID of the user who owns the memory", required),
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

        // Validate memory_id
        if memory_id.trim().is_empty() {
            return ToolResult::error("memory_id cannot be empty".to_string());
        }

        // Validate user_id
        if user_id.trim().is_empty() {
            return ToolResult::error("user_id cannot be empty".to_string());
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
                let _db_id: String = row.get(0);
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
                "You can only delete memories that belong to you".to_string()
            );
        }

        // Check if running in interactive mode for confirmation
        if !context.non_interactive {
            // Check permission using policy system
            let (approved, rejection_reason) = match context.check_permission(
                apchat_policy::ActionType::MemoryDelete,
                &memory_id,
                &format!("Are you sure you want to delete memory '{}'? This action cannot be undone.", memory_id)
            ) {
                Ok((approved, reason)) => (approved, reason),
                Err(e) => return ToolResult::error(format!("Permission check failed: {}", e)),
            };

            if !approved {
                let error_msg = if let Some(reason) = rejection_reason {
                    format!("Delete cancelled: {}", reason)
                } else {
                    "Delete cancelled by user or policy".to_string()
                };
                return ToolResult::error(error_msg);
            }
        }

        // Delete the memory
        let deleted = match delete_memory(&pool, &memory_id).await {
            Ok(deleted) => deleted,
            Err(e) => return ToolResult::error(format!("Failed to delete memory: {}", e)),
        };

        if deleted {
            let response_json = serde_json::json!({
                "message": "Memory deleted successfully",
                "memory_id": memory_id,
            }).to_string();
            ToolResult::success(response_json)
        } else {
            ToolResult::error(format!("Memory with ID '{}' not found", memory_id))
        }
    }
}

/// Tool for listing memories with filtering and pagination
pub struct ListMemoriesTool;

#[async_trait]
impl Tool for ListMemoriesTool {
    fn name(&self) -> &str {
        "list_memories"
    }

    fn description(&self) -> &str {
        "List memories with optional filtering by user ID, conversation ID, and pagination. Returns memories sorted by timestamp (newest first)."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("user_id", "string", "Filter memories by user ID", optional, ""),
            param!("conversation_id", "string", "Filter memories by conversation ID", optional, ""),
            param!("limit", "integer", "Maximum number of memories to return", optional, "50"),
            param!("offset", "integer", "Number of memories to skip for pagination", optional, "0"),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        // Extract parameters
        let user_id = params.get_optional::<String>("user_id").unwrap_or(None);
        let conversation_id = params.get_optional::<String>("conversation_id").unwrap_or(None);
        let limit_str = params.get_optional::<String>("limit").unwrap_or(None);
        let offset_str = params.get_optional::<String>("offset").unwrap_or(None);

        // Parse limit and offset
        let limit = limit_str.and_then(|s| s.parse::<usize>().ok());
        let offset = offset_str.and_then(|s| s.parse::<usize>().ok());

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

        // Check permission using policy system
        let (approved, rejection_reason) = match context.check_permission(
            apchat_policy::ActionType::MemoryList,
            "",
            "List memories"
        ) {
            Ok((approved, reason)) => (approved, reason),
            Err(e) => return ToolResult::error(format!("Permission check failed: {}", e)),
        };

        if !approved {
            let error_msg = if let Some(reason) = rejection_reason {
                format!("List operation cancelled: {}", reason)
            } else {
                "List operation cancelled by policy".to_string()
            };
            return ToolResult::error(error_msg);
        }

        // List memories
        let memories = match list_memories(
            &pool,
            user_id.as_deref(),
            conversation_id.as_deref(),
            limit,
            offset,
        )
        .await {
            Ok(memories) => memories,
            Err(e) => return ToolResult::error(format!("Failed to list memories: {}", e)),
        };

        // Format the response
        let mut memories_json = Vec::new();
        for memory in memories {
            memories_json.push(serde_json::json!({
                "id": memory.id,
                "user_id": memory.user_id,
                "conversation_id": memory.conversation_id,
                "content": memory.content,
                "timestamp": memory.timestamp,
                "metadata": memory.metadata,
            }));
        }

        let response_json = serde_json::json!({
            "total": memories_json.len(),
            "memories": memories_json,
        }).to_string();

        ToolResult::success(response_json)
    }
}

