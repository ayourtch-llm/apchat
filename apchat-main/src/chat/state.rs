use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

use apchat_models::{Message, ModelColor};
use apchat_todo::Task;

/// Serializable state for saving/loading conversations
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatState {
    pub messages: Vec<Message>,
    pub current_model: ModelColor,
    pub total_tokens_used: usize,
    pub version: String,
    /// Todo tasks (optional for backward compatibility with old save files)
    #[serde(default)]
    pub tasks: Vec<Task>,
}

impl ChatState {
    /// Create a new ChatState from the given parameters
    pub fn new(
        messages: Vec<Message>,
        current_model: ModelColor,
        total_tokens_used: usize,
        tasks: Vec<Task>,
    ) -> Self {
        Self {
            messages,
            current_model,
            total_tokens_used,
            version: env!("CARGO_PKG_VERSION").to_string(),
            tasks,
        }
    }

    /// Save the chat state to a file
    pub fn save(&self, file_path: &str) -> Result<String> {
        let json = serde_json::to_string_pretty(&self)
            .context("Failed to serialize chat state")?;

        fs::write(file_path, json)
            .with_context(|| format!("Failed to write state to file: {}", file_path))?;

        Ok(format!(
            "Saved conversation state to {} ({} messages, {} tasks, {} total tokens)",
            file_path,
            self.messages.len(),
            self.tasks.len(),
            self.total_tokens_used
        ))
    }

    /// Load a chat state from a file
    pub fn load(file_path: &str) -> Result<Self> {
        let json = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read state from file: {}", file_path))?;

        let state: ChatState = serde_json::from_str(&json)
            .context("Failed to deserialize chat state")?;

        Ok(state)
    }
}

/// Save conversation state to a file (standalone function for backward compatibility)
pub fn save_state(
    messages: &[Message],
    current_model: &ModelColor,
    total_tokens_used: usize,
    tasks: &[Task],
    file_path: &str,
) -> Result<String> {
    let state = ChatState::new(
        messages.to_vec(),
        current_model.clone(),
        total_tokens_used,
        tasks.to_vec(),
    );
    state.save(file_path)
}

/// Load conversation state from a file (standalone function)
pub fn load_state(file_path: &str) -> Result<(Vec<Message>, ModelColor, usize, String, Vec<Task>)> {
    let state = ChatState::load(file_path)?;
    Ok((
        state.messages,
        state.current_model,
        state.total_tokens_used,
        state.version,
        state.tasks,
    ))
}
