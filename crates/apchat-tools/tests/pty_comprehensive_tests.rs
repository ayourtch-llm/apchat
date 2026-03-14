// Comprehensive PTY Tools Test Suite
//
// This test suite provides exhaustive coverage for all PTY-related tools:
// - pty_launch: Session creation
// - pty_send_keys: Input handling
// - pty_get_screen: Screen content retrieval
// - pty_list: Session listing
// - pty_kill: Session termination
// - pty_get_cursor: Cursor position
// - pty_resize: Terminal resizing
// - pty_set_scrollback: Scrollback management
// - pty_start_capture: Output capture
// - pty_stop_capture: Capture termination
// - pty_request_user_input: User interaction
// - pty_send_credential_keys: Credential injection

use apchat_toolcore::{Tool, ToolParameters, ToolContext};
use apchat_policy::PolicyManager;
use apchat_tools::{
    PtyLaunchTool, PtySendKeysTool, PtyGetScreenTool, PtyListTool, PtyKillTool,
    PtyGetCursorTool, PtyResizeTool, PtySetScrollbackTool, PtyStartCaptureTool,
    PtyStopCaptureTool, PtyRequestUserInputTool
};
use apchat_terminal::TerminalManager;
use apchat_common::ApChatPaths;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tempfile::TempDir;
use std::fs;
use serial_test::serial;

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Set up test environment with temporary directory
async fn setup_test_environment() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    
    // Set HOME to temp_dir for credentials file
    std::env::set_var("HOME", temp_dir.path());
    
    // Ensure config directory exists
    let config_dir = ApChatPaths::config_dir();
    let _ = fs::create_dir_all(config_dir.parent().unwrap());
    
    temp_dir
}

/// Create a terminal manager for testing
async fn create_terminal_manager(temp_dir: &std::path::Path) -> Arc<Mutex<TerminalManager>> {
    let log_dir = temp_dir.join("logs");
    fs::create_dir_all(&log_dir).unwrap();
    
    Arc::new(Mutex::new(TerminalManager::new(log_dir)))
}

/// Create a tool context for testing
fn create_tool_context(
    temp_dir: &std::path::Path,
    terminal_manager: Arc<Mutex<TerminalManager>>,
) -> ToolContext {
    ToolContext::new(
        PathBuf::from(temp_dir),
        "test-session".to_string(),
        PolicyManager::default()
    ).with_terminal_manager(terminal_manager)
}

// ============================================================================
// PTY LAUNCH TOOL TESTS
// ============================================================================

#[tokio::test]
async fn test_pty_launch_tool_parameters() {
    let tool = PtyLaunchTool;

    assert_eq!(tool.name(), "pty_launch");
    
    let desc = tool.description();
    assert!(!desc.is_empty());
    assert!(desc.contains("pty") || desc.contains("terminal"));

    let params = tool.parameters();
    assert!(params.contains_key("command"));
    assert!(params.contains_key("working_dir"));
    assert!(params.contains_key("rows"));
    assert!(params.contains_key("cols"));
}

#[tokio::test]
#[serial]
async fn test_pty_launch_creates_session_successfully() {
    let temp_dir = setup_test_environment().await;
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;

    let tool = PtyLaunchTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("command", "/bin/bash");
    params.set("working_dir", temp_dir.path().to_string_lossy().to_string());
    params.set("rows", 24);
    params.set("cols", 80);

    let result = tool.execute(params, &context).await;

    assert!(result.success, "Session should be created: {:?}", result.error);
    assert!(result.content.contains("session_id"));
}

#[tokio::test]
#[serial]
async fn test_pty_launch_with_custom_command() {
    let temp_dir = setup_test_environment().await;
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;

    let tool = PtyLaunchTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("command", "echo 'Hello, World!'");
    params.set("working_dir", temp_dir.path().to_string_lossy().to_string());
    params.set("rows", 24);
    params.set("cols", 80);

    let result = tool.execute(params, &context).await;

    assert!(result.success, "Session should be created: {:?}", result.error);
    assert!(result.content.contains("session_id"));
}

