use chrono::Local;
use serde::Serialize;
use serde_json;
use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use crate::get_logs_dir;

#[derive(Serialize)]
struct ToolCallInfo {
    id: String,
    name: String,
    arguments: String,
}

#[derive(Serialize)]
struct LogEntry {
    timestamp: String, // ISO-8601 Local time
    role: String,
    content: String,
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_binary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCallInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_depth: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_name: Option<String>,
}

pub struct ConversationLogger {
    file_path: PathBuf,
    file: Option<tokio::fs::File>,
}

impl ConversationLogger {
    /// Create a new logger; generates the file name based on the current local time.
    pub async fn new(_workspace: &Path) -> Result<Self> {
        // Use shared logs directory from utility function
        let logs_dir = get_logs_dir()?;

        let now_local = Local::now();
        let filename = format!(
            "kchat-{}.jsonl",
            now_local.format("%Y-%m-%d-%H%M%S")
        );
        let file_path = logs_dir.join(filename);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await?;
        Ok(Self { file_path, file: Some(file) })
    }

    /// Create a new logger for task mode; generates the file name with "-task" suffix.
    pub async fn new_task_mode(_workspace: &Path) -> Result<Self> {
        // Use shared logs directory from utility function
        let logs_dir = get_logs_dir()?;

        let now_local = Local::now();
        let filename = format!(
            "kchat-{}-task.jsonl",
            now_local.format("%Y-%m-%d-%H%M%S")
        );
        let file_path = logs_dir.join(filename);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await?;
        Ok(Self { file_path, file: Some(file) })
    }

