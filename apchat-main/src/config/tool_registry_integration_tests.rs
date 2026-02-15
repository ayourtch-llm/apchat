#![cfg(test)]

use apchat_toolcore::ToolRegistry;

/// Integration test to verify that llm_oneshot tool is properly registered
#[test]
fn test_llm_oneshot_tool_registration() {
    // Import the initialize_tool_registry function
    use crate::config::{initialize_tool_registry, FeatureFlags};
    
    // Initialize the tool registry
    let registry = initialize_tool_registry(&FeatureFlags::default());
    
    // Verify that the llm_oneshot tool is registered
    assert!(registry.has_tool("llm_oneshot"), "llm_oneshot tool should be registered");
    
    // Verify that the tool is in the correct categories
    let llm_tools = registry.get_tools_by_category("llm");
    let ai_tools = registry.get_tools_by_category("ai");
    let model_tools = registry.get_tools_by_category("model");
    
    // Check that llm_oneshot is in all three categories
    let llm_oneshot_names: Vec<&str> = llm_tools.iter().map(|t| t.name()).collect();
    assert!(llm_oneshot_names.contains(&"llm_oneshot"), "llm_oneshot should be in 'llm' category");
    
    let ai_tool_names: Vec<&str> = ai_tools.iter().map(|t| t.name()).collect();
    assert!(ai_tool_names.contains(&"llm_oneshot"), "llm_oneshot should be in 'ai' category");
    
    let model_tool_names: Vec<&str> = model_tools.iter().map(|t| t.name()).collect();
    assert!(model_tool_names.contains(&"llm_oneshot"), "llm_oneshot should be in 'model' category");
    
    // Verify tool properties
    if let Some(tool) = registry.get_tool("llm_oneshot") {
        assert_eq!(tool.name(), "llm_oneshot");
        assert!(!tool.description().is_empty());
        
        // Check that the tool has the expected parameters
        let params = tool.parameters();
        assert!(params.contains_key("model_color"), "Tool should have 'model_color' parameter");
        assert!(params.contains_key("instruction"), "Tool should have 'instruction' parameter");
        assert!(params.contains_key("file_path"), "Tool should have 'file_path' parameter");
    } else {
        panic!("llm_oneshot tool should exist in registry");
    }
}

/// Test that feature flags correctly gate tool registration
#[test]
fn test_feature_flags_gate_tools() {
    use crate::config::{initialize_tool_registry, FeatureFlags};

    // Default flags: metacog and self_regulate disabled
    let registry = initialize_tool_registry(&FeatureFlags::default());
    assert!(!registry.has_tool("become"), "become should not be registered by default");
    assert!(!registry.has_tool("drugs"), "drugs should not be registered by default");
    assert!(!registry.has_tool("ritual"), "ritual should not be registered by default");
    assert!(!registry.has_tool("self_regulate"), "self_regulate should not be registered by default");

    // Enable metacog tools
    let registry = initialize_tool_registry(&FeatureFlags {
        metacog_tools: true,
        ..FeatureFlags::default()
    });
    assert!(registry.has_tool("become"), "become should be registered when metacog_tools is true");
    assert!(registry.has_tool("drugs"), "drugs should be registered when metacog_tools is true");
    assert!(registry.has_tool("ritual"), "ritual should be registered when metacog_tools is true");

    // Enable self-regulate
    let registry = initialize_tool_registry(&FeatureFlags {
        self_regulate: true,
        ..FeatureFlags::default()
    });
    assert!(registry.has_tool("self_regulate"), "self_regulate should be registered when self_regulate is true");
}