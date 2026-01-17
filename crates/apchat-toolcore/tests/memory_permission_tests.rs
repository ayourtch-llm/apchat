use apchat_toolcore::tool_context::ToolContext;
use apchat_policy::PolicyManager;
use tempfile::TempDir;

#[tokio::test]
async fn test_memory_operations_auto_approve_integration() {
    use apchat_policy::{ActionType, Decision};
    
    let temp_dir = TempDir::new().unwrap();
    let work_dir = temp_dir.path().to_path_buf();
    let policy_manager = PolicyManager::new();
    
    // Create a context that would normally prompt (interactive mode)
    let context = ToolContext::new(work_dir, "test_session".to_string(), policy_manager);
    
    // Verify that memory operations auto-approve even in interactive mode
    let memory_actions = vec![
        ActionType::MemoryStore,
        ActionType::MemoryQuery,
        ActionType::MemoryUpdate,
        ActionType::MemoryList,
        ActionType::MemoryDelete,
    ];
    
    for action in memory_actions {
        let target = "test_memory";
        let prompt = "This should auto-approve";
        
        let result = context.check_permission(action.clone(), target, prompt);
        assert!(result.is_ok(), "Permission check for {:?} failed", action);
        
        let (approved, reason) = result.unwrap();
        assert!(approved, "Memory operation {:?} should auto-approve", action);
        assert!(reason.is_none(), "Memory operation {:?} should not have rejection reason", action);
    }
    
    // Verify that non-memory operations still behave normally (would prompt in interactive mode)
    // In non-interactive mode, they auto-approve
    let context_non_interactive = context.with_non_interactive(true);
    let action = ActionType::FileRead;
    let target = "/tmp/test.txt";
    let prompt = "Read file";
    
    let result = context_non_interactive.check_permission(action, target, prompt);
    assert!(result.is_ok());
    let (approved, reason) = result.unwrap();
    assert!(approved, "Non-memory operations should auto-approve in non-interactive mode");
}

#[tokio::test]
async fn test_memory_operations_dont_prompt() {
    use apchat_policy::ActionType;
    
    let temp_dir = TempDir::new().unwrap();
    let work_dir = temp_dir.path().to_path_buf();
    let policy_manager = PolicyManager::new();
    
    // Create a context in interactive mode (non_interactive = false)
    let context = ToolContext::new(work_dir, "test_session".to_string(), policy_manager);
    
    // Memory operations should not prompt even in interactive mode
    let action = ActionType::MemoryStore;
    let target = "user_conversation";
    let prompt = "Store memory?";
    
    // This should complete without requiring user input
    let result = context.check_permission(action, target, prompt);
    assert!(result.is_ok());
    
    let (approved, reason) = result.unwrap();
    assert!(approved, "Memory operations should auto-approve");
    assert!(reason.is_none(), "Memory operations should not require rejection reasons");
}
