use std::path::PathBuf;
use std::fs;

pub struct ApChatPaths;

impl ApChatPaths {
    /// Base config directory (~/.config/apchat)
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("apchat")
    }

    /// Base data directory (~/.local/share/apchat)
    pub fn data_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"))
            .join("apchat")
    }

    /// Base cache directory (~/.cache/apchat)
    pub fn cache_dir() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("~/.cache"))
            .join("apchat")
    }

    /// Credentials file (config_dir/credentials.toml)
    pub fn credentials_file() -> PathBuf {
        Self::config_dir().join("credentials.toml")
    }

    /// Environment file (config_dir/env)
    pub fn env_file() -> PathBuf {
        Self::config_dir().join("env")
    }

    /// Histories directory (data_dir/histories)
    pub fn histories_dir() -> PathBuf {
        Self::data_dir().join("histories")
    }

    /// Sessions directory (data_dir/sessions)
    pub fn sessions_dir() -> PathBuf {
        Self::data_dir().join("sessions")
    }

    /// Logs directory (cache_dir/logs)
    pub fn logs_dir() -> PathBuf {
        Self::cache_dir().join("logs")
    }

    /// Fastembed cache directory (cache_dir/fastembed)
    pub fn fastembed_dir() -> PathBuf {
        Self::cache_dir().join("fastembed")
    }

    /// Candle cache directory (cache_dir/candle)
    pub fn candle_dir() -> PathBuf {
        Self::cache_dir().join("candle")
    }

    /// Ensure a directory exists, creating it if necessary
    pub fn ensure_dir(path: &PathBuf) -> std::io::Result<()> {
        fs::create_dir_all(path)
    }

    /// Initialize all base directories
    pub fn init_all() -> std::io::Result<()> {
        Self::ensure_dir(&Self::config_dir())?;
        Self::ensure_dir(&Self::data_dir())?;
        Self::ensure_dir(&Self::cache_dir())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_dir_contains_apchat() {
        let dir = ApChatPaths::config_dir();
        assert!(dir.to_string_lossy().contains("apchat"));
    }

    #[test]
    fn test_data_dir_contains_apchat() {
        let dir = ApChatPaths::data_dir();
        assert!(dir.to_string_lossy().contains("apchat"));
    }

    #[test]
    fn test_cache_dir_contains_apchat() {
        let dir = ApChatPaths::cache_dir();
        assert!(dir.to_string_lossy().contains("apchat"));
    }

    #[test]
    fn test_credentials_file_in_config() {
        let file = ApChatPaths::credentials_file();
        assert!(file.to_string_lossy().contains("apchat"));
        assert!(file.file_name().map_or(false, |n| n == "credentials.toml"));
    }

    #[test]
    fn test_histories_dir_in_data() {
        let dir = ApChatPaths::histories_dir();
        assert!(dir.to_string_lossy().contains("histories"));
    }

    #[test]
    fn test_logs_dir_in_cache() {
        let dir = ApChatPaths::logs_dir();
        assert!(dir.to_string_lossy().contains("logs"));
    }
}