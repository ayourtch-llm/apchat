// Chat module - conversation state management, history, and session handling
pub mod state;
pub mod history;
pub mod session;
pub mod readline_history; // Added readline_history module

// Re-export commonly used items
pub use state::{save_state, load_state};
pub use history::{calculate_conversation_size, get_max_session_size, should_compact_session, intelligent_compaction};
pub use readline_history::{ // Added readline_history exports
    ReadlineEntry,
    ReadlineHistory,
    save_history,
    load_history,
    get_default_history_path,
};

// Include test module
#[cfg(test)]
mod tests;
