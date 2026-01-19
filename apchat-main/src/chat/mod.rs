// Chat module - conversation state management, history, and session handling
pub mod state;
pub mod history;
pub mod session;
pub mod readline_history; // Added readline_history module
pub mod readline_instance; // Singleton readline instance management
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
pub use readline_instance::ReadlineInstance;
pub use mspc_session::{execute_chat_turn, chat_with_mspc};


