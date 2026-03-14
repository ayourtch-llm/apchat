#![cfg(test)]

use apchat_toolcore::ToolRegistry;

/// Integration test to verify that llm_oneshot tool is properly registered
#[test]
fn test_llm_oneshot_tool_registration() {
    // Import the initialize_tool_registry function
    use crate::config::{initialize_tool_registry, FeatureFlags};
    use crate::terminal::TerminalManager;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    // Initialize the tool registry
    let terminal_manager = Arc::new(Mutex::new(TerminalManager::new(std::path::PathBuf::from("/tmp"))));
    let registry = initialize_tool_registry(
        &FeatureFlags::default(),
        terminal_manager,
        std::path::PathBuf::from("/tmp")
    );
    
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
    use crate::terminal::TerminalManager;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Setup terminal manager for all calls
    let terminal_manager = Arc::new(Mutex::new(TerminalManager::new(std::path::PathBuf::from("/tmp"))));
    let work_dir = std::path::PathBuf::from("/tmp");

    // Default flags: metacog and self_regulate disabled
    let registry = initialize_tool_registry(&FeatureFlags::default(), terminal_manager.clone(), work_dir.clone());
    assert!(!registry.has_tool("become"), "become should not be registered by default");
    assert!(!registry.has_tool("drugs"), "drugs should not be registered by default");
    assert!(!registry.has_tool("ritual"), "ritual should not be registered by default");
    assert!(!registry.has_tool("self_regulate"), "self_regulate should not be registered by default");

    // Enable metacog tools
    let registry = initialize_tool_registry(&FeatureFlags {
        metacog_tools: true,
        ..FeatureFlags::default()
    }, terminal_manager.clone(), work_dir.clone());
    assert!(registry.has_tool("become"), "become should be registered when metacog_tools is true");
    assert!(registry.has_tool("drugs"), "drugs should be registered when metacog_tools is true");
    assert!(registry.has_tool("ritual"), "ritual should be registered when metacog_tools is true");

    // Enable self-regulate
    let registry = initialize_tool_registry(&FeatureFlags {
        self_regulate: true,
        ..FeatureFlags::default()
    }, terminal_manager.clone(), work_dir.clone());
    assert!(registry.has_tool("self_regulate"), "self_regulate should be registered when self_regulate is true");

    // Default: self_edit tools disabled
    assert!(!registry.has_tool("delete_items"), "delete_items should not be registered by default");
    assert!(!registry.has_tool("edit_item"), "edit_item should not be registered by default");

    // Enable self-edit
    let registry = initialize_tool_registry(&FeatureFlags {
        self_edit: true,
        ..FeatureFlags::default()
    }, terminal_manager.clone(), work_dir.clone());
    assert!(registry.has_tool("delete_items"), "delete_items should be registered when self_edit is true");
    assert!(registry.has_tool("edit_item"), "edit_item should be registered when self_edit is true");

    // Default: diff_fuzz tool disabled
    let registry = initialize_tool_registry(&FeatureFlags::default(), terminal_manager.clone(), work_dir.clone());
    assert!(!registry.has_tool("diff_fuzz"), "diff_fuzz should not be registered by default");

    // Enable diff_fuzz
    let registry = initialize_tool_registry(&FeatureFlags {
        diff_fuzz: true,
        ..FeatureFlags::default()
    }, terminal_manager.clone(), work_dir.clone());
    assert!(registry.has_tool("diff_fuzz"), "diff_fuzz should be registered when diff_fuzz is true");

    // Default: web_search (searxng) tool disabled
    let registry = initialize_tool_registry(&FeatureFlags::default(), terminal_manager.clone(), work_dir.clone());
    assert!(!registry.has_tool("web_search"), "web_search should not be registered by default");

    // Enable searxng
    let registry = initialize_tool_registry(&FeatureFlags {
        searxng_url: Some("http://localhost:8888".to_string()),
        ..FeatureFlags::default()
    }, terminal_manager.clone(), work_dir.clone());
    assert!(registry.has_tool("web_search"), "web_search should be registered when searxng_url is set");

    // Default: python_sandbox tool disabled
    let registry = initialize_tool_registry(&FeatureFlags::default(), terminal_manager.clone(), work_dir.clone());
    assert!(!registry.has_tool("python_sandbox"), "python_sandbox should not be registered by default");

    // Enable python_sandbox (only effective when compiled with python-sandbox feature)
    #[cfg(feature = "python-sandbox")]
    {
        let registry = initialize_tool_registry(&FeatureFlags {
            python_sandbox: true,
            ..FeatureFlags::default()
        }, terminal_manager.clone(), work_dir.clone());
        assert!(registry.has_tool("python_sandbox"), "python_sandbox should be registered when python_sandbox is true");
    }

    // Test pty_tools flag: by default, PTY tools are NOT registered (PTY tools disabled by default)
    let registry = initialize_tool_registry(&FeatureFlags::default(), terminal_manager.clone(), work_dir.clone());
    assert!(!registry.has_tool("pty_launch"), "pty_launch should NOT be registered by default");
    assert!(!registry.has_tool("pty_send_keys"), "pty_send_keys should NOT be registered by default");
    assert!(!registry.has_tool("pty_get_screen"), "pty_get_screen should NOT be registered by default");

    // PTY tools enabled when explicitly set to true
    let registry = initialize_tool_registry(&FeatureFlags {
        pty_tools: true,
        ..FeatureFlags::default()
    }, terminal_manager.clone(), work_dir.clone());
    assert!(registry.has_tool("pty_launch"), "pty_launch should be registered when pty_tools is true");
    assert!(registry.has_tool("pty_send_keys"), "pty_send_keys should be registered when pty_tools is true");
    assert!(registry.has_tool("pty_get_screen"), "pty_get_screen should be registered when pty_tools is true");

    // PTY tools disabled when explicitly set to false
    let registry = initialize_tool_registry(&FeatureFlags {
        pty_tools: false,
        ..FeatureFlags::default()
    }, terminal_manager.clone(), work_dir.clone());
    assert!(!registry.has_tool("pty_launch"), "pty_launch should NOT be registered when pty_tools is false");
    assert!(!registry.has_tool("pty_send_keys"), "pty_send_keys should NOT be registered when pty_tools is false");
    assert!(!registry.has_tool("pty_get_screen"), "pty_get_screen should NOT be registered when pty_tools is false");
}