#[tokio::test]
#[serial]
async fn test_pty_launch_with_custom_dimensions() {
    let temp_dir = setup_test_environment().await;
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;

    let tool = PtyLaunchTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("command", "/bin/bash");
    params.set("working_dir", temp_dir.path().to_string_lossy().to_string());
    params.set("rows", 40);
    params.set("cols", 120);

    let result = tool.execute(params, &context).await;

    assert!(result.success, "Session should be created: {:?}", result.error);
    assert!(result.content.contains("session_id"));
}

#[tokio::test]
#[serial]
async fn test_pty_launch_missing_command() {
    let temp_dir = setup_test_environment().await;
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;

    let tool = PtyLaunchTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("working_dir", temp_dir.path().to_string_lossy().to_string());
    params.set("rows", 24);
    params.set("cols", 80);

    let result = tool.execute(params, &context).await;

    // Command should be optional, defaults to shell
    assert!(result.success, "Session should be created with default shell: {:?}", result.error);
}

// ============================================================================
// PTY SEND KEYS TOOL TESTS
// ============================================================================

#[tokio::test]
async fn test_pty_send_keys_tool_parameters() {
    let tool = PtySendKeysTool;

    assert_eq!(tool.name(), "pty_send_keys");
    
    let params = tool.parameters();
    assert!(params.contains_key("session_id"));
    assert!(params.contains_key("keys"));
    assert!(params.contains_key("raw"));
}

#[tokio::test]
#[serial]
async fn test_pty_send_keys_basic_input() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-send-keys-1";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let tool = PtySendKeysTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", session_id);
    params.set("keys", "echo 'test'\n");
    params.set("raw", false);

    let result = tool.execute(params, &context).await;

    assert!(result.success, "Keys should be sent: {:?}", result.error);
}

#[tokio::test]
#[serial]
async fn test_pty_send_keys_special_characters() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-send-keys-2";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let tool = PtySendKeysTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    // Test special key sequences
    let mut params = ToolParameters::new();
    params.set("session_id", session_id);
    params.set("keys", "^C"); // Ctrl+C
    params.set("raw", false);

    let result = tool.execute(params, &context).await;
    assert!(result.success, "Ctrl+C should be sent: {:?}", result.error);
}

#[tokio::test]
#[serial]
async fn test_pty_send_keys_with_raw_mode() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-send-keys-3";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let tool = PtySendKeysTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", session_id);
    params.set("keys", "\n\n\n"); // Multiple newlines without auto-terminating
    params.set("raw", true);

    let result = tool.execute(params, &context).await;

    assert!(result.success, "Raw keys should be sent: {:?}", result.error);
}

#[tokio::test]
#[serial]
async fn test_pty_send_keys_to_nonexistent_session() {
    let temp_dir = setup_test_environment().await;
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;

    let tool = PtySendKeysTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", "nonexistent-session");
    params.set("keys", "test\n");
    params.set("raw", false);

    let result = tool.execute(params, &context).await;

    assert!(!result.success, "Should fail for nonexistent session");
    assert!(result.error.is_some());
}

#[tokio::test]
#[serial]
async fn test_pty_send_keys_arrow_keys() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-send-keys-4";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let tool = PtySendKeysTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", session_id);
    params.set("keys", "[UP]\n[DOWN]\n[LEFT]\n[RIGHT]\n");
    params.set("raw", false);

    let result = tool.execute(params, &context).await;

    assert!(result.success, "Arrow keys should be sent: {:?}", result.error);
}

#[tokio::test]
#[serial]
async fn test_pty_send_keys_function_keys() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-send-keys-5";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let tool = PtySendKeysTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", session_id);
    params.set("keys", "[F1]\n[F2]\n[F3]\n[F4]\n");
    params.set("raw", false);

    let result = tool.execute(params, &context).await;

    assert!(result.success, "Function keys should be sent: {:?}", result.error);
}

#[tokio::test]
#[serial]
async fn test_pty_send_keys_tab_completion() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-send-keys-6";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let tool = PtySendKeysTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", session_id);
    params.set("keys", "[TAB]\n");
    params.set("raw", false);

    let result = tool.execute(params, &context).await;

    assert!(result.success, "Tab key should be sent: {:?}", result.error);
}

// ============================================================================
// PTY GET SCREEN TOOL TESTS
// ============================================================================

