// Chat module - conversation state management, history, and session handling
pub mod state;
pub mod history;
pub mod session;
pub mod readline_history; // Added readline_history module
pub mod mspc_session; // MSPC-integrated session module

// Re-export commonly used items
pub use state::{save_state, load_state};
pub use history::{calculate_conversation_size, get_max_session_size, should_compact_session, intelligent_compaction};
pub use readline_history::{ // Added readline_history exports
    ReadlineEntry,
    ReadlineHistory,
    save_history,
    load_history,
    load_and_add_to_editor,
    save_to_file,
    get_default_history_path,
    history_file_exists,
};

// Include test module only for lib tests
#[cfg(all(test, not(miri)))]
mod tests;

