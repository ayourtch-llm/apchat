//! Test to understand readline history loading/saving issues
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// Copy necessary types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ReadlineEntry {
    pub command: String,
    pub session_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ReadlineHistory {
    pub entries: Vec<ReadlineEntry>,
    pub version: String,
}

fn main() {
    // Test 1: Verify that get_logs_dir creates the directory
    println!("Test 1: Checking if logs directory is created...");
    let logs_dir = apchat_logging::get_logs_dir();
    match logs_dir {
        Ok(dir) => {
            println!("✓ Logs directory exists: {}", dir.display());
            println!("  Directory exists: {}", dir.exists());
        }
        Err(e) => {
            println!("✗ Failed to get logs directory: {}", e);
        }
    }
    
    // Test 2: Verify history file path
    println!("\nTest 2: Checking history file path...");
    let history_path = apchat_logging::get_logs_dir()
        .map(|dir| dir.join("readline_history.jsonl"))
        .map(|p| p.to_string_lossy().into_owned());
    
    match history_path {
        Ok(path) => {
            println!("✓ History file path: {}", path);
            println!("  File exists: {}", std::path::Path::new(&path).exists());
        }
        Err(e) => {
            println!("✗ Failed to get history path: {}", e);
        }
    }
    
    // Test 3: Try to load history
    println!("\nTest 3: Testing history load...");
    match apchat_main::chat::readline_history::load_history(None) {
        Ok(history) => {
            println!("✓ Successfully loaded history with {} entries", history.len());
        }
        Err(e) => {
            println!("✗ Failed to load history: {}", e);
        }
    }
    
    // Test 4: Try to save history
    println!("\nTest 4: Testing history save...");
    let test_history = ReadlineHistory {
        entries: vec![
            ReadlineEntry {
                command: "test command 1".to_string(),
                session_id: Some("session_test".to_string()),
                timestamp: chrono::Utc::now(),
            },
            ReadlineEntry {
                command: "test command 2".to_string(),
                session_id: Some("session_test".to_string()),
                timestamp: chrono::Utc::now(),
            },
        ],
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    
    match apchat_main::chat::readline_history::save_history(&test_history, None) {
        Ok(msg) => {
            println!("✓ Successfully saved history: {}", msg);
        }
        Err(e) => {
            println!("✗ Failed to save history: {}", e);
        }
    }
}
