// Create presentation using Cisco template
use apchat_toolcore::{Tool, ToolParameters, ToolContext};
use apchat_tools::CreatePresentationTool;
use std::collections::HashMap;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let tool = CreatePresentationTool;
    
    let params = ToolParameters {
        data: HashMap::from([
            ("path".to_string(), serde_json::Value::String("About_APChat_Themed.pptx".to_string())),
            ("title".to_string(), serde_json::json!("About APChat")),
            ("author".to_string(), serde_json::json!("APChat AI Assistant")),
            ("template".to_string(), serde_json::Value::String("/tmp/GBP with Multi DNAC Single ISE LA Oct 20.pptx".to_string())),
            ("slides".to_string(), serde_json::json!(
                [
                    {
                        "type": "title",
                        "title": "About APChat",
                        "subtitle": "A Rust CLI for AI-Powered Development",
                        "background_color": "005073"
                    },
                    {
                        "type": "content",
                        "title": "What is APChat?",
                        "bullets": [
                            "Rust-based CLI application for AI-assisted development",
                            "Multi-model support (Groq, Anthropic, Llama.cpp, OpenAI)",
                            "Interactive terminal interface with streaming responses",
                            "Extensible tool system for file operations, search, and more",
                            "Skills-based workflow for proven development patterns"
                        ]
                    },
                    {
                        "type": "content",
                        "title": "Key Features",
                        "bullets": [
                            "🔧 50+ built-in tools (file ops, search, git, terminal)",
                            "🎯 Skills system for TDD, debugging, planning workflows",
                            "🔄 Multi-model switching (blu/grn/red models)",
                            "📊 Subagent support for parallel task execution",
                            "💾 Memory system for conversation context",
                            "🌐 MCP server integration for extensibility"
                        ]
                    },
                    {
                        "type": "title",
                        "title": "Thank You!",
                        "subtitle": "Questions?",
                        "background_color": "005073"
                    }
                ]
            )),
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