// Test reading the GBP presentation
use apchat_toolcore::{Tool, ToolParameters, ToolContext};
use apchat_tools::ReadPptxTool;
use std::collections::HashMap;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let tool = ReadPptxTool;
    
    let params = ToolParameters {
        data: HashMap::from([
            ("path".to_string(), serde_json::Value::String("/tmp/GBP with Multi DNAC Single ISE LA Oct 20.pptx".to_string())),
        ]),
    };

    let context = ToolContext::new(
        PathBuf::from("/Users/ayourtch/llm-rust/a/apchat"),
        "test_session".to_string(),
        apchat_policy::PolicyManager::new(),
    );

    let result = tool.execute(params, &context).await;
    println!("Success: {}", result.success);
    println!("\nResult:\n{}", result.content);
}