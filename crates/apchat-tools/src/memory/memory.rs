// Memory model and core functions

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub user_id: String,
    pub conversation_id: String,
    pub content: String,
    pub timestamp: i64,
    pub metadata: Option<String>,
}

impl Memory {
    /// Create a new memory
    pub fn new(
        user_id: String,
        conversation_id: String,
        content: String,
        timestamp: i64,
        metadata: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            conversation_id,
            content,
            timestamp,
            metadata,
        }
    }
}