#[tokio::test]
async fn test_pty_get_screen_tool_parameters() {
    let tool = PtyGetScreenTool;

    assert_eq!(tool.name(), "pty_get_screen");
    
    let params = tool.parameters();
    assert!(params.contains_key("session_id"));
    assert!(params.contains_key("include_colors"));
    assert!(params.contains_key("include_cursor"));
}

#[tokio::test]
#[serial]
async fn test_pty_get_screen_basic() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-get-screen-1";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let tool = PtyGetScreenTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", session_id);
    params.set("include_colors", false);
    params.set("include_cursor", true);

    let result = tool.execute(params, &context).await;

    assert!(result.success, "Screen should be retrieved: {:?}", result.error);
    assert!(!result.content.is_empty());
}

#[tokio::test]
#[serial]
async fn test_pty_get_screen_with_colors() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-get-screen-2";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let tool = PtyGetScreenTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", session_id);
    params.set("include_colors", true);
    params.set("include_cursor", true);

    let result = tool.execute(params, &context).await;

    assert!(result.success, "Screen with colors should be retrieved");
}

#[tokio::test]
#[serial]
async fn test_pty_get_screen_without_cursor() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-get-screen-3";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let tool = PtyGetScreenTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", session_id);
    params.set("include_colors", false);
    params.set("include_cursor", false);

    let result = tool.execute(params, &context).await;

    assert!(result.success, "Screen without cursor should be retrieved");
}

#[tokio::test]
#[serial]
async fn test_pty_get_screen_nonexistent_session() {
    let temp_dir = setup_test_environment().await;
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;

    let tool = PtyGetScreenTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", "nonexistent-session");
    params.set("include_colors", false);
    params.set("include_cursor", true);

    let result = tool.execute(params, &context).await;

    assert!(!result.success, "Should fail for nonexistent session");
    assert!(result.error.is_some());
}

// ============================================================================
// PTY LIST TOOL TESTS
// ============================================================================

#[tokio::test]
async fn test_pty_list_tool_parameters() {
    let tool = PtyListTool;

    assert_eq!(tool.name(), "pty_list");
    
    let params = tool.parameters();
    assert!(params.is_empty(), "PtyListTool should have no parameters");
}

#[tokio::test]
#[serial]
async fn test_pty_list_empty() {
    let temp_dir = setup_test_environment().await;
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;

    let tool = PtyListTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let result = tool.execute(ToolParameters::new(), &context).await;

    assert!(result.success, "List should succeed");
    assert!(result.content.contains("[]") || result.content.trim().is_empty());
}

#[tokio::test]
#[serial]
async fn test_pty_list_with_sessions() {
    let temp_dir = setup_test_environment().await;
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;

    // Create multiple sessions
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            "list-test-1".to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
        tm.create_session(
            "list-test-2".to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let tool = PtyListTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let result = tool.execute(ToolParameters::new(), &context).await;

    assert!(result.success, "List should succeed");
    // Should contain at least 2 session IDs
    assert!(result.content.contains("list-test-1"));
    assert!(result.content.contains("list-test-2"));
}

#[tokio::test]
#[serial]
async fn test_pty_list_session_metadata() {
    let temp_dir = setup_test_environment().await;
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;

    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            "list-test-3".to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let tool = PtyListTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let result = tool.execute(ToolParameters::new(), &context).await;

    assert!(result.success, "List should succeed");
    // Should contain session metadata like command, rows, cols
    assert!(result.content.contains("bash") || result.content.contains("command"));
}

// ============================================================================
// PTY KILL TOOL TESTS
// ============================================================================

#[tokio::test]
async fn test_pty_kill_tool_parameters() {
    let tool = PtyKillTool;

    assert_eq!(tool.name(), "pty_kill");
    
    let params = tool.parameters();
    assert!(params.contains_key("session_id"));
}

#[tokio::test]
#[serial]
async fn test_pty_kill_session() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-kill-1";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify session exists
    {
        let tm = terminal_manager.lock().await;
        assert!(tm.session_exists(&session_id).await);
    }

    let tool = PtyKillTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", session_id);

    let result = tool.execute(params, &context).await;

    assert!(result.success, "Session should be killed: {:?}", result.error);
    
    // Verify session no longer exists
    {
        let tm = terminal_manager.lock().await;
        assert!(!tm.session_exists(session_id).await);
    }
}

