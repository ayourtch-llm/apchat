use std::collections::HashMap;
use std::sync::Arc;
use super::tool::{Tool, ToolParameters, ToolResult};
use super::tool_context::ToolContext;
use super::content_limiter::ContentLimiter;

/// Registry for managing and discovering tools
#[derive(Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    categories: HashMap<String, Vec<String>>,
    content_limiter: Option<Arc<ContentLimiter>>,
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("tool_count", &self.tools.len())
            .field("categories", &self.categories)
            .finish()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            categories: HashMap::new(),
            content_limiter: None,
        }
    }

    /// Register a new tool
    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        let name = tool.name().to_string();
        let tool_arc = Arc::new(tool);
        self.tools.insert(name.clone(), tool_arc);
    }

    /// Register a tool with categories
    pub fn register_with_categories<T: Tool + 'static>(&mut self, tool: T, categories: Vec<String>) {
        let name = tool.name().to_string();
        self.register(tool);

        for category in categories {
            self.categories.entry(category).or_insert_with(Vec::new).push(name.clone());
        }
    }

    /// Get a tool by name
    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Get all tools
    pub fn get_all_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.values().cloned().collect()
    }

    /// Get tools by category
    pub fn get_tools_by_category(&self, category: &str) -> Vec<Arc<dyn Tool>> {
        if let Some(tool_names) = self.categories.get(category) {
            tool_names.iter()
                .filter_map(|name| self.tools.get(name))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Check if a tool exists
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get tool names
    pub fn get_tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Execute a tool by name
    pub async fn execute_tool(
        &self,
        name: &str,
        params: ToolParameters,
        context: &ToolContext,
    ) -> ToolResult {
        match self.get_tool(name) {
            Some(tool) => {
                // Use content limiter from registry if available, otherwise use context's
                let effective_context = if let Some(limiter) = &self.content_limiter {
                    if context.content_limiter.is_none() {
                        let mut context_clone = context.clone();
                        context_clone.content_limiter = Some(Arc::clone(limiter));
                        context_clone
                    } else {
                        context.clone()
                    }
                } else {
                    context.clone()
                };
                tool.execute(params, &effective_context).await
            },
            None => ToolResult::error(format!("Tool '{}' not found", name)),
        }
    }

    /// Get all tool definitions in OpenAI format
    pub fn get_openai_tool_definitions(&self) -> Vec<serde_json::Value> {
        let mut tools: Vec<_> = self.tools.iter().collect();
        // Sort by tool name to ensure consistent ordering (critical for prompt caching)
        tools.sort_by_key(|(name, _)| name.as_str());
        tools.into_iter()
            .map(|(_, tool)| tool.to_openai_definition())
            .collect()
    }

    /// Get all categories
    pub fn get_categories(&self) -> Vec<String> {
        self.categories.keys().cloned().collect()
    }

    /// Set the content limiter for the registry
    pub fn set_content_limiter(&mut self, content_limiter: Arc<ContentLimiter>) {
        self.content_limiter = Some(content_limiter);
    }

    /// Create a new ToolRegistry with a content limiter
    pub fn with_content_limiter(mut self, content_limiter: Arc<ContentLimiter>) -> Self {
        self.content_limiter = Some(content_limiter);
        self
    }

    /// Create a new context with the registry's content limiter propagated
    /// If the context already has a content limiter, it takes precedence
    pub fn to_context(&self, context: ToolContext) -> ToolContext {
        if let Some(limiter) = &self.content_limiter {
            if context.content_limiter.is_none() {
                let mut context_clone = context;
                context_clone.content_limiter = Some(Arc::clone(limiter));
                context_clone
            } else {
                context
            }
        } else {
            context
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::ParameterDefinition;

    struct MockTool {
        name: String,
        description: String,
    }

    #[async_trait::async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            &self.description
        }

        fn parameters(&self) -> HashMap<String, ParameterDefinition> {
            HashMap::new()
        }

        async fn execute(&self, _params: ToolParameters, _context: &ToolContext) -> ToolResult {
            ToolResult::success("mock result".to_string())
        }
    }

    #[tokio::test]
    async fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        let tool = MockTool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
        };

        registry.register(tool);

        assert!(registry.has_tool("test_tool"));
        let retrieved_tool = registry.get_tool("test_tool");
        assert!(retrieved_tool.is_some());

        let params = ToolParameters { data: HashMap::new() };
        let context = ToolContext::new(
            std::path::PathBuf::from("/tmp"),
            "test_session".to_string(),
            apchat_policy::PolicyManager::new(),
        );
        let result = registry.execute_tool("test_tool", params, &context).await;
        assert!(result.success);
    }
}

