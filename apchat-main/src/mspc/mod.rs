// MSPC module - Multi-Stream Processing Channel
// Re-exports from apchat-mspc crate

use tokio::sync::broadcast;
use once_cell::sync::Lazy;

/// Global broadcast channel for TextOutput messages
/// Allows non-blocking sends from synchronous code
pub static TEXT_OUTPUT_TX: Lazy<broadcast::Sender<apchat_mspc::output::TextOutput>> =
    Lazy::new(|| broadcast::channel(100).0);

pub mod router;

pub use apchat_mspc::{
    MspcChannel,
    MspcMessage,
    MessagePair,
    HistoryError,
    OutputDestination,
    OutputMessage,
    broadcast_to_all,
};

pub use router::OutputRouter;

// Re-export output module and TextOutput type
pub use apchat_mspc::output;
pub use apchat_mspc::output::TextOutput;
