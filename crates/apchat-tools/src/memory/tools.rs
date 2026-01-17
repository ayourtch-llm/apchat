// Memory tools implementation

use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use chrono::{Utc, DateTime};

use crate::memory::{Memory, connect_pool, init_db, get_memory_db_path};

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