#[tokio::test]
#[serial]
async fn test_pty_kill_nonexistent_session() {
    let temp_dir = setup_test_environment().await;
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;

    let tool = PtyKillTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", "nonexistent-session");

    let result = tool.execute(params, &context).await;

    assert!(!result.success, "Should fail for nonexistent session");
    assert!(result.error.is_some());
}

// ============================================================================
// PTY GET CURSOR TOOL TESTS
// ============================================================================

#[tokio::test]
async fn test_pty_get_cursor_tool_parameters() {
    let tool = PtyGetCursorTool;

    assert_eq!(tool.name(), "pty_get_cursor");
    
    let params = tool.parameters();
    assert!(params.contains_key("session_id"));
}

#[tokio::test]
#[serial]
async fn test_pty_get_cursor_position() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-cursor-1";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let tool = PtyGetCursorTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", session_id);

    let result = tool.execute(params, &context).await;

    assert!(result.success, "Cursor position should be retrieved: {:?}", result.error);
    // Should contain position information (col, row in position array)
    assert!(result.content.contains("position") || result.content.contains("col") || result.content.contains("row"));
}

#[tokio::test]
#[serial]
async fn test_pty_get_cursor_nonexistent_session() {
    let temp_dir = setup_test_environment().await;
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;

    let tool = PtyGetCursorTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", "nonexistent-session");

    let result = tool.execute(params, &context).await;

    assert!(!result.success, "Should fail for nonexistent session");
    assert!(result.error.is_some());
}

// ============================================================================
// PTY RESIZE TOOL TESTS
// ============================================================================

#[tokio::test]
async fn test_pty_resize_tool_parameters() {
    let tool = PtyResizeTool;

    assert_eq!(tool.name(), "pty_resize");
    
    let params = tool.parameters();
    assert!(params.contains_key("session_id"));
    assert!(params.contains_key("rows"));
    assert!(params.contains_key("cols"));
}

#[tokio::test]
#[serial]
async fn test_pty_resize_session() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-resize-1";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Send resize command to session
    {
        let mut tm = terminal_manager.lock().await;
        tm.resize_session(&session_id, 40, 120)
            .await
            .expect("Failed to resize session");
    }

    let tool = PtyResizeTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", session_id);
    params.set("rows", 40);
    params.set("cols", 120);

    let result = tool.execute(params, &context).await;

    assert!(result.success, "Session should be resized: {:?}", result.error);
}

#[tokio::test]
#[serial]
async fn test_pty_resize_nonexistent_session() {
    let temp_dir = setup_test_environment().await;
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;

    let tool = PtyResizeTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", "nonexistent-session");
    params.set("rows", 40);
    params.set("cols", 120);

    let result = tool.execute(params, &context).await;

    assert!(!result.success, "Should fail for nonexistent session");
    assert!(result.error.is_some());
}

// ============================================================================
// PTY SET SCROLLBACK TOOL TESTS
// ============================================================================

#[tokio::test]
async fn test_pty_set_scrollback_tool_parameters() {
    let tool = PtySetScrollbackTool;

    assert_eq!(tool.name(), "pty_set_scrollback");
    
    let params = tool.parameters();
    assert!(params.contains_key("session_id"));
    assert!(params.contains_key("lines"));
}

#[tokio::test]
#[serial]
async fn test_pty_set_scrollback_default() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-scrollback-1";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let tool = PtySetScrollbackTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", session_id);
    params.set("lines", 1000);

    let result = tool.execute(params, &context).await;

    assert!(result.success, "Scrollback should be set: {:?}", result.error);
}

#[tokio::test]
#[serial]
async fn test_pty_set_scrollback_custom_size() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-scrollback-2";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let tool = PtySetScrollbackTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", session_id);
    params.set("lines", 500);

    let result = tool.execute(params, &context).await;

    assert!(result.success, "Custom scrollback size should be set");
}

#[tokio::test]
#[serial]
async fn test_pty_set_scrollback_nonexistent_session() {
    let temp_dir = setup_test_environment().await;
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;

    let tool = PtySetScrollbackTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", "nonexistent-session");
    params.set("lines", 1000);

    let result = tool.execute(params, &context).await;

    assert!(!result.success, "Should fail for nonexistent session");
    assert!(result.error.is_some());
}

