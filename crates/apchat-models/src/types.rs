use serde::{Deserialize, Deserializer, Serialize};

/// Backend type for LLM models
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BackendType {
    Groq,
    Anthropic,
    Llama,
    OpenAI,
}

impl BackendType {
    /// Parse backend type from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "groq" => Some(Self::Groq),
            "anthropic" | "claude" => Some(Self::Anthropic),
            "llama" | "llamacpp" | "llama.cpp" | "llama-cpp" => Some(Self::Llama),
            "openai" => Some(Self::OpenAI),
            _ => None,
        }
    }

    /// Get string representation
    pub fn as_str(&self) -> &str {
        match self {
            Self::Groq => "groq",
            Self::Anthropic => "anthropic",
            Self::Llama => "llama",
            Self::OpenAI => "openai",
        }
    }
}

/// Model colors supported by the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ModelColor {
    BluModel = 0,
    GrnModel = 1,
    RedModel = 2,
}

/// Model provider configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct ModelProvider {
    /// Model name (e.g., "moonshotai/kimi-k2-instruct-0905")
    pub model_name: String,
    /// Backend type (Groq, Anthropic, Llama, etc.)
    pub backend: Option<BackendType>,
    /// API URL for the provider
    pub api_url: Option<String>,
    /// API key for the provider
    pub api_key: Option<String>,
}

impl std::fmt::Debug for ModelProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let masked_key = match &self.api_key {
            Some(key) if key.len() > 3 => format!("{}***", &key[..3]),
            Some(key) => format!("{}***", &key),
            None => "None".to_string(),
        };
        
        f.debug_struct("ModelProvider")
            .field("model_name", &self.model_name)
            .field("backend", &self.backend)
            .field("api_url", &self.api_url)
            .field("api_key", &masked_key)
            .finish()
    }
}

impl ModelProvider {
    /// Create a new ModelProvider with minimal configuration
    pub fn new(model_name: String) -> Self {
        Self {
            model_name,
            backend: None,
            api_url: None,
            api_key: None,
        }
    }
    
    /// Create a new ModelProvider with all fields
    pub fn with_config(model_name: String, backend: Option<BackendType>, api_url: Option<String>, api_key: Option<String>) -> Self {
        Self {
            model_name,
            backend,
            api_url,
            api_key,
        }
    }
}

/// CLI configuration for a specific model
#[derive(Debug, Clone, Default)]
pub struct ModelConfig {
    /// Backend type for this model
    pub backend: Option<String>,
    /// API URL for this model
    pub api_url: Option<String>,
    /// API key for this model
    pub api_key: Option<String>,
    /// Model name override for this model
    pub model: Option<String>,
}

impl ModelColor {
    /// Total number of model colors
    pub const COUNT: usize = 3;
    
    /// Get an iterator over all model colors
    pub fn iter() -> impl Iterator<Item = ModelColor> {
        [ModelColor::BluModel, ModelColor::GrnModel, ModelColor::RedModel]
            .iter().copied()
    }

