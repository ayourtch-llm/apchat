//! Tool trait and types - re-exported from kimichat-types

pub use kimichat_types::{
    ToolParameters, ToolResult, ParameterDefinition, param
};
use async_trait::async_trait;
use std::collections::HashMap;

/// Tool trait that all tools must implement
#[async_trait]
pub trait Tool: Send + Sync {
    /// Name of the tool (must be unique)
    fn name(&self) -> &str;

    /// Human-readable description
    fn description(&self) -> &str;

    /// Parameter definitions
    fn parameters(&self) -> HashMap<String, ParameterDefinition>;

    /// Execute the tool
    async fn execute(&self, params: ToolParameters, context: &crate::core::tool_context::ToolContext) -> ToolResult;

    /// Get OpenAI-compatible tool definition
    fn to_openai_definition(&self) -> serde_json::Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for (name, param_def) in self.parameters() {
            // Build parameter definition with sorted keys
            let mut param_obj = serde_json::Map::new();
            if let Some(default) = &param_def.default {
                param_obj.insert("default".to_string(), default.clone());
            }
            param_obj.insert("description".to_string(), serde_json::Value::String(param_def.description.clone()));
            param_obj.insert("type".to_string(), serde_json::Value::String(param_def.param_type.clone()));

            properties.insert(name.clone(), serde_json::Value::Object(param_obj));

            if param_def.required {
                required.push(name);
            }
        }

        // Sort required array alphabetically for consistent caching
        required.sort();

        // Build properties in sorted order
        let mut sorted_properties = serde_json::Map::new();
        let mut prop_keys: Vec<_> = properties.keys().cloned().collect();
        prop_keys.sort();
        for key in prop_keys {
            sorted_properties.insert(key.clone(), properties[&key].clone());
        }

        // Build parameters object with sorted keys
        let mut parameters = serde_json::Map::new();
        parameters.insert("properties".to_string(), serde_json::Value::Object(sorted_properties));
        parameters.insert("required".to_string(), serde_json::Value::Array(required.into_iter().map(serde_json::Value::String).collect()));
        parameters.insert("type".to_string(), serde_json::Value::String("object".to_string()));

        // Build function object with sorted keys
        let mut function = serde_json::Map::new();
        function.insert("description".to_string(), serde_json::Value::String(self.description().to_string()));
        function.insert("name".to_string(), serde_json::Value::String(self.name().to_string()));
        function.insert("parameters".to_string(), serde_json::Value::Object(parameters));

        // Build top-level object with sorted keys
        let mut result = serde_json::Map::new();
        result.insert("function".to_string(), serde_json::Value::Object(function));
        result.insert("type".to_string(), serde_json::Value::String("function".to_string()));

        serde_json::Value::Object(result)
    }
}
