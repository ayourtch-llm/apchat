//! Inference outcome types for the REPL.
//!
//! This module defines the `InferenceOutcome` enum which represents the result
//! of a single inference cycle in the REPL.
//!
//! Note: The actual inference logic has been moved to `run_tool_loop()` in the
//! parent `repl` module as part of the REPL refactoring. This module now only
//! contains the outcome type definition.

/// Outcome of a single inference cycle.
pub enum InferenceOutcome {
    /// Inference completed successfully with this response text.
    Response(String),
    /// Inference was interrupted by the user (Ctrl-C or interrupt signal).
    /// An "[Interrupted by user]" assistant message has already been pushed.
    Interrupted,
    /// Inference failed with an error.
    /// An "[Error: ...]" assistant message has already been pushed.
    Error,
}