    /// Get the display name for the model (just the color)
    pub fn display_name(&self) -> &'static str {
        match self {
            ModelColor::BluModel => "BluModel",
            ModelColor::GrnModel => "GrnModel", 
            ModelColor::RedModel => "RedModel",
        }
    }

    /// Get the default model for this color
    /// 
    /// Note: The color-to-model mapping shown here is not fixed and can be changed
    /// independently of the color scheme. These are just default fallback values.
    /// The actual model assignments can be configured at runtime through CLI arguments,
    /// configuration files, or API parameters, allowing any model to be associated
    /// with any color regardless of the default mapping shown below.
    pub fn default_model(&self) -> String {
        match self {
            ModelColor::BluModel => "some-model".to_string(),
            ModelColor::GrnModel => "some-model".to_string(),
            ModelColor::RedModel => "some-model".to_string(),
        }
    }

    /// Get the lowercase string representation of the model color
    pub fn as_str_lowercase(&self) -> &'static str {
        match self {
            ModelColor::BluModel => "blu",
            ModelColor::GrnModel => "grn",
            ModelColor::RedModel => "red",
        }
    }

    /// Get the default model string for this color (alias for default_model)
    pub fn as_str_default(&self) -> String {
        self.default_model()
    }

    /// Get model identifier with optional overrides
    pub fn as_str(
        &self,
        blu_model_override: Option<&str>,
        grn_model_override: Option<&str>,
        red_model_override: Option<&str>,
    ) -> String {
        match self {
            ModelColor::BluModel => blu_model_override
                .map(|s| s.to_string())
                .unwrap_or_else(|| self.as_str_default()),
            ModelColor::GrnModel => grn_model_override
                .map(|s| s.to_string())
                .unwrap_or_else(|| self.as_str_default()),
            ModelColor::RedModel => red_model_override
                .map(|s| s.to_string())
                .unwrap_or_else(|| self.as_str_default()),
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "blu_model" | "blu-model" | "blumodel" => ModelColor::BluModel,
            "grn_model" | "grn-model" | "grnmodel" => ModelColor::GrnModel,
            "red_model" | "red-model" | "redmodel" => ModelColor::RedModel,
            _ => {
                // For backward compatibility:
                // - Anthropic models default to BluModel
                // - Custom models default to GrnModel
                if s.to_lowercase().contains("anthropic") || s.to_lowercase().contains("claude") {
                    ModelColor::BluModel // Anthropic models map to BluModel
                } else if s.to_lowercase().contains("openai") || s.to_lowercase().contains("gpt") {
                    ModelColor::GrnModel // OpenAI models map to GrnModel
                } else {
                    ModelColor::GrnModel // Default to GrnModel for other custom models
                }
            }
        }
    }
}

impl std::str::FromStr for ModelColor {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ModelColor::from_string(s))
    }
}

/// Helper function to deserialize string or null values
pub fn deserialize_string_or_null<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::String(s) => Ok(s),
        serde_json::Value::Null => Ok(String::new()),
        _ => Ok(String::new()),
    }
}

/// Image URL structure for multimodal content
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageUrl {
    pub url: String,
}

/// Content part for multimodal messages
#[derive(Debug, Clone, PartialEq)]
pub enum ContentPart {
    Text(String),
    ImageUrl { url: String },
}

impl Serialize for ContentPart {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        
        match self {
            ContentPart::Text(text) => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("type", "text")?;
                map.serialize_entry("text", text)?;
                map.end()
            }
            ContentPart::ImageUrl { url } => {
                // Create a helper struct for image_url serialization
                #[derive(Serialize)]
                struct ImageUrlHelper<'a> {
                    url: &'a str,
                }
                
                let image_url_helper = ImageUrlHelper { url };
                
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("type", "image_url")?;
                map.serialize_entry("image_url", &image_url_helper)?;
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for ContentPart {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct ContentPartVisitor;

        impl<'de> Visitor<'de> for ContentPartVisitor {
            type Value = ContentPart;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a content part object with 'type' field")
            }

            fn visit_map<V>(self, mut map: V) -> Result<ContentPart, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut content_type: Option<String> = None;
                let mut text: Option<String> = None;
                let mut image_url: Option<ImageUrl> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "type" => {
                            content_type = Some(map.next_value()?);
                        }
                        "text" => {
                            text = Some(map.next_value()?);
                        }
                        "image_url" => {
                            image_url = Some(map.next_value()?);
                        }
                        _ => {
                            // Skip unknown fields
                            let _: de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                match content_type.as_deref() {
                    Some("text") => {
                        text.ok_or_else(|| de::Error::missing_field("text"))
                            .map(ContentPart::Text)
                    }
                    Some("image_url") => {
                        image_url
                            .ok_or_else(|| de::Error::missing_field("image_url"))
                            .map(|iu| ContentPart::ImageUrl { url: iu.url })
                    }
                    _ => Err(de::Error::missing_field("type")),
                }
            }
        }

        deserializer.deserialize_map(ContentPartVisitor)
    }
}

/// Message structure for chat API
#[derive(Debug, Clone, Default)]
pub struct Message {
    pub role: String,
    pub content: Vec<ContentPart>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
    pub name: Option<String>,
    pub reasoning: Option<String>,
}

impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("role", &self.role)?;
        map.serialize_entry("content", &self.content)?;
        
        if let Some(ref tool_calls) = self.tool_calls {
            map.serialize_entry("tool_calls", tool_calls)?;
        }
        if let Some(ref tool_call_id) = self.tool_call_id {
            map.serialize_entry("tool_call_id", tool_call_id)?;
        }
        if let Some(ref name) = self.name {
            map.serialize_entry("name", name)?;
        }
        if let Some(ref reasoning) = self.reasoning {
            map.serialize_entry("reasoning", reasoning)?;
        }
        
        map.end()
    }
}

