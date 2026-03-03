//! Inference outcome types for the REPL.
//!
//! This module defines the `InferenceOutcome` enum which represents the result
//! of a single inference cycle in the REPL.

/// Outcome of a single inference cycle.
#[derive(PartialEq, Debug, Clone)]
pub enum InferenceOutcome {
    /// Inference completed successfully with this response text.
    Response(String),
    /// Inference was interrupted by the user (Ctrl-C or interrupt signal).
    /// An "[Interrupted by user]" assistant message has already been pushed.
    Interrupted,
    /// Inference failed with an error.
    /// An "[Error: ...]" assistant message has already been pushed.
    Error,
    /// Inference had toolcalls, needs to continue
    ToolsContinue,
}