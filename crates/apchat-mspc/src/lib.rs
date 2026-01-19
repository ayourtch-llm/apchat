// MSPC module - Multi-Stream Processing Channel
// Handles inputs from multiple sources (terminal, webex, etc.)
// and routes them to the LLM interaction loop

pub mod channel;
pub mod output;

pub use channel::{
    MspcChannel,
    MspcMessage,
    MessagePair,
    HistoryError,
};

pub use output::{
    OutputDestination,
    OutputMessage,
    broadcast_to_all,
};
