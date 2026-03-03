//! Tool modules for APChat
//!
//! This module contains all available tools that can be used by AI models,
//! organized by functionality (file operations, search, system, model management, project tools, web).

pub mod file_ops;
pub mod search;
pub mod system;
pub mod model_management;
pub mod iteration_control;
pub mod llm_oneshot;

pub mod project_tools;
pub mod helpers;
pub mod skill_tools;
pub mod todo_tools;
pub mod terminal_tools;
pub mod read_file;
pub mod subagent_tools;
pub mod web;

pub mod memory;
pub mod file_curly_glance;
pub mod pdf_reader;
pub mod pdf_tool;
pub mod pdf_content_parser;
pub mod long_wait;
pub mod metacog;
pub mod self_regulate;
pub mod context_edit;
pub mod diff_fuzz;
pub mod rlm;
pub mod mcp_client;
pub mod searxng;
pub mod python_sandbox;
pub mod save_full_state;
pub mod load_full_state;
pub mod send_webex_message;
pub mod delete_webex_message;

#[cfg(feature = "forecasting")]
pub mod forecasting;

pub use file_ops::*;
pub use search::*;
pub use system::*;
pub use model_management::*;
pub use iteration_control::*;
pub use skill_tools::*;
pub use todo_tools::*;
pub use terminal_tools::*;
pub use subagent_tools::*;
pub use web::*;
pub use llm_oneshot::*;
pub use file_curly_glance::*;
pub use memory::*;
pub use pdf_tool::*;
pub use long_wait::*;
pub use metacog::*;
pub use self_regulate::*;
pub use context_edit::*;
pub use diff_fuzz::*;
pub use rlm::*;
pub use python_sandbox::*;
pub use save_full_state::*;
pub use load_full_state::*;
pub use send_webex_message::*;
pub use delete_webex_message::*;