    /// Create a logger writing to a specific file path (for testing).
    #[cfg(test)]
    async fn new_with_path(file_path: PathBuf) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await?;
        Ok(Self { file_path, file: Some(file) })
    }

    /// Append a single log entry.
    pub async fn log(&mut self, role: &str, content: &str, model: Option<&str>, is_binary: bool) {
        self.log_with_task_context(role, content, model, is_binary, None, None, None, None).await;
    }

    /// Append a log entry with task context
    pub async fn log_with_task_context(
        &mut self,
        role: &str,
        content: &str,
        model: Option<&str>,
        is_binary: bool,
        task_id: Option<&str>,
        parent_task_id: Option<&str>,
        task_depth: Option<usize>,
        agent_name: Option<&str>,
    ) {
        let entry = LogEntry {
            timestamp: Local::now().to_rfc3339(),
            role: role.to_string(),
            content: content.to_string(),
            model: model.map(|s| s.to_string()),
            is_binary: if is_binary { Some(true) } else { None },
            tool_calls: None,
            tool_call_id: None,
            name: None,
            task_id: task_id.map(|s| s.to_string()),
            parent_task_id: parent_task_id.map(|s| s.to_string()),
            task_depth,
            agent_name: agent_name.map(|s| s.to_string()),
        };
        if let Some(file) = &mut self.file {
            if let Ok(json) = serde_json::to_string(&entry) {
                // Write the JSON line
                if let Err(e) = file.write_all(json.as_bytes()).await {
                    eprintln!("[Logging error] {}", e);
                } else if let Err(e) = file.write_all(b"\n").await {
                    eprintln!("[Logging error] {}", e);
                }
            }
        }
    }

    /// Log an assistant message with tool calls
    pub async fn log_with_tool_calls(
        &mut self,
        role: &str,
        content: &str,
        model: Option<&str>,
        tool_calls: Vec<(String, String, String)>, // (id, name, arguments)
    ) {
        self.log_with_tool_calls_and_task(role, content, model, tool_calls, None, None, None, None).await;
    }

    /// Log an assistant message with tool calls and task context
    pub async fn log_with_tool_calls_and_task(
        &mut self,
        role: &str,
        content: &str,
        model: Option<&str>,
        tool_calls: Vec<(String, String, String)>, // (id, name, arguments)
        task_id: Option<&str>,
        parent_task_id: Option<&str>,
        task_depth: Option<usize>,
        agent_name: Option<&str>,
    ) {
        let tool_call_info: Vec<ToolCallInfo> = tool_calls
            .into_iter()
            .map(|(id, name, arguments)| ToolCallInfo { id, name, arguments })
            .collect();

        let entry = LogEntry {
            timestamp: Local::now().to_rfc3339(),
            role: role.to_string(),
            content: content.to_string(),
            model: model.map(|s| s.to_string()),
            is_binary: None,
            tool_calls: Some(tool_call_info),
            tool_call_id: None,
            name: None,
            task_id: task_id.map(|s| s.to_string()),
            parent_task_id: parent_task_id.map(|s| s.to_string()),
            task_depth,
            agent_name: agent_name.map(|s| s.to_string()),
        };
        if let Some(file) = &mut self.file {
            if let Ok(json) = serde_json::to_string(&entry) {
                if std::env::var("DEBUG_LOG").is_ok() {
                    eprintln!("[DEBUG] Writing tool_calls log entry: {}", &json[..json.len().min(100)]);
                }
                if let Err(e) = file.write_all(json.as_bytes()).await {
                    eprintln!("[Logging error] {}", e);
                } else if let Err(e) = file.write_all(b"\n").await {
                    eprintln!("[Logging error] {}", e);
                } else {
                    // Flush to ensure it's written
                    let _ = file.flush().await;
                }
            }
        }
    }

    /// Log a tool result
    pub async fn log_tool_result(
        &mut self,
        content: &str,
        tool_call_id: &str,
        tool_name: &str,
    ) {
        self.log_tool_result_with_task(content, tool_call_id, tool_name, None, None, None, None).await;
    }

    /// Log a tool result with task context
    pub async fn log_tool_result_with_task(
        &mut self,
        content: &str,
        tool_call_id: &str,
        tool_name: &str,
        task_id: Option<&str>,
        parent_task_id: Option<&str>,
        task_depth: Option<usize>,
        agent_name: Option<&str>,
    ) {
        let entry = LogEntry {
            timestamp: Local::now().to_rfc3339(),
            role: "tool".to_string(),
            content: content.to_string(),
            model: None,
            is_binary: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.to_string()),
            name: Some(tool_name.to_string()),
            task_id: task_id.map(|s| s.to_string()),
            parent_task_id: parent_task_id.map(|s| s.to_string()),
            task_depth,
            agent_name: agent_name.map(|s| s.to_string()),
        };
        if let Some(file) = &mut self.file {
            if let Ok(json) = serde_json::to_string(&entry) {
                if std::env::var("DEBUG_LOG").is_ok() {
                    eprintln!("[DEBUG] Writing tool result for {}: {}", tool_name, &content[..content.len().min(50)]);
                }
                if let Err(e) = file.write_all(json.as_bytes()).await {
                    eprintln!("[Logging error] {}", e);
                } else if let Err(e) = file.write_all(b"\n").await {
                    eprintln!("[Logging error] {}", e);
                } else {
                    // Flush to ensure it's written
                    let _ = file.flush().await;
                }
            }
        }
    }

    /// Close the logger (explicit drop). Called on graceful shutdown.
    pub async fn shutdown(&mut self) {
        if let Some(file) = self.file.take() {
            // Ensure data is flushed
            let _ = file.sync_all().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_log_entry_serialization_basic() {
        let entry = LogEntry {
            timestamp: "2024-01-01T00:00:00+00:00".to_string(),
            role: "user".to_string(),
            content: "Hello".to_string(),
            model: None,
            is_binary: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            task_id: None,
            parent_task_id: None,
            task_depth: None,
            agent_name: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello\""));
        // Optional None fields with skip_serializing_if should be absent
        assert!(!json.contains("tool_calls"));
        assert!(!json.contains("tool_call_id"));
        assert!(!json.contains("task_id"));
        assert!(!json.contains("agent_name"));
    }

    #[tokio::test]
    async fn test_log_entry_serialization_with_binary_flag() {
        let entry = LogEntry {
            timestamp: "2024-01-01T00:00:00+00:00".to_string(),
            role: "assistant".to_string(),
            content: "binary data".to_string(),
            model: Some("test-model".to_string()),
            is_binary: Some(true),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            task_id: None,
            parent_task_id: None,
            task_depth: None,
            agent_name: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"is_binary\":true"));
        assert!(json.contains("\"model\":\"test-model\""));
    }

    #[tokio::test]
    async fn test_log_entry_serialization_with_tool_calls() {
        let entry = LogEntry {
            timestamp: "2024-01-01T00:00:00+00:00".to_string(),
            role: "assistant".to_string(),
            content: "".to_string(),
            model: None,
            is_binary: None,
            tool_calls: Some(vec![
                ToolCallInfo {
                    id: "call_1".to_string(),
                    name: "read_file".to_string(),
                    arguments: "{\"path\": \"/tmp/test\"}".to_string(),
                },
            ]),
            tool_call_id: None,
            name: None,
            task_id: None,
            parent_task_id: None,
            task_depth: None,
            agent_name: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"tool_calls\""));
        assert!(json.contains("\"name\":\"read_file\""));
        assert!(json.contains("call_1"));
    }

    #[tokio::test]
    async fn test_log_entry_with_task_context() {
        let entry = LogEntry {
            timestamp: "2024-01-01T00:00:00+00:00".to_string(),
            role: "assistant".to_string(),
            content: "working".to_string(),
            model: None,
            is_binary: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            task_id: Some("task-123".to_string()),
            parent_task_id: Some("task-000".to_string()),
            task_depth: Some(2),
            agent_name: Some("code_analyzer".to_string()),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"task_id\":\"task-123\""));
        assert!(json.contains("\"parent_task_id\":\"task-000\""));
        assert!(json.contains("\"task_depth\":2"));
        assert!(json.contains("\"agent_name\":\"code_analyzer\""));
    }

    #[tokio::test]
    async fn test_logger_writes_jsonl_lines() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.jsonl");

        let mut logger = ConversationLogger::new_with_path(file_path.clone()).await.unwrap();
        logger.log("user", "Hello", None, false).await;
        logger.log("assistant", "Hi there", Some("test-model"), false).await;
        logger.shutdown().await;

        let contents = tokio::fs::read_to_string(&file_path).await.unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2);

        let entry1: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(entry1["role"], "user");
        assert_eq!(entry1["content"], "Hello");

        let entry2: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(entry2["role"], "assistant");
        assert_eq!(entry2["content"], "Hi there");
        assert_eq!(entry2["model"], "test-model");
    }

    #[tokio::test]
    async fn test_logger_log_with_task_context() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test_task.jsonl");

        let mut logger = ConversationLogger::new_with_path(file_path.clone()).await.unwrap();
        logger.log_with_task_context(
            "assistant", "result", Some("model-x"), false,
            Some("t1"), Some("t0"), Some(1), Some("planner"),
        ).await;
        logger.shutdown().await;

        let contents = tokio::fs::read_to_string(&file_path).await.unwrap();
        let entry: serde_json::Value = serde_json::from_str(contents.trim()).unwrap();
        assert_eq!(entry["task_id"], "t1");
        assert_eq!(entry["parent_task_id"], "t0");
        assert_eq!(entry["task_depth"], 1);
        assert_eq!(entry["agent_name"], "planner");
    }

    #[tokio::test]
    async fn test_logger_log_with_tool_calls() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test_tools.jsonl");

        let mut logger = ConversationLogger::new_with_path(file_path.clone()).await.unwrap();
        logger.log_with_tool_calls(
            "assistant", "I'll read the file",
            Some("model-y"),
            vec![
                ("id1".to_string(), "read_file".to_string(), "{\"path\":\"/tmp\"}".to_string()),
                ("id2".to_string(), "list_files".to_string(), "{}".to_string()),
            ],
        ).await;
        logger.shutdown().await;

        let contents = tokio::fs::read_to_string(&file_path).await.unwrap();
        let entry: serde_json::Value = serde_json::from_str(contents.trim()).unwrap();
        assert_eq!(entry["role"], "assistant");
        let tool_calls = entry["tool_calls"].as_array().unwrap();
        assert_eq!(tool_calls.len(), 2);
        assert_eq!(tool_calls[0]["name"], "read_file");
        assert_eq!(tool_calls[1]["name"], "list_files");
    }

    #[tokio::test]
    async fn test_logger_log_tool_result() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test_tool_result.jsonl");

        let mut logger = ConversationLogger::new_with_path(file_path.clone()).await.unwrap();
        logger.log_tool_result("file contents here", "call_42", "read_file").await;
        logger.shutdown().await;

        let contents = tokio::fs::read_to_string(&file_path).await.unwrap();
        let entry: serde_json::Value = serde_json::from_str(contents.trim()).unwrap();
        assert_eq!(entry["role"], "tool");
        assert_eq!(entry["tool_call_id"], "call_42");
        assert_eq!(entry["name"], "read_file");
        assert_eq!(entry["content"], "file contents here");
    }

    #[tokio::test]
    async fn test_logger_binary_flag_omitted_when_false() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test_binary.jsonl");

        let mut logger = ConversationLogger::new_with_path(file_path.clone()).await.unwrap();
        logger.log("user", "not binary", None, false).await;
        logger.shutdown().await;

        let contents = tokio::fs::read_to_string(&file_path).await.unwrap();
        // is_binary should not appear (it's None when false)
        assert!(!contents.contains("is_binary"));
    }

    #[tokio::test]
    async fn test_logger_binary_flag_present_when_true() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test_binary_true.jsonl");

        let mut logger = ConversationLogger::new_with_path(file_path.clone()).await.unwrap();
        logger.log("user", "binary data", None, true).await;
        logger.shutdown().await;

        let contents = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert!(contents.contains("\"is_binary\":true"));
    }

    #[tokio::test]
    async fn test_logger_empty_content() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test_empty.jsonl");

        let mut logger = ConversationLogger::new_with_path(file_path.clone()).await.unwrap();
        logger.log("user", "", None, false).await;
        logger.shutdown().await;

        let contents = tokio::fs::read_to_string(&file_path).await.unwrap();
        let entry: serde_json::Value = serde_json::from_str(contents.trim()).unwrap();
        assert_eq!(entry["content"], "");
    }

    #[tokio::test]
    async fn test_logger_shutdown_makes_file_none() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test_shutdown.jsonl");

        let mut logger = ConversationLogger::new_with_path(file_path.clone()).await.unwrap();
        assert!(logger.file.is_some());
        logger.shutdown().await;
        assert!(logger.file.is_none());
    }

    #[tokio::test]
    async fn test_logger_log_after_shutdown_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test_after_shutdown.jsonl");

        let mut logger = ConversationLogger::new_with_path(file_path.clone()).await.unwrap();
        logger.shutdown().await;
        // Logging after shutdown should not panic, just silently do nothing
        logger.log("user", "should not appear", None, false).await;

        let contents = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert!(contents.is_empty());
    }
}
