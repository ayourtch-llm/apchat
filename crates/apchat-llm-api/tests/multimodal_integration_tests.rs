//! Multimodal integration tests for the LLM API
//! 
//! These tests verify:
//! - Multimodal message roundtrip (serialize/deserialize)
//! - Text-only message backward compatibility
//! - Message with multiple content parts

use apchat_models::types::{Message, ContentPart};
use serde_json;

/// Test multimodal message roundtrip serialization/deserialization
#[test]
fn test_multimodal_message_roundtrip() {
    // Create a message with both text and image content
    let original_message = Message {
        role: "user".to_string(),
        content: vec![
            ContentPart::Text("What is in this image?".to_string()),
            ContentPart::ImageUrl { 
                url: "https://example.com/image.jpg".to_string() 
            },
        ],
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning: None,
    };

    // Serialize to JSON
    let json_str = serde_json::to_string_pretty(&original_message).expect("Failed to serialize message");
    
    // Verify JSON structure
    assert!(json_str.contains("\"role\""));
    assert!(json_str.contains("\"content\""));
    assert!(json_str.contains("What is in this image?"));
    assert!(json_str.contains("https://example.com/image.jpg"));
    
    // Deserialize back
    let deserialized: Message = serde_json::from_str(&json_str).expect("Failed to deserialize message");
    
    // Verify roundtrip
    assert_eq!(original_message.role, deserialized.role);
    assert_eq!(original_message.content.len(), deserialized.content.len());
    
    // Check content parts
    match &deserialized.content[0] {
        ContentPart::Text(text) => assert_eq!(text, "What is in this image?"),
        _ => panic!("Expected Text content part"),
    }
    
    match &deserialized.content[1] {
        ContentPart::ImageUrl { url } => assert_eq!(url, "https://example.com/image.jpg"),
        _ => panic!("Expected ImageUrl content part"),
    }
}

/// Test backward compatibility with text-only messages (string content)
#[test]
fn test_text_only_message_backward_compatibility() {
    // Old format: content as a string
    let old_format_json = r#"{
        "role": "user",
        "content": "Hello, how are you?"
    }"#;
    
    // Should deserialize successfully
    let message: Message = serde_json::from_str(old_format_json).expect("Failed to deserialize old format message");
    
    // Verify it was converted to a single Text content part
    assert_eq!(message.role, "user");
    assert_eq!(message.content.len(), 1);
    
    match &message.content[0] {
        ContentPart::Text(text) => assert_eq!(text, "Hello, how are you?"),
        _ => panic!("Expected Text content part"),
    }
    
    // Serialize and verify it now uses the new array format
    let new_format_json = serde_json::to_string_pretty(&message).expect("Failed to serialize message");
    assert!(new_format_json.contains("\"content\""));
    
    // Deserialize again to ensure roundtrip works
    let message2: Message = serde_json::from_str(&new_format_json).expect("Failed to deserialize new format");
    assert_eq!(message.content.len(), message2.content.len());
}

/// Test message with multiple content parts
#[test]
fn test_message_multiple_content_parts() {
    // Create a complex message with multiple text and image parts
    let original_message = Message {
        role: "user".to_string(),
        content: vec![
            ContentPart::Text("Here are some images I'd like you to analyze:".to_string()),
            ContentPart::ImageUrl { 
                url: "https://example.com/image1.png".to_string() 
            },
            ContentPart::Text("And here's another one:".to_string()),
            ContentPart::ImageUrl { 
                url: "https://example.com/image2.png".to_string() 
            },
            ContentPart::Text("What do you notice about these images?".to_string()),
        ],
        tool_calls: None,
        tool_call_id: None,
        name: Some("test_user".to_string()),
        reasoning: None,
    };

    // Serialize to JSON
    let json_str = serde_json::to_string_pretty(&original_message).expect("Failed to serialize message");
    
    // Verify JSON structure contains all parts
    assert!(json_str.contains("Here are some images"));
    assert!(json_str.contains("image1.png"));
    assert!(json_str.contains("And here's another one"));
    assert!(json_str.contains("image2.png"));
    assert!(json_str.contains("What do you notice"));
    assert!(json_str.contains("test_user"));
    
    // Deserialize back
    let deserialized: Message = serde_json::from_str(&json_str).expect("Failed to deserialize message");
    
    // Verify all content parts
    assert_eq!(original_message.content.len(), deserialized.content.len());
    assert_eq!(deserialized.content.len(), 5);
    
    // Verify each content part in order
    match &deserialized.content[0] {
        ContentPart::Text(text) => assert_eq!(text, "Here are some images I'd like you to analyze:"),
        _ => panic!("Expected Text at index 0"),
    }
    
    match &deserialized.content[1] {
        ContentPart::ImageUrl { url } => assert_eq!(url, "https://example.com/image1.png"),
        _ => panic!("Expected ImageUrl at index 1"),
    }
    
    match &deserialized.content[2] {
        ContentPart::Text(text) => assert_eq!(text, "And here's another one:"),
        _ => panic!("Expected Text at index 2"),
    }
    
    match &deserialized.content[3] {
        ContentPart::ImageUrl { url } => assert_eq!(url, "https://example.com/image2.png"),
        _ => panic!("Expected ImageUrl at index 3"),
    }
    
    match &deserialized.content[4] {
        ContentPart::Text(text) => assert_eq!(text, "What do you notice about these images?"),
        _ => panic!("Expected Text at index 4"),
    }
    
    // Verify name field
    assert_eq!(deserialized.name, Some("test_user".to_string()));
}

