//! Binary version detection and auto-reload functionality.
//!
//! This module provides functionality to detect if the binary has been replaced
//! on disk and automatically save state to a temporary file, then re-execute
//! with loading of that temporary state.

use anyhow::Result;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Generate a default temporary state file path based on process ID
pub fn default_temp_state_path() -> PathBuf {
    let pid = std::process::id();
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(format!(".apchat_temp_state_{}.json", pid))
}

/// Calculate a hash of the binary file to detect if it has been replaced
pub fn get_binary_hash(binary_path: &Path) -> Result<String, std::io::Error> {
    let mut file = fs::File::open(binary_path)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    
    // Simple hash: sum of bytes (not cryptographically secure, but sufficient for change detection)
    let hash: u64 = contents.iter().map(|&b| b as u64).sum();
    Ok(format!("{:x}", hash))
}

/// Save current state to a temporary file
pub fn save_to_temp_state(
    state_content: &str,
    temp_path: &Path,
) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = temp_path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    let mut file = fs::File::create(temp_path)?;
    file.write_all(state_content.as_bytes())?;
    
    Ok(())
}

/// Check if binary has been replaced by comparing hash
/// Returns:
/// - Ok(Some(new_hash)) if binary exists and hash differs from stored
/// - Ok(None) if binary doesn't exist or hash matches
/// - Err if we can't read the binary
pub fn check_binary_replaced(
    binary_path: &Path,
    stored_hash_path: &Path,
) -> Result<Option<String>> {
    // Get current binary hash
    let current_hash = get_binary_hash(binary_path)?;
    
    // Read stored hash if exists
    let stored_hash = if stored_hash_path.exists() {
        let mut file = fs::File::open(stored_hash_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Some(contents.trim().to_string())
    } else {
        None
    };
    
    // If no stored hash or hashes match, binary hasn't been replaced
    if stored_hash.as_ref() != Some(&current_hash) {
        Ok(Some(current_hash))
    } else {
        Ok(None)
    }
}

/// Store the current binary hash for future comparison
pub fn store_binary_hash(binary_path: &Path, hash_path: &Path) -> Result<()> {
    let hash = get_binary_hash(binary_path)?;
    
    if let Some(parent) = hash_path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    fs::write(hash_path, &hash)?;
    
    Ok(())
}

/// Re-execute the current binary with the temporary state file
pub fn reexec_with_temp_state(
    temp_state_path: &Path,
    args: &[String],
) -> Result<(), anyhow::Error> {
    let current_exe = std::env::current_exe()?;
    let mut cmd = Command::new(current_exe);
    
    // Add the --load flag for the temp state
    cmd.arg("--load").arg(temp_state_path);
    
    // Add all original args except the binary path (args[0]) and --temp-state
    for arg in args.iter().skip(1) {
        if !arg.starts_with("--temp-state") {
            cmd.arg(arg);
        }
    }
    
    // Execute
    let status = cmd.status()?;
    
    if !status.success() {
        Err(anyhow::anyhow!("Re-execution failed with status: {}", status))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_default_temp_state_path() {
        let path = default_temp_state_path();
        assert!(path.to_string_lossy().contains(".apchat_temp_state_"));
        assert!(path.to_string_lossy().ends_with(".json"));
    }
    
    #[test]
    fn test_binary_hash_changes() {
        let temp_dir = TempDir::new().unwrap();
        let binary_path = temp_dir.path().join("test_binary");
        let hash_path = temp_dir.path().join("hash.txt");
        
        // Create initial binary
        fs::write(&binary_path, "initial content").unwrap();
        
        // Get hash
        let hash1 = get_binary_hash(&binary_path).unwrap();
        store_binary_hash(&binary_path, &hash_path).unwrap();
        
        // Check no replacement
        assert!(check_binary_replaced(&binary_path, &hash_path).unwrap().is_none());
        
        // Modify binary
        fs::write(&binary_path, "modified content").unwrap();
        
        // Check replacement detected
        assert!(check_binary_replaced(&binary_path, &hash_path).unwrap().is_some());
    }
}