#[cfg(test)]
mod content_limiter_tests {
    use super::*;
    use crate::content_limiter::ContentLimiterConfig;
    use crate::tool::ParameterDefinition;
    use tempfile::TempDir;

    struct MockTool {
        name: String,
        description: String,
    }

    #[async_trait::async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            &self.description
        }

        fn parameters(&self) -> HashMap<String, ParameterDefinition> {
            HashMap::new()
        }

        async fn execute(&self, _params: ToolParameters, _context: &ToolContext) -> ToolResult {
            ToolResult::success("mock result".to_string())
        }
    }

    #[tokio::test]
    async fn test_tool_registry_with_content_limiter() {
        let mut registry = ToolRegistry::new();
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().to_path_buf();
        let policy_manager = apchat_policy::PolicyManager::new();

        // Create a content limiter
        let config = ContentLimiterConfig::new(&work_dir);
        let limiter = Arc::new(ContentLimiter::new(config));

        // Set content limiter on registry
        registry.set_content_limiter(Arc::clone(&limiter));

        // Register a tool
        let tool = MockTool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
        };

        registry.register(tool);

        // Create context without content limiter
        let context = ToolContext::new(
            work_dir.clone(),
            "test_session".to_string(),
            policy_manager,
        );

        // Execute tool - should use registry's content limiter
        assert!(registry.has_tool("test_tool"));

        let params = ToolParameters { data: HashMap::new() };
        let result = registry.execute_tool("test_tool", params, &context).await;
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_tool_registry_with_content_limiter_method() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().to_path_buf();

        // Create registry with content limiter using with_content_limiter
        let config = ContentLimiterConfig::new(&work_dir);
        let limiter = Arc::new(ContentLimiter::new(config));
        let registry = ToolRegistry::new().with_content_limiter(Arc::clone(&limiter));

        // Register a tool
        let tool = MockTool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
        };

        let mut registry_clone = registry.clone();
        registry_clone.register(tool);

        // Create context without content limiter
        let policy_manager = apchat_policy::PolicyManager::new();
        let context = ToolContext::new(
            work_dir.clone(),
            "test_session".to_string(),
            policy_manager,
        );

        // Execute tool - should use registry's content limiter
        assert!(registry_clone.has_tool("test_tool"));

        let params = ToolParameters { data: HashMap::new() };
        let result = registry_clone.execute_tool("test_tool", params, &context).await;
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_tool_registry_to_context() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().to_path_buf();

        // Create registry with content limiter
        let config = ContentLimiterConfig::new(&work_dir);
        let limiter = Arc::new(ContentLimiter::new(config));
        let registry = ToolRegistry::new().with_content_limiter(Arc::clone(&limiter));

        // Create context without content limiter
        let policy_manager = apchat_policy::PolicyManager::new();
        let context = ToolContext::new(
            work_dir.clone(),
            "test_session".to_string(),
            policy_manager,
        );

        // Use to_context to propagate content limiter
        let context_with_limiter = registry.to_context(context);

        // Verify content limiter was propagated
        assert!(context_with_limiter.content_limiter.is_some());
    }

    #[tokio::test]
    async fn test_tool_registry_context_takes_precedence() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().to_path_buf();

        // Create registry with content limiter
        let config1 = ContentLimiterConfig::new(&work_dir);
        let limiter1 = Arc::new(ContentLimiter::new(config1));

        // Create registry with content limiter using with_content_limiter
        let registry = ToolRegistry::new().with_content_limiter(Arc::clone(&limiter1));

        // Create context with its own content limiter
        let config2 = ContentLimiterConfig::new(&work_dir);
        let limiter2 = Arc::new(ContentLimiter::new(config2));

        let policy_manager = apchat_policy::PolicyManager::new();
        let context = ToolContext::new(
            work_dir.clone(),
            "test_session".to_string(),
            policy_manager,
        ).with_content_limiter(Arc::clone(&limiter2));

        // Use to_context - context's limiter should take precedence
        let context_with_limiter = registry.to_context(context);

        // Verify context's content limiter was preserved
        assert!(context_with_limiter.content_limiter.is_some());
    }
}