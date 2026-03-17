// Chat module - conversation state management, history, and session handling
pub mod state;
pub mod history;
pub mod session;
pub mod mspc_session; // MSPC-integrated session module
pub mod context_edit;
pub mod early_compaction;

// Re-export commonly used items
pub use state::{save_state, load_state};
pub use history::{calculate_conversation_size, get_max_session_size, should_compact_session, intelligent_compaction};
pub(crate) use history::{find_cutoff_preserving_tool_pairs, ensure_proper_role_alternation, extract_latest_todo_state};
pub use mspc_session::{execute_chat_turn, chat_with_mspc};

#[cfg(test)]
mod tests;
