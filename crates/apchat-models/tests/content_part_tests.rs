use apchat_models::types::{Message, ContentPart};
use serde_json;

#[test]
fn test_content_part_text_serialization() {
    let part = ContentPart::Text("Hello".to_string());
    let json = serde_json::to_string(&part).unwrap();
    assert_eq!(json, r#"{"type":"text","text":"Hello"}"#);
}

#[test]
fn test_content_part_image_serialization() {
    let part = ContentPart::ImageUrl {
        url: "data:image/jpeg;base64,abc123".to_string(),
    };
    let json = serde_json::to_string(&part).unwrap();
    assert_eq!(json, r#"{"type":"image_url","image_url":{"url":"data:image/jpeg;base64,abc123"}}"#);
}

#[test]
fn test_message_with_text_content() {
    let msg = Message {
        role: "user".to_string(),
        content: vec![ContentPart::Text("Hello".to_string())],
        ..Default::default()
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"role\":\"user\""));
    assert!(json.contains("\"type\":\"text\""));
}

#[test]
fn test_message_with_image_content() {
    let msg = Message {
        role: "user".to_string(),
        content: vec![
            ContentPart::Text("Describe this".to_string()),
            ContentPart::ImageUrl { url: "data:image/png;base64,xyz".to_string() },
        ],
        ..Default::default()
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"image_url\""));
}

#[test]
fn test_message_backward_compatible_string() {
    let json = r#"{"role": "user", "content": "Hello world"}"#;
    let msg: Message = serde_json::from_str(json).unwrap();
    assert_eq!(msg.role, "user");
    assert!(!msg.content.is_empty());
}