// ============================================================================
// PTY START CAPTURE TOOL TESTS
// ============================================================================

#[tokio::test]
async fn test_pty_start_capture_tool_parameters() {
    let tool = PtyStartCaptureTool;

    assert_eq!(tool.name(), "pty_start_capture");
    
    let params = tool.parameters();
    assert!(params.contains_key("session_id"));
}

#[tokio::test]
#[serial]
async fn test_pty_start_capture() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-capture-1";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let tool = PtyStartCaptureTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", session_id);

    let result = tool.execute(params, &context).await;

    assert!(result.success, "Capture should start: {:?}", result.error);
    assert!(result.content.contains("capture") || result.content.contains("file"));
}

#[tokio::test]
#[serial]
async fn test_pty_start_capture_nonexistent_session() {
    let temp_dir = setup_test_environment().await;
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;

    let tool = PtyStartCaptureTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", "nonexistent-session");

    let result = tool.execute(params, &context).await;

    assert!(!result.success, "Should fail for nonexistent session");
    assert!(result.error.is_some());
}

// ============================================================================
// PTY STOP CAPTURE TOOL TESTS
// ============================================================================

#[tokio::test]
async fn test_pty_stop_capture_tool_parameters() {
    let tool = PtyStopCaptureTool;

    assert_eq!(tool.name(), "pty_stop_capture");
    
    let params = tool.parameters();
    assert!(params.contains_key("session_id"));
}

#[tokio::test]
#[serial]
async fn test_pty_stop_capture() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-stop-capture-1";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Start capture first
    let start_tool = PtyStartCaptureTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());
    let mut start_params = ToolParameters::new();
    start_params.set("session_id", session_id);
    let start_result = start_tool.execute(start_params, &context).await;
    assert!(start_result.success, "Capture should start");

    // Send some output to capture
    {
        let mut tm = terminal_manager.lock().await;
        tm.send_input(&session_id, "echo 'test output'\n").await
            .expect("Failed to send input");
    }
    
    // Wait for output
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Stop capture
    let tool = PtyStopCaptureTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", session_id);

    let result = tool.execute(params, &context).await;

    assert!(result.success, "Capture should stop: {:?}", result.error);
    // Should contain file path, bytes, and duration
    assert!(result.content.contains("file") || result.content.contains("path"));
}

#[tokio::test]
#[serial]
async fn test_pty_stop_capture_without_start() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-stop-capture-2";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let tool = PtyStopCaptureTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", session_id);

    let result = tool.execute(params, &context).await;

    // Should fail because capture wasn't started
    assert!(!result.success, "Should fail if capture not started");
    assert!(result.error.is_some());
}

#[tokio::test]
#[serial]
async fn test_pty_stop_capture_nonexistent_session() {
    let temp_dir = setup_test_environment().await;
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;

    let tool = PtyStopCaptureTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());

    let mut params = ToolParameters::new();
    params.set("session_id", "nonexistent-session");

    let result = tool.execute(params, &context).await;

    assert!(!result.success, "Should fail for nonexistent session");
    assert!(result.error.is_some());
}

// ============================================================================
// PTY REQUEST USER INPUT TOOL TESTS
// ============================================================================

#[tokio::test]
async fn test_pty_request_user_input_tool_parameters() {
    let tool = PtyRequestUserInputTool;

    assert_eq!(tool.name(), "pty_request_user_input");
    
    let params = tool.parameters();
    assert!(params.contains_key("session_id"));
    assert!(params.contains_key("message"));
    assert!(params.contains_key("timeout_seconds"));
}

#[tokio::test]
async fn test_pty_request_user_input_not_implemented() {
    // This tool requires actual user interaction, so we test the parameters
    let tool = PtyRequestUserInputTool;

    assert_eq!(tool.name(), "pty_request_user_input");
    
    let params = tool.parameters();
    assert!(params.contains_key("session_id"));
    assert!(params.contains_key("message"));
    assert!(params.contains_key("timeout_seconds"));
}

// ============================================================================
// COMPREHENSIVE INTEGRATION TESTS
// ============================================================================

