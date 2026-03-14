//! Configuration management for PTY server

use std::env;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Configuration for the PTY server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Directory for storing session logs
    pub log_dir: PathBuf,
    /// Default terminal width
    pub default_cols: u16,
    /// Default terminal height
    pub default_rows: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_dir: Self::default_log_dir(),
            default_cols: 80,
            default_rows: 24,
        }
    }
}

impl Config {
    /// Get the default log directory from environment or temp
    pub fn default_log_dir() -> PathBuf {
        env::var("PTY_LOG_DIR")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::temp_dir().join("apchat-pty-logs")
            })
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            log_dir: env::var("PTY_LOG_DIR")
                .ok()
                .map(PathBuf::from)
                .unwrap_or_else(Self::default_log_dir),
            default_cols: env::var("PTY_DEFAULT_COLS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(80),
            default_rows: env::var("PTY_DEFAULT_ROWS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(24),
        }
    }

    /// Get the log directory
    pub fn log_dir(&self) -> &PathBuf {
        &self.log_dir
    }
}

/// Environment-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnvironmentConfig {
    /// Development environment
    Dev,
    /// Production environment
    Prod,
}

impl EnvironmentConfig {
    /// Get the current environment
    pub fn current() -> Self {
        env::var("ENVIRONMENT")
            .map(|v| match v.to_lowercase().as_str() {
                "production" | "prod" => EnvironmentConfig::Prod,
                _ => EnvironmentConfig::Dev,
            })
            .unwrap_or(EnvironmentConfig::Dev)
    }

    /// Check if running in production
    pub fn is_prod(&self) -> bool {
        matches!(self, EnvironmentConfig::Prod)
    }
}