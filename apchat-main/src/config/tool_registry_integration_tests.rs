#![cfg(test)]

use apchat_toolcore::ToolRegistry;

/// Integration test to verify that llm_oneshot tool is properly registered
#[test]
fn test_llm_oneshot_tool_registration() {
    // Import the initialize_tool_registry function
    use crate::config::initialize_tool_registry;
    
    // Initialize the tool registry
    let registry = initialize_tool_registry(false);
    
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