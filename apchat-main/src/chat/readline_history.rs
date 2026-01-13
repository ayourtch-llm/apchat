// Readline history module - manages command history persistence
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A single readline entry representing a command from the REPL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadlineEntry {
    pub timestamp: u64,  // Unix timestamp in milliseconds
    pub command: String,
    pub session_id: Option<String>,  // Optional session identifier
    pub model: Option<String>,  // Model used for this command
}

impl ReadlineEntry {
    /// Create a new ReadlineEntry
    pub fn new(command: String) -> Self {
        Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            command,
            session_id: None,
            model: None,
        }
    }

    /// Create a new ReadlineEntry with session and model information
    pub fn new_with_context(command: String, session_id: Option<String>, model: Option<String>) -> Self {
        Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            command,
            session_id,
            model,
        }
    }
}

/// Collection of readline entries for persistence
#[derive(Debug, Serialize, Deserialize)]
pub struct ReadlineHistory {
    pub entries: Vec<ReadlineEntry>,
    pub version: String,
}

impl ReadlineHistory {
    /// Create a new empty ReadlineHistory
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Add a new entry to the history
    pub fn add_entry(&mut self, entry: ReadlineEntry) {
        self.entries.push(entry);
    }

    /// Get all entries
    pub fn get_entries(&self) -> &[ReadlineEntry] {
        &self.entries
    }

    /// Get entries as strings for readline compatibility
    pub fn get_lines(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.command.clone()).collect()
    }

    /// Save history to a file
    pub fn save(&self, file_path: &str) -> Result<String> {
        let json = serde_json::to_string_pretty(&self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize readline history: {}", e))?;

        fs::write(file_path, json)
            .map_err(|e| anyhow::anyhow!("Failed to write readline history to file: {}: {}", file_path, e))?;

        Ok(format!(
            "Saved readline history to {} ({} entries)",
            file_path,
            self.entries.len()
        ))
    }

    /// Load history from a file
    pub fn load(file_path: &str) -> Result<Self> {
        let json = fs::read_to_string(file_path)
            .map_err(|e| anyhow::anyhow!("Failed to read readline history from file: {}: {}", file_path, e))?;

        let history: ReadlineHistory = serde_json::from_str(&json)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize readline history: {}", e))?;

        Ok(history)
    }

    /// Append entries from another history
    pub fn append(&mut self, other: &ReadlineHistory) {
        self.entries.extend(other.entries.clone());
    }

    /// Clear the history
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get history file path (default location)
    pub fn default_file_path() -> String {
        let home_dir = dirs::home_dir()
            .unwrap_or_else(|| Path::new(".").to_path_buf());
        
        home_dir.join(".apchat")
            .join("readline_history.json")
            .to_string_lossy()
            .into_owned()
    }
}

/// Save readline history to a file (standalone function)
pub fn save_history(history: &ReadlineHistory, file_path: &str) -> Result<String> {
    history.save(file_path)
}

/// Load readline history from a file (standalone function)
pub fn load_history(file_path: &str) -> Result<ReadlineHistory> {
    ReadlineHistory::load(file_path)
}

/// Create a default history file path
pub fn get_default_history_path() -> String {
    ReadlineHistory::default_file_path()
}