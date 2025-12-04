use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use async_trait::async_trait;

/// Tool parameters
#[derive(Debug, Clone)]
pub struct ToolParameters {
    pub data: HashMap<String, Value>,
}

impl ToolParameters {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn from_json(json_str: &str) -> Result<Self> {
        let data: HashMap<String, Value> = serde_json::from_str(json_str)?;
        Ok(Self { data })
    }

    pub fn set<T: Serialize>(&mut self, key: &str, value: T) {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.data.insert(key.to_string(), json_value);
        }
    }

    pub fn get_required<T>(&self, key: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let value = self.data.get(key)
            .ok_or_else(|| anyhow::anyhow!("Required parameter '{}' missing", key))?;

        serde_json::from_value(value.clone())
            .map_err(|e| anyhow::anyhow!("Failed to parse parameter '{}': {}", key, e))
    }

    pub fn get_optional<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        match self.data.get(key) {
            Some(value) => {
                let parsed: T = serde_json::from_value(value.clone())
                    .map_err(|e| anyhow::anyhow!("Failed to parse parameter '{}': {}", key, e))?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub content: String,
    pub error: Option<String>,
}

impl ToolResult {
    pub fn success(content: String) -> Self {
        Self {
            success: true,
            content,
            error: None,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            content: String::new(),
            error: Some(error),
        }
    }
}

/// Tool parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDefinition {
    pub param_type: String,
    pub description: String,
    pub required: bool,
    pub default: Option<Value>,
}

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
    async fn execute(&self, params: ToolParameters, context: &crate::tool_context::ToolContext) -> ToolResult;

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

/// Helper macro for creating parameter definitions
#[macro_export]
macro_rules! param {
    ($name:expr, $type:expr, $desc:expr, required) => {
        (
            $name.to_string(),
            ParameterDefinition {
                param_type: $type.to_string(),
                description: $desc.to_string(),
                required: true,
                default: None,
            }
        )
    };
    ($name:expr, $type:expr, $desc:expr, optional, $default:expr) => {
        (
            $name.to_string(),
            ParameterDefinition {
                param_type: $type.to_string(),
                description: $desc.to_string(),
                required: false,
                default: Some(serde_json::Value::from($default)),
            }
        )
    };
    ($name:expr, $type:expr, $desc:expr, optional) => {
        (
            $name.to_string(),
            ParameterDefinition {
                param_type: $type.to_string(),
                description: $desc.to_string(),
                required: false,
                default: None,
            }
        )
    };
}