//! PTY (Pseudo-Terminal) Server Tools and MCP Protocol Implementation
//!
//! This crate provides:
//! - PTY tool implementations for apchat (Tool trait)
//! - MCP (Model Context Protocol) server implementation
//! - Shared configuration and utilities

pub mod config;
pub mod mcp;

// Re-export public API
pub use config::{Config, EnvironmentConfig};
pub use mcp::{McpHandler, tool_definitions};

/// Error type for the PTY server
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;