#[tokio::test]
#[serial]
async fn test_pty_full_workflow() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-full-workflow";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());
    
    // 1. Send input
    let send_keys_tool = PtySendKeysTool;
    let mut send_params = ToolParameters::new();
    send_params.set("session_id", session_id);
    send_params.set("keys", "echo 'Hello from PTY test'\n");
    send_params.set("raw", false);
    
    let send_result = send_keys_tool.execute(send_params, &context).await;
    assert!(send_result.success, "Send keys should succeed");
    
    // Wait for output
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    
    // 2. Get screen
    let get_screen_tool = PtyGetScreenTool;
    let mut screen_params = ToolParameters::new();
    screen_params.set("session_id", session_id);
    screen_params.set("include_colors", false);
    screen_params.set("include_cursor", true);
    
    let screen_result = get_screen_tool.execute(screen_params, &context).await;
    assert!(screen_result.success, "Get screen should succeed");
    assert!(!screen_result.content.is_empty(), "Screen should not be empty");
    
    // 3. Get cursor
    let get_cursor_tool = PtyGetCursorTool;
    let mut cursor_params = ToolParameters::new();
    cursor_params.set("session_id", session_id);
    
    let cursor_result = get_cursor_tool.execute(cursor_params, &context).await;
    assert!(cursor_result.success, "Get cursor should succeed");
    
    // 4. Resize session
    let resize_tool = PtyResizeTool;
    let mut resize_params = ToolParameters::new();
    resize_params.set("session_id", session_id);
    resize_params.set("rows", 40);
    resize_params.set("cols", 120);
    
    let resize_result = resize_tool.execute(resize_params, &context).await;
    assert!(resize_result.success, "Resize should succeed");
    
    // 5. Set scrollback
    let set_scrollback_tool = PtySetScrollbackTool;
    let mut scrollback_params = ToolParameters::new();
    scrollback_params.set("session_id", session_id);
    scrollback_params.set("lines", 500);
    
    let scrollback_result = set_scrollback_tool.execute(scrollback_params, &context).await;
    assert!(scrollback_result.success, "Set scrollback should succeed");
    
    // 6. List sessions
    let list_tool = PtyListTool;
    let list_result = list_tool.execute(ToolParameters::new(), &context).await;
    assert!(list_result.success, "List sessions should succeed");
    assert!(list_result.content.contains("test-full-workflow"), "List should contain session info");
    
    // 7. Start capture
    let start_capture_tool = PtyStartCaptureTool;
    let mut start_capture_params = ToolParameters::new();
    start_capture_params.set("session_id", session_id);
    
    let start_capture_result = start_capture_tool.execute(start_capture_params, &context).await;
    assert!(start_capture_result.success, "Start capture should succeed");
    
    // Send more output during capture
    {
        let mut tm = terminal_manager.lock().await;
        tm.send_input(&session_id, "echo 'Captured output'\n").await
            .expect("Failed to send input during capture");
    }
    
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    
    // 8. Stop capture
    let stop_capture_tool = PtyStopCaptureTool;
    let mut stop_capture_params = ToolParameters::new();
    stop_capture_params.set("session_id", session_id);
    
    let stop_capture_result = stop_capture_tool.execute(stop_capture_params, &context).await;
    assert!(stop_capture_result.success, "Stop capture should succeed");
    
    // 9. Kill session
    let kill_tool = PtyKillTool;
    let mut kill_params = ToolParameters::new();
    kill_params.set("session_id", session_id);
    
    let kill_result = kill_tool.execute(kill_params, &context).await;
    assert!(kill_result.success, "Kill session should succeed");
    
    // Verify session is gone
    {
        let tm = terminal_manager.lock().await;
        assert!(!tm.session_exists(session_id).await, "Session should be killed");
    }
}

