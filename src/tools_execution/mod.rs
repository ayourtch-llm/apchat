// Tools execution module - tool parsing, execution, and validation
pub mod parsing;

// Re-export commonly used functions
pub use parsing::parse_xml_tool_calls;
