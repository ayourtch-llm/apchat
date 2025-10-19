# Kimi Chat - Claude Code-like Experience

A Rust-based CLI application that uses Groq's API to interact with multiple AI models (Kimi-K2-Instruct-0905 and GPT-OSS-120B), providing a Claude Code-like experience with file operations, tool calling, and automatic model switching capabilities.

## Features

- **Multi-Model Support**: Automatically switch between Kimi-K2-Instruct-0905 and GPT-OSS-120B
- **Intelligent Model Switching**: Models can autonomously switch to better-suited models for specific tasks
- **Interactive Chat Interface**: Natural conversation with AI models
- **File Operations**: Read, write, edit, and list files in a dedicated workspace
- **Tool Calling**: Automatic tool execution when the model needs to interact with files
- **Conversation History**: Maintains context throughout the session and across model switches
- **Colored Output**: Beautiful terminal UI with colored output and model indicators

## Prerequisites

- Rust (latest stable version)
- A Groq API key with access to both Kimi-K2-Instruct-0905 and GPT-OSS-120B models

## Installation

1. Clone or navigate to this repository:
```bash
cd /Users/ayourtch/rust/kimichat
```

2. Build the project:
```bash
cargo build --release
```

## Configuration

Set your Groq API key as an environment variable:

```bash
export GROQ_API_KEY=your_api_key_here
```

## Usage

Run the application:

```bash
cargo run
```

Or use the release build:

```bash
./target/release/kimichat
```

The application will:
1. Create a `workspace` directory in the current directory if it doesn't exist
2. Start an interactive chat session with Kimi-K2-Instruct-0905 as the default model
3. Allow you to chat with AI models that can perform file operations and switch between models

### Available Models

- **Kimi-K2-Instruct-0905**: Good for general tasks, coding, and quick responses (default)
- **GPT-OSS-120B**: Good for complex reasoning, analysis, and advanced problem-solving

Models can automatically switch to the most appropriate model for your request using the `switch_model` tool.

### Available Tools

The AI assistant has access to the following tools:

- **read_file**: Read the contents of a file from the workspace directory
- **write_file**: Write content to a file in the workspace directory
- **edit_file**: Edit a file by replacing old content with new content
- **list_files**: List files in the workspace matching a glob pattern
- **switch_model**: Switch to a different AI model (used automatically by the models)

### Example Interactions

#### Basic File Operations

```
[Kimi-K2-Instruct-0905] You: Create a hello.txt file with "Hello, World!"

🔧 Calling tool: write_file with args: {"file_path":"hello.txt","content":"Hello, World!"}
📋 Result: Successfully wrote to hello.txt

[Kimi-K2-Instruct-0905] Assistant: I've created a hello.txt file with "Hello, World!" in your workspace.
```

#### Automatic Model Switching

The models can automatically switch when they determine another model would be better suited for the task:

```
[Kimi-K2-Instruct-0905] You: Analyze the algorithmic complexity of quicksort and explain the mathematical proof

🔧 Calling tool: switch_model with args: {"model":"gpt-oss","reason":"Complex mathematical analysis and proof explanation"}
📋 Result: Switched from Kimi-K2-Instruct-0905 to GPT-OSS-120B - Reason: Complex mathematical analysis and proof explanation

[GPT-OSS-120B] Assistant: [Provides detailed mathematical analysis...]
```

### Model Indicators

The current model is always displayed in the prompt:
- `[Kimi-K2-Instruct-0905]` - Kimi model is active
- `[GPT-OSS-120B]` - GPT-OSS model is active

### How Model Switching Works

1. Models are aware they can switch to other models via the `switch_model` tool
2. When a model determines another model would be better suited, it automatically calls `switch_model`
3. Conversation history is preserved across model switches
4. The new model continues the conversation with full context

## Architecture

The application consists of:
- **Model Management**: Enum-based model type system with display names
- **API Client**: Groq API integration with model-aware requests
- **Tool System**: Extensible tool calling framework
- **File Operations**: Safe workspace-scoped file manipulation
- **Interactive UI**: Colored terminal interface with model indicators

## Project Structure

```
kimichat/
├── Cargo.toml          # Dependencies
├── src/
│   └── main.rs         # Main application (~450 lines)
├── workspace/          # Working directory (auto-created)
└── README.md           # Documentation
```

## Contributing

Feel free to extend the project with:
- Additional AI models
- More file operation tools
- Enhanced UI features
- Custom model switching logic

## License

This project is provided as-is for educational and development purposes