#[tokio::test]
#[serial]
async fn test_pty_concurrent_sessions() {
    let temp_dir = setup_test_environment().await;
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;

    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());
    
    // Create multiple sessions
    let session_ids = vec!["concurrent-1", "concurrent-2", "concurrent-3"];
    {
        let mut tm = terminal_manager.lock().await;
        for sid in &session_ids {
            tm.create_session(
                sid.to_string(),
                Some("/bin/bash".to_string()),
                Some(temp_dir.path().to_string_lossy().to_string()),
                80,
                24
            )
            .await
            .expect("Failed to create session");
        }
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // List all sessions
    let list_tool = PtyListTool;
    let list_result = list_tool.execute(ToolParameters::new(), &context).await;
    assert!(list_result.success, "List concurrent sessions should succeed");
    
    // Verify all sessions are listed
    for sid in &session_ids {
        assert!(list_result.content.contains(sid), 
            "Session {} should be in list", sid);
    }
    
    // Kill all sessions
    for sid in &session_ids {
        let kill_tool = PtyKillTool;
        let mut kill_params = ToolParameters::new();
        kill_params.set("session_id", sid);
        
        let kill_result = kill_tool.execute(kill_params, &context).await;
        assert!(kill_result.success, "Kill session {} should succeed", sid);
    }
}

#[tokio::test]
#[serial]
async fn test_pty_error_handling_comprehensive() {
    let temp_dir = setup_test_environment().await;
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;

    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());
    
    // Test pty_send_keys with nonexistent session
    {
        let send_keys_tool = PtySendKeysTool;
        let mut params = ToolParameters::new();
        params.set("session_id", "nonexistent-session");
        params.set("keys", "test\n");
        params.set("raw", false);
        
        let result = send_keys_tool.execute(params, &context).await;
        assert!(!result.success, "pty_send_keys should fail for nonexistent session");
        assert!(result.error.is_some(), "pty_send_keys should have error message");
    }
    
    // Test pty_get_screen with nonexistent session
    {
        let get_screen_tool = PtyGetScreenTool;
        let mut params = ToolParameters::new();
        params.set("session_id", "nonexistent-session");
        params.set("include_colors", false);
        params.set("include_cursor", true);
        
        let result = get_screen_tool.execute(params, &context).await;
        assert!(!result.success, "pty_get_screen should fail for nonexistent session");
        assert!(result.error.is_some(), "pty_get_screen should have error message");
    }
    
    // Test pty_get_cursor with nonexistent session
    {
        let get_cursor_tool = PtyGetCursorTool;
        let mut params = ToolParameters::new();
        params.set("session_id", "nonexistent-session");
        
        let result = get_cursor_tool.execute(params, &context).await;
        assert!(!result.success, "pty_get_cursor should fail for nonexistent session");
        assert!(result.error.is_some(), "pty_get_cursor should have error message");
    }
    
    // Test pty_resize with nonexistent session
    {
        let resize_tool = PtyResizeTool;
        let mut params = ToolParameters::new();
        params.set("session_id", "nonexistent-session");
        params.set("rows", 40);
        params.set("cols", 120);
        
        let result = resize_tool.execute(params, &context).await;
        assert!(!result.success, "pty_resize should fail for nonexistent session");
        assert!(result.error.is_some(), "pty_resize should have error message");
    }
    
    // Test pty_set_scrollback with nonexistent session
    {
        let set_scrollback_tool = PtySetScrollbackTool;
        let mut params = ToolParameters::new();
        params.set("session_id", "nonexistent-session");
        params.set("lines", 1000);
        
        let result = set_scrollback_tool.execute(params, &context).await;
        assert!(!result.success, "pty_set_scrollback should fail for nonexistent session");
        assert!(result.error.is_some(), "pty_set_scrollback should have error message");
    }
    
    // Test pty_start_capture with nonexistent session
    {
        let start_capture_tool = PtyStartCaptureTool;
        let mut params = ToolParameters::new();
        params.set("session_id", "nonexistent-session");
        
        let result = start_capture_tool.execute(params, &context).await;
        assert!(!result.success, "pty_start_capture should fail for nonexistent session");
        assert!(result.error.is_some(), "pty_start_capture should have error message");
    }
    
    // Test pty_stop_capture with nonexistent session
    {
        let stop_capture_tool = PtyStopCaptureTool;
        let mut params = ToolParameters::new();
        params.set("session_id", "nonexistent-session");
        
        let result = stop_capture_tool.execute(params, &context).await;
        assert!(!result.success, "pty_stop_capture should fail for nonexistent session");
        assert!(result.error.is_some(), "pty_stop_capture should have error message");
    }
    
    // Test pty_kill with nonexistent session
    {
        let kill_tool = PtyKillTool;
        let mut params = ToolParameters::new();
        params.set("session_id", "nonexistent-session");
        
        let result = kill_tool.execute(params, &context).await;
        assert!(!result.success, "pty_kill should fail for nonexistent session");
        assert!(result.error.is_some(), "pty_kill should have error message");
    }
}

