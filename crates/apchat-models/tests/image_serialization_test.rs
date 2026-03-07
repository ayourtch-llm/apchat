//! Test image serialization format

use apchat_models::types::ContentPart;

#[test]
fn test_image_url_serialization_format() {
    let part = ContentPart::ImageUrl {
        url: "data:image/jpeg;base64,abc123".to_string(),
    };
    
    // Serialize to JSON
    let json = serde_json::to_string(&part).unwrap();
    println!("Serialized: {}", json);
    
    // Verify format: {"type":"image_url","url":"..."}
    assert!(json.contains(r#""type":"image_url""#));
    assert!(json.contains(r#""url":"data:image/jpeg;base64,abc123""#));
    
    // Verify it does NOT contain the old nested format
    assert!(!json.contains(r#"image_url":{"url"#));
    
    // Test deserialization of new format
    let deserialized: ContentPart = serde_json::from_str(&json).unwrap();
    match deserialized {
        ContentPart::ImageUrl { url } => assert_eq!(url, "data:image/jpeg;base64,abc123"),
        _ => panic!("Expected ImageUrl"),
    }
    
    // Test backward compatibility with old nested format
    let old_format = r#"{"type":"image_url","image_url":{"url":"data:image/jpeg;base64,old123"}}"#;
    let old_deserialized: ContentPart = serde_json::from_str(old_format).unwrap();
    match old_deserialized {
        ContentPart::ImageUrl { url } => assert_eq!(url, "data:image/jpeg;base64,old123"),
        _ => panic!("Expected ImageUrl from old format"),
    }
    
    println!("All tests passed!");
}