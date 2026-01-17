// Terminal module
//
// Re-exports terminal functionality from apchat-terminal and apchat-tools crates.

pub mod input_listener;
pub use input_listener::TerminalInputListener;

// Re-export core types from apchat-terminal
pub use apchat_terminal::{TerminalManager, TerminalBackendType, MAX_CONCURRENT_SESSIONS};

// Re-export terminal tools from apchat-tools
pub use apchat_tools::{
    PtyLaunchTool, PtySendKeysTool, PtyGetScreenTool,
    PtyListTool, PtyKillTool, PtyGetCursorTool,
    PtyResizeTool, PtySetScrollbackTool,
    PtyStartCaptureTool, PtyStopCaptureTool,
    PtyRequestUserInputTool,
};
