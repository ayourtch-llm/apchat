// Test reading the About APChat presentation
use apchat_toolcore::{Tool, ToolParameters, ToolContext};
use apchat_tools::ReadPptxTool;
use std::collections::HashMap;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let tool = ReadPptxTool;
    
    let params = ToolParameters {
        data: HashMap::from([
            ("path".to_string(), serde_json::Value::String("About_APChat.pptx".to_string())),
        ]),
    };

    let context = ToolContext::new(
        PathBuf::from("/Users/ayourtch/llm-rust/a/apchat"),
        "test_session".to_string(),
        apchat_policy::PolicyManager::new(),
    );

    let result = tool.execute(params, &context).await;
    println!("Result:\n{}", result.content);
    println!("\nSuccess: {}", result.success);
}