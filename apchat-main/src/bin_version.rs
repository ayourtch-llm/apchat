//! Binary version detection and auto-reload functionality.
//!
//! This module provides functionality to detect if the binary has been replaced
//! on disk and automatically save state to a temporary file, then re-execute
//! with loading of that temporary state.
//!
//! Uses std::process::Command for cross-platform compatibility.
//! Note: This spawns a new process (different PID), but preserves all state.
//! For true in-place execve() with PID preservation, see the alternative implementation below.

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

/// Re-execute the current binary with the temporary state file using Command
/// This spawns a new process (different PID) but preserves all state.
pub fn reexec_with_temp_state(
    temp_state_path: &Path,
    args: &[String],
) -> Result<()> {
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

/// Alternative: Re-execute using execve() for true in-place process replacement
/// This preserves the PID but is Unix-specific and more complex.
/// Currently not used due to complexity and potential issues.
#[cfg(target_os = "linux")]
pub fn reexec_with_execve(
    temp_state_path: &Path,
    args: &[String],
) -> Result<()> {
    use std::ffi::{CString, c_char};
    
    // Build argv array - must keep CStrings alive until execve completes
    let exe_cstr = CString::new(std::env::current_exe()?.to_str().ok_or_else(|| anyhow::anyhow!("Invalid path"))?)
        .map_err(|e| anyhow::anyhow!("Failed to convert path to C string: {}", e))?;
    
    let mut argv: Vec<CString> = Vec::new();
    argv.push(exe_cstr); // argv[0] should be the program name
    
    // Add --load flag with temp state path
    let load_arg = format!("--load {}", temp_state_path.display());
    argv.push(CString::new(load_arg)
        .map_err(|e| anyhow::anyhow!("Failed to create --load argument: {}", e))?);
    
    // Add original args (skip binary path and --temp-state)
    for arg in args.iter().skip(1) {
        if !arg.starts_with("--temp-state") {
            argv.push(CString::new(arg.as_str())
                .map_err(|e| anyhow::anyhow!("Failed to convert argument to C string: {}", e))?);
        }
    }
    
    // Build envp array
    let mut envp: Vec<CString> = std::env::vars()
        .map(|(k, v)| CString::new(format!("{}={}", k, v)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("Failed to convert environment variable to C string: {}", e))?;
    
    // Call execve - this will never return if successful
    unsafe {
        // Convert CStrings to pointers
        let argv_ptrs: Vec<*const c_char> = argv.iter().map(|s| s.as_ptr() as *const c_char).collect();
        let env_ptrs: Vec<*const c_char> = envp.iter().map(|s| s.as_ptr() as *const c_char).collect();
        
        let result = libc::execve(
            argv[0].as_ptr(),
            argv_ptrs.as_ptr(),
            env_ptrs.as_ptr(),
        );
        
        // If we get here, execve failed
        let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
        let err_msg = std::io::Error::from_raw_os_error(errno).to_string();
        Err(anyhow::anyhow!("execve() failed with errno {}: {}", errno, err_msg))
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