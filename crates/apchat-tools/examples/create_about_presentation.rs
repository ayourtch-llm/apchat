// Create the About APChat presentation
use apchat_toolcore::{Tool, ToolParameters, ToolContext};
use apchat_tools::CreatePresentationTool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;

#[tokio::main]
async fn main() {
    let tool = CreatePresentationTool;
    
    // Read the presentation JSON
    let json_content = fs::read_to_string("/tmp/about_apchat_presentation.json")
        .expect("Failed to read presentation JSON");
    
    let params = ToolParameters {
        data: HashMap::from([
            ("path".to_string(), serde_json::Value::String("About_APChat.pptx".to_string())),
            ("title".to_string(), serde_json::json!("About APChat")),
            ("author".to_string(), serde_json::json!("APChat AI Assistant")),
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
                        ],
                        "background_color": "FFFFFF"
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
                        ],
                        "background_color": "F5F5F5"
                    },
                    {
                        "type": "content",
                        "title": "Architecture",
                        "bullets": [
                            "Modular crate-based design (Rust workspace)",
                            "apchat-toolcore: Tool registry and execution framework",
                            "apchat-tools: 50+ tools for development tasks",
                            "apchat-models: Multi-backend AI model support",
                            "apchat-skills: Workflow patterns and best practices",
                            "apchat-terminal: PTY session management"
                        ],
                        "background_color": "FFFFFF"
                    },
                    {
                        "type": "content",
                        "title": "Recent Additions",
                        "bullets": [
                            "📊 PPTX tools: create, read, and edit presentations",
                            "🖼️ Image processing for multimodal models",
                            "🐍 Python sandbox via ouros",
                            "🔍 SearXNG web search integration",
                            "💼 Financial services skills suite",
                            "📱 Webex bot with WebSocket support"
                        ],
                        "background_color": "F5F5F5"
                    },
                    {
                        "type": "content",
                        "title": "Use Cases",
                        "bullets": [
                            "Code review and refactoring assistance",
                            "Test-driven development workflow",
                            "Systematic debugging with root cause analysis",
                            "Documentation generation and maintenance",
                            "Presentation creation and editing",
                            "Knowledge extraction from large codebases"
                        ],
                        "background_color": "FFFFFF"
                    },
                    {
                        "type": "content",
                        "title": "Getting Started",
                        "bullets": [
                            "Install: cargo install apchat",
                            "Basic usage: apchat --interactive",
                            "With local model: apchat --llama-cpp-url http://localhost:8080",
                            "Enable tools: apchat --pptx-tools --image-processing",
                            "Auto-confirm mode: apchat --auto-confirm",
                            "Learn skills: apchat --early-superpowers"
                        ],
                        "background_color": "F5F5F5"
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