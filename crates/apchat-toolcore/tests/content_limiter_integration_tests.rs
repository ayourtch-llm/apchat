// Test to demonstrate how a tool would use the content limiter
use apchat_toolcore::{Tool, ToolParameters, ToolResult, ParameterDefinition};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(test)]
mod content_limiter_integration_tests {
    use super::*;

    // Example tool that uses content limiter
    pub struct ContentGeneratingTool;

    #[async_trait]
    impl Tool for ContentGeneratingTool {
        fn name(&self) -> &str {
            "content_generating_tool"
        }

        fn description(&self) -> &str {
            "A tool that generates large content and uses the content limiter"
        }

        fn parameters(&self) -> HashMap<String, ParameterDefinition> {
            HashMap::from([
                apchat_toolcore::param!("size", "integer", "Size of content to generate", required),
            ])
        }

        async fn execute(&self, params: ToolParameters, context: &apchat_toolcore::ToolContext) -> ToolResult {
            let size = params.get_required::<usize>("size").unwrap();
            
            // Generate large content
            let content = "a".repeat(size);
            let content_clone = content.clone();
            
            // Check if content limiter is available
            if let Some(content_limiter) = &context.content_limiter {
                let (truncated_content, note, was_truncated) = content_limiter.save_and_truncate(
                    content, self.name()
                );
                
                if was_truncated {
                    let full_path = note.as_ref().and_then(|n| {
                        n.split("at: ").last().map(|s| s.trim().to_string())
                    }).unwrap_or_default();
                    
                    return ToolResult::success_with_truncation(truncated_content, full_path);
                }
            }
            
            // Return normal result if no truncation
            ToolResult::success(content_clone)
        }
    }

    #[tokio::test]
    async fn test_tool_with_content_limiter() {
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().to_path_buf();
        
        // Create tool context with content limiter
        let policy_manager = apchat_policy::PolicyManager::new();
        let content_limiter_config = apchat_toolcore::content_limiter::ContentLimiterConfig::new(&work_dir);
        let content_limiter = Arc::new(apchat_toolcore::content_limiter::ContentLimiter::new(content_limiter_config));
        
        let context = apchat_toolcore::ToolContext::new(
            work_dir.clone(),
            "test_session".to_string(),
            policy_manager
        ).with_content_limiter(content_limiter);
        
        // Create tool
        let tool = ContentGeneratingTool;
        
        // Test with small content (should not truncate)
        let mut params = ToolParameters::new();
        params.set("size", 100);
        let result = tool.execute(params.clone(), &context).await;
        assert!(result.success);
        assert!(!result.truncated);
        assert!(result.content.len() == 100);
        
        // Test with large content (should truncate)
        let mut params = ToolParameters::new();
        params.set("size", 30_000);
        let result = tool.execute(params, &context).await;
        assert!(result.success);
        assert!(result.truncated);
        assert!(result.content.contains("🚨 LARGE OUTPUT TRUNCATED 🚨"));
        assert!(result.content.contains("content_generating_tool"));
        assert!(result.full_path.is_some());
    }

    #[tokio::test]
    async fn test_tool_without_content_limiter() {
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().to_path_buf();
        
        // Create tool context WITHOUT content limiter
        let policy_manager = apchat_policy::PolicyManager::new();
        let context = apchat_toolcore::ToolContext::new(
            work_dir.clone(),
            "test_session".to_string(),
            policy_manager
        );
        
        // Create tool
        let tool = ContentGeneratingTool;
        
        // Test with large content (should not truncate since no limiter)
        let mut params = ToolParameters::new();
        params.set("size", 30_000);
        let result = tool.execute(params, &context).await;
        assert!(result.success);
        assert!(!result.truncated);
        assert!(result.content.len() == 30_000);
    }
}