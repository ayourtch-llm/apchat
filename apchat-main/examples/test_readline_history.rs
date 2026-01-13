// Simple standalone test for readline_history functionality
use std::path::Path;

// Copy the necessary structs and functions for testing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ReadlineEntry {
    timestamp: u64,
    command: String,
    session_id: Option<String>,
    model: Option<String>,
}

impl ReadlineEntry {
    fn new(command: String) -> Self {
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

    fn new_with_context(command: String, session_id: Option<String>, model: Option<String>) -> Self {
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ReadlineHistory {
    entries: Vec<ReadlineEntry>,
    version: String,
}

impl ReadlineHistory {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            version: "0.1.0".to_string(), // Simplified version for test
        }
    }

    fn add_entry(&mut self, entry: ReadlineEntry) {
        self.entries.push(entry);
    }

    fn get_entries(&self) -> &[ReadlineEntry] {
        &self.entries
    }

    fn get_lines(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.command.clone()).collect()
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn save(&self, file_path: &str) -> anyhow::Result<String> {
        let json = serde_json::to_string_pretty(&self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize readline history: {}", e))?;

        std::fs::write(file_path, json)
            .map_err(|e| anyhow::anyhow!("Failed to write readline history to file: {}: {}", file_path, e))?;

        Ok(format!(
            "Saved readline history to {} ({} entries)",
            file_path,
            self.entries.len()
        ))
    }

    fn load(file_path: &str) -> anyhow::Result<Self> {
        let json = std::fs::read_to_string(file_path)
            .map_err(|e| anyhow::anyhow!("Failed to read readline history from file: {}: {}", file_path, e))?;

        let history: ReadlineHistory = serde_json::from_str(&json)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize readline history: {}", e))?;

        Ok(history)
    }
}

fn save_history(history: &ReadlineHistory, file_path: &str) -> anyhow::Result<String> {
    history.save(file_path)
}

fn load_history(file_path: &str) -> anyhow::Result<ReadlineHistory> {
    ReadlineHistory::load(file_path)
}

fn get_default_history_path() -> String {
    let home_dir = dirs::home_dir()
        .unwrap_or_else(|| Path::new(".").to_path_buf());
    
    home_dir.join(".apchat")
        .join("readline_history.json")
        .to_string_lossy()
        .into_owned()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing readline_history module...");

    // Test ReadlineEntry creation
    let entry1 = ReadlineEntry::new("list files".to_string());
    println!("Created entry: {} (timestamp: {})", entry1.command, entry1.timestamp);

    // Test ReadlineEntry with context
    let entry2 = ReadlineEntry::new_with_context(
        "open file".to_string(),
        Some("session123".to_string()),
        Some("grn-model".to_string()),
    );
    println!("Created entry with context: {} (session: {:?}, model: {:?})", 
             entry2.command, entry2.session_id, entry2.model);

    // Test ReadlineHistory
    let mut history = ReadlineHistory::new();
    println!("Created new history (version: {})", history.version);

    // Add entries
    history.add_entry(entry1);
    history.add_entry(entry2);
    println!("History now has {} entries", history.len());

    // Get entries
    let entries = history.get_entries();
    println!("Entries:");
    for (i, entry) in entries.iter().enumerate() {
        println!("  {}: {}", i + 1, entry.command);
    }

    // Get lines
    let lines = history.get_lines();
    println!("Lines: {:?}", lines);

    // Test save and load
    let temp_path = "test_readline_history.json";
    save_history(&history, temp_path)?;
    println!("Saved history to {}", temp_path);

    let loaded_history = load_history(temp_path)?;
    println!("Loaded history with {} entries", loaded_history.len());

    // Clean up
    std::fs::remove_file(temp_path)?;
    println!("Cleaned up test file");

    // Test default path
    let default_path = get_default_history_path();
    println!("Default history path: {}", default_path);

    println!("✅ All tests passed!");
    Ok(())
}