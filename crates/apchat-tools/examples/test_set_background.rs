// Test the set_slide_background tool
use apchat_toolcore::{Tool, ToolParameters, ToolContext};
use apchat_tools::SetSlideBackgroundTool;
use std::collections::HashMap;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let tool = SetSlideBackgroundTool;
    
    let params = ToolParameters {
        data: HashMap::from([
            ("path".to_string(), serde_json::Value::String("test_rust_edit.pptx".to_string())),
            ("slide_number".to_string(), serde_json::Value::Number(1.into())),
            ("color".to_string(), serde_json::Value::String("E94560".to_string())),
        ]),
    };

    let context = ToolContext::new(
        PathBuf::from("/Users/ayourtch/llm-rust/a/apchat"),
        "test_session".to_string(),
        apchat_policy::PolicyManager::new(),
    );

    let result = tool.execute(params, &context).await;
    println!("Result: {}", result.content);
    println!("Success: {}", result.success);
}