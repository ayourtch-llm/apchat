// Readline history module - manages command history persistence
use anyhow::Result;
use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use apchat_common::ApChatPaths;

/// A single readline entry representing a command from the REPL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadlineEntry {
    pub command: String,
    pub session_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl ReadlineEntry {
    /// Create a new ReadlineEntry
    pub fn new(line: &str) -> Self {
        Self {
            command: line.to_string(),
            session_id: None,
            timestamp: Utc::now(),
        }
    }
    
    /// Create a new ReadlineEntry with session context
    pub fn with_session(line: &str, session_id: String) -> Self {
        Self {
            command: line.to_string(),
            session_id: Some(session_id),
            timestamp: Utc::now(),
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

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Get or create the logs directory using ApChatPaths
fn get_logs_dir() -> Result<PathBuf> {
    let logs_dir = ApChatPaths::logs_dir();
    
    // Create directory if it doesn't exist
    ApChatPaths::ensure_dir(&logs_dir)
        .context("Failed to create logs directory")?;

    Ok(logs_dir)
}

/// Get the default path for the readline history file
pub fn get_default_history_path() -> Result<String> {
    let logs_dir = get_logs_dir()?;
    let history_file = logs_dir.join("readline_history.jsonl");
    Ok(history_file.to_string_lossy().into_owned())
}

/// Check if history file exists at given path
pub fn history_file_exists(file_path: Option<&str>) -> bool {
    let path = match file_path {
        Some(p) => p.to_string(),
        None => get_default_history_path().unwrap_or_else(|_| String::new()),
    };
    
    Path::new(&path).exists()
}

/// Save readline history to a file in JSONL format
pub fn save_history(history: &ReadlineHistory, file_path: Option<&str>) -> Result<String> {
    let path = match file_path {
        Some(p) => p.to_string(),
        None => get_default_history_path()?,
    };
    
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(&path)
        .map_err(|e| anyhow::anyhow!("Failed to open history file: {}: {}", path, e))?;
    
    for entry in &history.entries {
        let json_line = serde_json::to_string(entry)
            .map_err(|e| anyhow::anyhow!("Failed to serialize ReadlineEntry: {}", e))?;
        writeln!(file, "{}", json_line)
            .map_err(|e| anyhow::anyhow!("Failed to write to history file: {}: {}", path, e))?;
    }
    
    Ok(format!(
        "Saved readline history to {} ({} entries)",
        path, history.entries.len()
    ))
}

/// Load readline history from a file
pub fn load_history(file_path: Option<&str>) -> Result<ReadlineHistory> {
    let path = match file_path {
        Some(p) => p.to_string(),
        None => get_default_history_path()?,
    };
    
    if !Path::new(&path).exists() {
        return Ok(ReadlineHistory::new());
    }
    
    let file = File::open(&path)
        .map_err(|e| anyhow::anyhow!("Failed to open history file: {}: {}", path, e))?;
    let reader = BufReader::new(file);
    
    let mut history = ReadlineHistory::new();
    let mut corrupted_lines = Vec::new();
    
    for (line_num, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| anyhow::anyhow!("Failed to read line from history file: {}", e))?;
        let trimmed = line.trim();

        // Skip empty lines and comment lines (starting with #)
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Try to parse the line as JSON
        match serde_json::from_str::<ReadlineEntry>(&trimmed) {
            Ok(entry) => {
                history.add_entry(entry);
            }
            Err(e) => {
                // Check if this is a "trailing characters" error, which suggests
                // multiple JSON objects concatenated on one line
                let error_msg = e.to_string().to_lowercase();
                if error_msg.contains("trailing character") {
                    // Try to extract valid JSON objects from the corrupted line
                    // by manually parsing the JSON objects (finding matching braces)
                    let mut recovered_count = 0;
                    let mut pos = 0;

                    while pos < trimmed.len() {
                        // Skip leading whitespace
                        while pos < trimmed.len() && trimmed.as_bytes()[pos].is_ascii_whitespace() {
                            pos += 1;
                        }

                        if pos >= trimmed.len() {
                            break;
                        }

                        // Find the next JSON object by matching braces
                        let bytes = trimmed.as_bytes();
                        if bytes[pos] != b'{' {
                            // Not a JSON object start, skip to next
                            pos += 1;
                            continue;
                        }

                        // Find matching closing brace
                        let mut brace_count = 0;
                        let mut end_pos = pos;
                        let mut found = false;

                        for i in pos..trimmed.len() {
                            match bytes[i] {
                                b'{' => brace_count += 1,
                                b'}' => {
                                    brace_count -= 1;
                                    if brace_count == 0 {
                                        end_pos = i + 1;
                                        found = true;
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        }

                        if !found {
                            // Couldn't find matching brace, give up
                            break;
                        }

                        // Try to parse the JSON object
                        let json_str = &trimmed[pos..end_pos];
                        match serde_json::from_str::<ReadlineEntry>(json_str) {
                            Ok(entry) => {
                                history.add_entry(entry);
                                recovered_count += 1;
                                pos = end_pos;

                                // Skip any whitespace/commas between objects
                                while pos < trimmed.len() {
                                    let next_char = bytes[pos];
                                    if next_char.is_ascii_whitespace() || next_char == b',' {
                                        pos += 1;
                                    } else {
                                        break;
                                    }
                                }
                            }
                            Err(_) => {
                                // Not a valid ReadlineEntry, skip this position
                                pos += 1;
                            }
                        }
                    }

                    if recovered_count > 0 {
                        eprintln!(
                            "⚠️  Recovered {} entries from corrupted line {} (multiple JSON objects on one line)",
                            recovered_count,
                            line_num + 1
                        );
                    } else {
                        // If we haven't recovered anything yet, this line is truly corrupted
                        corrupted_lines.push((line_num + 1, trimmed.to_string(), e.to_string()));
                    }
                } else {
                    // Other type of parsing error
                    corrupted_lines.push((line_num + 1, trimmed.to_string(), e.to_string()));
                }
            }
        }
    }
    
    // Report corrupted lines if any (excluding recovered ones)
    if !corrupted_lines.is_empty() {
        eprintln!(
            "⚠️  Found {} corrupted line(s) in readline history file:",
            corrupted_lines.len()
        );
        for (line_num, content, error) in &corrupted_lines {
            eprintln!("  Line {}: {}", line_num, error);
            // Truncate for display
            let display = if content.len() > 80 {
                format!("{}...", &content[..80])
            } else {
                content.clone()
            };
            eprintln!("    Content: {}", display);
        }
        eprintln!("  These lines have been skipped.");
    }
    
    Ok(history)
}

/// Clean up corrupted entries from the history file by rewriting it with only valid entries
/// This is useful if the history file has corrupted lines that need to be removed
pub fn cleanup_history_file(file_path: Option<&str>) -> Result<String> {
    let path = match file_path {
        Some(p) => p.to_string(),
        None => get_default_history_path()?,
    };
    
    // Load the history (which will handle corruption recovery)
    let history = load_history(Some(&path))?;
    
    // Rewrite the entire file with only valid entries
    let temp_path = format!("{}.clean", path);
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&temp_path)
        .map_err(|e| anyhow::anyhow!("Failed to create temp file: {}: {}", temp_path, e))?;
    
    for entry in history.get_entries() {
        let json_line = serde_json::to_string(entry)
            .map_err(|e| anyhow::anyhow!("Failed to serialize ReadlineEntry: {}", e))?;
        writeln!(file, "{}", json_line)
            .map_err(|e| anyhow::anyhow!("Failed to write to temp file: {}: {}", temp_path, e))?;
    }
    
    // Drop the file handle to ensure it's flushed
    drop(file);
    
    // Replace the original file with the cleaned version
    std::fs::rename(&temp_path, &path)
        .map_err(|e| anyhow::anyhow!("Failed to replace history file: {}: {}", path, e))?;
    
    Ok(format!(
        "Cleaned up history file {} ({} valid entries)",
        path,
        history.len()
    ))
}

/// Save a single readline entry to file (append mode)
pub fn save_to_file(entry: &ReadlineEntry) -> Result<String> {
    let path = get_default_history_path()?;
    let json_line = serde_json::to_string(entry)
        .map_err(|e| anyhow::anyhow!("Failed to serialize ReadlineEntry: {}", e))?;
    
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(&path)
        .map_err(|e| anyhow::anyhow!("Failed to open history file: {}", path))?;
    
    writeln!(file, "{}", json_line)
        .map_err(|e| anyhow::anyhow!("Failed to write to history file: {}", path))?;
    
    Ok(format!("Saved readline history to {} (1 entry)", path))
}

/// Load history and add to crossterm readline editor
pub fn load_and_add_to_editor(rl: &mut crate::readline::Readline) -> Result<()> {
    let history = load_history(None)?;

    for entry in history.get_entries() {
        rl.add_history_entry(&entry.command);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_readline_entry_creation() {
        let entry = ReadlineEntry::new("test command");
        
        assert_eq!(entry.command, "test command");
        assert_eq!(entry.session_id, None);
        assert!(entry.timestamp <= Utc::now());
    }

    #[test]
    fn test_readline_entry_with_session() {
        let entry = ReadlineEntry::with_session("test command", "session_12345".to_string());
        
        assert_eq!(entry.command, "test command");
        assert_eq!(entry.session_id, Some("session_12345".to_string()));
        assert!(entry.timestamp <= Utc::now());
    }

    #[test]
    fn test_readline_entry_serialization() {
        let entry = ReadlineEntry::with_session(
            "test command",
            "session_12345".to_string()
        );
        
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: ReadlineEntry = serde_json::from_str(&json).unwrap();
        
        assert_eq!(entry.command, deserialized.command);
        assert_eq!(entry.session_id, deserialized.session_id);
        // Timestamps should be very close
        assert!((entry.timestamp - deserialized.timestamp).num_milliseconds().abs() < 1000);
    }

    #[test]
    fn test_readline_history_operations() {
        let mut history = ReadlineHistory::new();
        
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
        
        let entry1 = ReadlineEntry::new("command 1");
        let entry2 = ReadlineEntry::with_session("command 2", "session_1".to_string());
        
        history.add_entry(entry1.clone());
        history.add_entry(entry2.clone());
        
        assert_eq!(history.len(), 2);
        assert!(!history.is_empty());
        
        let entries = history.get_entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].command, "command 1");
        assert_eq!(entries[1].command, "command 2");
        
        let lines = history.get_lines();
        assert_eq!(lines, vec!["command 1", "command 2"]);
    }

    #[test]
    fn test_save_and_load_history() {
        let temp_dir = TempDir::new().unwrap();
        let test_logs_dir = temp_dir.path().join("logs");
        fs::create_dir_all(&test_logs_dir).unwrap();
        
        // Create a temporary history file path
        let history_file = test_logs_dir.join("readline.jsonl");
        
        // Create test entries
        let entry1 = ReadlineEntry::with_session("command 1", "session_1".to_string());
        let entry2 = ReadlineEntry::with_session("command 2", "session_1".to_string());
        
        let mut history = ReadlineHistory::new();
        history.add_entry(entry1.clone());
        history.add_entry(entry2.clone());
        
        // Save history
        let save_result = save_history(&history, Some(history_file.to_str().unwrap()));
        assert!(save_result.is_ok());
        assert!(history_file.exists());
        
        // Load history
        let loaded_history = load_history(Some(history_file.to_str().unwrap())).unwrap();
        assert_eq!(loaded_history.len(), 2);
        assert_eq!(loaded_history.get_entries()[0].command, "command 1");
        assert_eq!(loaded_history.get_entries()[1].command, "command 2");
    }

    #[test]
    fn test_save_to_file() {
        let temp_dir = TempDir::new().unwrap();
        
        // Temporarily set the environment to use our test directory as HOME
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
        
        // Create and save a single entry
        let entry = ReadlineEntry::with_session("test command", "session_123".to_string());
        
        let save_result = save_to_file(&entry);
        
        // Restore original HOME
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
        
        // Verify the file was created at the expected location
        // On macOS, dirs::cache_dir() returns ~/Library/Caches
        // On Linux/Unix, it returns ~/.cache
        let cache_dir = if cfg!(target_os = "macos") {
            temp_dir.path().join("Library").join("Caches")
        } else {
            temp_dir.path().join(".cache")
        };
        let expected_path = cache_dir.join("apchat").join("logs").join("readline_history.jsonl");
        assert!(expected_path.exists(), "History file should exist at {}", expected_path.display());
        
        // Read and verify the content
        let content = fs::read_to_string(&expected_path).unwrap();
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        
        assert_eq!(lines.len(), 1, "Should have exactly 1 line, got {}", lines.len());
        
        let deserialized: ReadlineEntry = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(deserialized.command, "test command");
        assert_eq!(deserialized.session_id, Some("session_123".to_string()));
    }

    #[test]
    fn test_empty_history_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_logs_dir = temp_dir.path().join("logs");
        fs::create_dir_all(&test_logs_dir).unwrap();
        
        let history_file = test_logs_dir.join("readline.jsonl");
        fs::File::create(&history_file).unwrap(); // Empty file
        
        let history = load_history(Some(history_file.to_str().unwrap())).unwrap();
        assert_eq!(history.len(), 0);
        assert!(history.is_empty());
    }

    #[test]
    fn test_history_file_exists() {
        let temp_dir = TempDir::new().unwrap();
        let test_logs_dir = temp_dir.path().join("logs");
        fs::create_dir_all(&test_logs_dir).unwrap();
        
        let history_file = test_logs_dir.join("readline.jsonl");
        
        // Should be false when file doesn't exist
        let exists_before = history_file_exists(Some(history_file.to_str().unwrap()));
        assert!(!exists_before);
        
        // Create file and check again
        fs::File::create(&history_file).unwrap();
        let exists_after = history_file_exists(Some(history_file.to_str().unwrap()));
        assert!(exists_after);
    }

    #[test]
    fn test_get_default_history_path() {
        let path = get_default_history_path().unwrap();
        assert!(path.contains("readline_history.jsonl"));
        assert!(path.contains("logs"));
    }

    #[test]
    fn test_multiple_save_operations() {
        let temp_dir = TempDir::new().unwrap();
        let test_logs_dir = temp_dir.path().join("logs");
        fs::create_dir_all(&test_logs_dir).unwrap();
        
        let history_file = test_logs_dir.join("readline.jsonl");
        
        // Save multiple entries
        for i in 0..5 {
            let entry = ReadlineEntry::with_session(
                &format!("command {}", i),
                "session_test".to_string()
            );
            
            save_history(
                &ReadlineHistory {
                    entries: vec![entry],
                    version: "1.0.0".to_string(),
                },
                Some(history_file.to_str().unwrap())
            ).unwrap();
        }
        
        // Load and verify all entries exist
        let loaded = load_history(Some(history_file.to_str().unwrap())).unwrap();
        assert_eq!(loaded.len(), 5);
        
        for i in 0..5 {
            assert_eq!(loaded.get_entries()[i].command, format!("command {}", i));
        }
    }
}