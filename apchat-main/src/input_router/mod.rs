use anyhow::Result;
use crate::mspc::{MspcChannel, MspcMessage};

pub mod manager;
pub mod terminal;
pub mod webex;

#[cfg(test)]
mod tests;

pub use manager::InputSourceManager;
pub use terminal::TerminalInputRouter;
pub use webex::WebexInputRouter;

/// Input router trait for common interface
pub trait InputRouter {
    fn send_message(&self, message: MspcMessage) -> impl std::future::Future<Output = Result<()>> + Send;
    // Removed run() method as it's not Send due to stdin handling
}

impl InputRouter for TerminalInputRouter {
    async fn send_message(&self, message: MspcMessage) -> Result<()> {
        self.channel.send(message).await
            .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))?;
        Ok(())
    }
}

impl InputRouter for WebexInputRouter {
    async fn send_message(&self, message: MspcMessage) -> Result<()> {
        self.channel.send(message).await
            .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))?;
        Ok(())
    }
}
