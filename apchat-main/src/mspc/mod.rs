// MSPC module - Multi-Stream Processing Channel
// Re-exports from apchat-mspc crate

use tokio::sync::broadcast;
use once_cell::sync::Lazy;
use std::sync::Arc;

/// Global broadcast channel for TextOutput messages
/// Allows non-blocking sends from synchronous code
pub static TEXT_OUTPUT_TX: Lazy<broadcast::Sender<apchat_mspc::output::TextOutput>> =
    Lazy::new(|| broadcast::channel(100).0);

pub mod router;
pub mod destinations;

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

/// Initialize the OutputRouter and connect it to vty's TEXT_OUTPUT_TX
/// This should be called during application startup
pub async fn initialize_output_router() -> Arc<OutputRouter> {
    use apchat_vty;
    
    let router = Arc::new(OutputRouter::new());
    
    // Register the terminal destination
    let terminal_dest = Arc::new(destinations::TerminalDestination::new("terminal"));
    router.register(terminal_dest).await;
    
    // Register the readline destination
    let readline_dest = Arc::new(destinations::ReadlineDestination::new("readline"));
    router.register(readline_dest).await;
    
    // Start the background monitoring task
    router.start_monitoring();
    
    // Set vty's TEXT_OUTPUT_TX to point to our TX
    // This connects print_with_emoji to the router
    let _ = apchat_vty::set_text_output_tx(TEXT_OUTPUT_TX.clone());
    
    router
}
