// Webex bot integration for APChat
// Provides input routing from Webex and output broadcasting to Webex

pub mod client;
pub mod config;
pub mod input_router;
pub mod output_sink;
pub mod types;

pub use client::WebexClient;
pub use config::load_webex_secret;
pub use input_router::WebexInputRouter;
pub use output_sink::WebexOutputSink;
