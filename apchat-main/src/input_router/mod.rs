pub mod terminal;
pub mod webex;

#[cfg(test)]
mod tests;

pub use terminal::TerminalInputRouter;
pub use webex::WebexInputRouter;