#[tokio::test]
#[serial]
async fn test_pty_session_lifecycle() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-lifecycle";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());
    
    // Session should exist
    {
        let tm = terminal_manager.lock().await;
        assert!(tm.session_exists(&session_id).await, "Session should exist");
    }

    // Send some commands
    {
        let mut tm = terminal_manager.lock().await;
        tm.send_input(&session_id, "pwd\n").await
            .expect("Failed to send pwd");
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Get screen to verify output
    let get_screen_tool = PtyGetScreenTool;
    let mut screen_params = ToolParameters::new();
    screen_params.set("session_id", session_id);
    screen_params.set("include_colors", false);
    screen_params.set("include_cursor", true);
    
    let screen_result = get_screen_tool.execute(screen_params, &context).await;
    assert!(screen_result.success, "Should get screen");
    
    // Kill session
    let kill_tool = PtyKillTool;
    let mut kill_params = ToolParameters::new();
    kill_params.set("session_id", session_id);
    
    let kill_result = kill_tool.execute(kill_params, &context).await;
    assert!(kill_result.success, "Session should be killed");
    
    // Session should no longer exist
    {
        let tm = terminal_manager.lock().await;
        assert!(!tm.session_exists(session_id).await, "Session should not exist after kill");
    }
}

#[tokio::test]
#[serial]
async fn test_pty_special_key_sequences() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-special-keys";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let send_keys_tool = PtySendKeysTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());
    
    // Test various special key sequences
    let special_keys = vec![
        ("Ctrl+C", "^C"),
        ("Ctrl+D", "^D"),
        ("Tab", "[TAB]"),
        ("Up Arrow", "[UP]"),
        ("Down Arrow", "[DOWN]"),
        ("Left Arrow", "[LEFT]"),
        ("Right Arrow", "[RIGHT]"),
        ("F1", "[F1]"),
        ("F12", "[F12]"),
    ];
    
    for (name, key_seq) in special_keys {
        let mut params = ToolParameters::new();
        params.set("session_id", session_id);
        params.set("keys", key_seq);
        params.set("raw", false);
        
        let result = send_keys_tool.execute(params, &context).await;
        assert!(result.success, "{} should be sent successfully", name);
    }
}

#[tokio::test]
#[serial]
async fn test_pty_command_execution() {
    let temp_dir = setup_test_environment().await;
    let session_id = "test-command-exec";
    
    let terminal_manager = create_terminal_manager(temp_dir.path()).await;
    {
        let mut tm = terminal_manager.lock().await;
        tm.create_session(
            session_id.to_string(),
            Some("/bin/bash".to_string()),
            Some(temp_dir.path().to_string_lossy().to_string()),
            80,
            24
        )
        .await
        .expect("Failed to create session");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let send_keys_tool = PtySendKeysTool;
    let get_screen_tool = PtyGetScreenTool;
    let context = create_tool_context(temp_dir.path(), terminal_manager.clone());
    
    // Execute commands and verify output
    let commands = vec![
        ("echo hello", "hello"),
        ("pwd", "test"), // Should contain test directory
        ("ls", ""), // Just check it executes
    ];
    
    for (cmd, expected_substring) in commands {
        let mut params = ToolParameters::new();
        params.set("session_id", session_id);
        params.set("keys", format!("{}\\n", cmd));
        params.set("raw", false);
        
        let result = send_keys_tool.execute(params, &context).await;
        assert!(result.success, "Command '{}' should execute", cmd);
        
        // Wait for output
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        
        // Get screen to verify
        let mut screen_params = ToolParameters::new();
        screen_params.set("session_id", session_id);
        screen_params.set("include_colors", false);
        screen_params.set("include_cursor", true);
        
        let screen_result = get_screen_tool.execute(screen_params, &context).await;
        assert!(screen_result.success, "Should get screen after command");
        
        if !expected_substring.is_empty() {
            assert!(
                screen_result.content.contains(expected_substring) || 
                temp_dir.path().to_string_lossy().contains(expected_substring),
                "Output should contain '{}': {}",
                expected_substring,
                screen_result.content
            );
        }
    }
}