impl<'de> Deserialize<'de> for Message {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct MessageVisitor;

        impl<'de> Visitor<'de> for MessageVisitor {
            type Value = Message;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a message object")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Message, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut role = String::new();
                let mut content = Vec::new();
                let mut tool_calls: Option<Vec<ToolCall>> = None;
                let mut tool_call_id: Option<String> = None;
                let mut name: Option<String> = None;
                let mut reasoning: Option<String> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "role" => {
                            role = map.next_value()?;
                        }
                        "content" => {
                            // Try to deserialize as string first (backward compatibility)
                            let content_value: serde_json::Value = map.next_value()?;
                            match content_value {
                                serde_json::Value::String(s) => {
                                    // Backward compatible: string content becomes a text part
                                    content.push(ContentPart::Text(s));
                                }
                                serde_json::Value::Array(arr) => {
                                    // New format: array of content parts
                                    for item in arr {
                                        let part: ContentPart = serde_json::from_value(item)
                                            .map_err(de::Error::custom)?;
                                        content.push(part);
                                    }
                                }
                                serde_json::Value::Null => {
                                    // Empty content
                                    content.push(ContentPart::Text(String::new()));
                                }
                                _ => {
                                    // Try to deserialize directly as ContentPart
                                    let part: ContentPart = serde_json::from_value(content_value)
                                        .map_err(de::Error::custom)?;
                                    content.push(part);
                                }
                            }
                        }
                        "tool_calls" => {
                            tool_calls = Some(map.next_value()?);
                        }
                        "tool_call_id" => {
                            tool_call_id = Some(map.next_value()?);
                        }
                        "name" => {
                            name = Some(map.next_value()?);
                        }
                        "reasoning" | "reasoning_content" => {
                            reasoning = Some(map.next_value()?);
                        }
                        _ => {
                            // Skip unknown fields
                            let _: de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                Ok(Message {
                    role,
                    content,
                    tool_calls,
                    tool_call_id,
                    name,
                    reasoning,
                })
            }
        }

        deserializer.deserialize_map(MessageVisitor)
    }
}

impl std::fmt::Display for ContentPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentPart::Text(text) => write!(f, "{}", text),
            ContentPart::ImageUrl { url } => write!(f, "[image: {}]", url.chars().take(50).collect::<String>()),
        }
    }
}

impl Message {
    /// Get text content concatenated
    pub fn text_only(&self) -> String {
        self.content
            .iter()
            .filter_map(|part| {
                if let ContentPart::Text(ref text) = part {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Create a new message with text content
    pub fn text(role: &str, content: &str) -> Self {
        Message {
            role: role.to_string(),
            content: vec![ContentPart::Text(content.to_string())],
            ..Default::default()
        }
    }
}

/// Tool call structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionCall,
}

/// Function call structure within a tool call
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

// ============================================================================
// Tool Argument Types
// ============================================================================

fn default_pattern() -> String {
    "*".to_string()
}

#[derive(Debug, Deserialize)]
pub struct ReadFileArgs {
    pub file_path: String,
}

#[derive(Debug, Deserialize)]
pub struct WriteFileArgs {
    pub file_path: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct ListFilesArgs {
    #[serde(default = "default_pattern")]
    pub pattern: String,
}

#[derive(Debug, Deserialize)]
pub struct EditFileArgs {
    pub file_path: String,
    pub old_content: String,
    pub new_content: String,
}

#[derive(Debug, Deserialize)]
pub struct SwitchModelArgs {
    pub model: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct RunCommandArgs {
    pub command: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchFilesArgs {
    #[serde(default)]
    pub query: String,
    #[serde(default = "default_pattern")]
    pub pattern: String,
    #[serde(default)]
    pub regex: bool,
    #[serde(default)]
    pub case_insensitive: bool,
    #[serde(default)]
    pub max_results: u32,
}

#[derive(Debug, Deserialize)]
pub struct OpenFileArgs {
    pub file_path: String,
    #[serde(default)]
    pub offset: usize,
    #[serde(default)]
    pub end_line: usize,
}