/// Test assistant message with reasoning
#[test]
fn test_assistant_message_with_reasoning() {
    let original_message = Message {
        role: "assistant".to_string(),
        content: vec![
            ContentPart::Text("Based on my analysis, the image shows a cat.".to_string()),
        ],
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning: Some("I examined the visual features and recognized feline characteristics.".to_string()),
    };

    let json_str = serde_json::to_string_pretty(&original_message).expect("Failed to serialize");
    let deserialized: Message = serde_json::from_str(&json_str).expect("Failed to deserialize");
    
    assert_eq!(deserialized.role, "assistant");
    assert_eq!(deserialized.reasoning, Some("I examined the visual features and recognized feline characteristics.".to_string()));
}

/// Test message with tool calls
#[test]
fn test_message_with_tool_calls() {
    use apchat_models::types::{ToolCall, FunctionCall};
    
    let original_message = Message {
        role: "assistant".to_string(),
        content: vec![
            ContentPart::Text("I need to read that file for you.".to_string()),
        ],
        tool_calls: Some(vec![
            ToolCall {
                id: "tool_call_123".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "read_file".to_string(),
                    arguments: r#"{"file_path": "example.txt"}"#.to_string(),
                },
            }
        ]),
        tool_call_id: None,
        name: None,
        reasoning: None,
    };

    let json_str = serde_json::to_string_pretty(&original_message).expect("Failed to serialize");
    let deserialized: Message = serde_json::from_str(&json_str).expect("Failed to deserialize");
    
    assert!(deserialized.tool_calls.is_some());
    let tool_calls = deserialized.tool_calls.unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].id, "tool_call_123");
    assert_eq!(tool_calls[0].function.name, "read_file");
}

/// Test tool response message with tool_call_id
#[test]
fn test_tool_response_message() {
    let original_message = Message {
        role: "tool".to_string(),
        content: vec![
            ContentPart::Text("File content: Hello, World!".to_string()),
        ],
        tool_calls: None,
        tool_call_id: Some("tool_call_123".to_string()),
        name: Some("read_file".to_string()),
        reasoning: None,
    };

    let json_str = serde_json::to_string_pretty(&original_message).expect("Failed to serialize");
    let deserialized: Message = serde_json::from_str(&json_str).expect("Failed to deserialize");
    
    assert_eq!(deserialized.role, "tool");
    assert_eq!(deserialized.tool_call_id, Some("tool_call_123".to_string()));
    assert_eq!(deserialized.name, Some("read_file".to_string()));
}

/// Test empty content message
#[test]
fn test_empty_content_message() {
    let original_message = Message {
        role: "user".to_string(),
        content: vec![],
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning: None,
    };

    let json_str = serde_json::to_string_pretty(&original_message).expect("Failed to serialize");
    let deserialized: Message = serde_json::from_str(&json_str).expect("Failed to deserialize");
    
    assert_eq!(deserialized.content.len(), 0);
}

/// Test JSON array format with string content backward compatibility
#[test]
fn test_json_array_format_backward_compat() {
    // Test that a JSON array with a single text part works
    let json_with_array = r#"{
        "role": "user",
        "content": [
            {"type": "text", "text": "Hello from array format"}
        ]
    }"#;
    
    let message: Message = serde_json::from_str(json_with_array).expect("Failed to deserialize");
    assert_eq!(message.role, "user");
    assert_eq!(message.content.len(), 1);
    
    match &message.content[0] {
        ContentPart::Text(text) => assert_eq!(text, "Hello from array format"),
        _ => panic!("Expected Text content part"),
    }
}

/// Test JSON array format with image content
#[test]
fn test_json_array_format_with_image() {
    let json_with_image = r#"{
        "role": "user",
        "content": [
            {"type": "text", "text": "Analyze this"},
            {"type": "image_url", "image_url": {"url": "https://test.com/img.png"}}
        ]
    }"#;
    
    let message: Message = serde_json::from_str(json_with_image).expect("Failed to deserialize");
    assert_eq!(message.content.len(), 2);
    
    match &message.content[0] {
        ContentPart::Text(text) => assert_eq!(text, "Analyze this"),
        _ => panic!("Expected Text content part"),
    }
    
    match &message.content[1] {
        ContentPart::ImageUrl { url } => assert_eq!(url, "https://test.com/img.png"),
        _ => panic!("Expected ImageUrl content part"),
    }
}