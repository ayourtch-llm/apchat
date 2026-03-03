//! Shared types for APChat.
//!
//! This crate contains types that are shared across multiple APChat crates,
//! including the `InferenceOutcome` enum which represents the result of an inference cycle.

pub mod types;

pub use types::InferenceOutcome;