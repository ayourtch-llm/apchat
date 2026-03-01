use apchat_toolcore::ToolRegistry;
use apchat_policy::PolicyManager;
use apchat_tools::*;
use apchat_models::{ModelColor, ModelProvider};
use apchat_vty::{print_heart_red, print_heart_yellow};

pub mod helpers;
pub use helpers::{get_system_prompt, get_api_url, get_api_key, create_model_client, create_client_for_model_color, create_client_for_model_color_with_verbose};

// Re-export types from apchat-llm-api
pub use apchat_llm_api::{BackendType, GROQ_API_URL, normalize_api_url};

/// Feature flags that control optional capabilities.
/// Centralizes the boolean flags that were previously passed as individual parameters.
#[derive(Debug, Clone, Default)]
pub struct FeatureFlags {
    pub early_superpowers: bool,
    pub delayed_instructions: bool,
    pub metacog_tools: bool,
    pub self_regulate: bool,
    pub learning_opportunities: bool,
    pub community_skills: bool,
    pub tiling_tree: bool,
    pub convening_experts: bool,
    pub crafting_instructions: bool,
    pub reviewing_ai_papers: bool,
    pub elements_of_style: bool,
    pub self_edit: bool,
    pub diff_fuzz: bool,
    pub forecasting: bool,
    pub context_mode: bool,
    pub financial_services: bool,
    pub mcp_servers: Vec<String>,
    pub searxng_url: Option<String>,
}

/// Configuration for APChat client
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// API key for authentication (Groq default)
    pub api_key: String,

    /// Model providers indexed by color [blu, grn, red]
    pub model_providers: [ModelProvider; ModelColor::COUNT],
}

impl ClientConfig {
    /// Create a new ClientConfig with default model providers
    pub fn new() -> Self {
        Self {
            api_key: String::new(),
            model_providers: [
                ModelProvider::new(ModelColor::BluModel.default_model()),
                ModelProvider::new(ModelColor::GrnModel.default_model()),
                ModelProvider::new(ModelColor::RedModel.default_model()),
            ],
        }
    }
    
    /// Get model provider for a specific model color
    pub fn get_provider(&self, color: ModelColor) -> &ModelProvider {
        &self.model_providers[color as usize]
    }
    
    /// Get mutable model provider for a specific model color
    pub fn get_provider_mut(&mut self, color: ModelColor) -> &mut ModelProvider {
        &mut self.model_providers[color as usize]
    }
    
    /// Set model provider for a specific model color
    pub fn set_provider(&mut self, color: ModelColor, provider: ModelProvider) {
        self.model_providers[color as usize] = provider;
    }
    
    // Legacy convenience methods for backward compatibility
    /// Get backend for a specific model color
    pub fn get_backend(&self, color: ModelColor) -> Option<&BackendType> {
        self.get_provider(color).backend.as_ref()
    }
    
    /// Set backend for a specific model color
    pub fn set_backend(&mut self, color: ModelColor, backend: Option<BackendType>) {
        self.get_provider_mut(color).backend = backend;
    }
    
    /// Get API URL for a specific model color
    pub fn get_api_url(&self, color: ModelColor) -> Option<&String> {
        self.get_provider(color).api_url.as_ref()
    }
    
    /// Set API URL for a specific model color
    pub fn set_api_url(&mut self, color: ModelColor, url: Option<String>) {
        self.get_provider_mut(color).api_url = url;
    }
    
    /// Get API key for a specific model color
    pub fn get_api_key(&self, color: ModelColor) -> Option<&String> {
        self.get_provider(color).api_key.as_ref()
    }
    
    /// Set API key for a specific model color
    pub fn set_api_key(&mut self, color: ModelColor, key: Option<String>) {
        self.get_provider_mut(color).api_key = key;
    }
    
    /// Get model name for a specific model color
    pub fn get_model_name(&self, color: ModelColor) -> &str {
        &self.get_provider(color).model_name
    }
    
    /// Set model name for a specific model color
    pub fn set_model_name(&mut self, color: ModelColor, model: String) {
        self.get_provider_mut(color).model_name = model;
    }
    
    /// Legacy method: Get model override for a specific model color
    pub fn get_model_override(&self, color: ModelColor) -> Option<&String> {
        Some(&self.get_provider(color).model_name)
    }
    
    /// Legacy method: Set model override for a specific model color
    pub fn set_model_override(&mut self, color: ModelColor, model: Option<String>) {
        if let Some(model) = model {
            self.get_provider_mut(color).model_name = model;
        }
    }
}

#[cfg(test)]
mod tool_registry_integration_tests;

/// Initialize the tool registry with all available tools
pub fn initialize_tool_registry(flags: &FeatureFlags) -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    // Register file operation tools
    registry.register_with_categories(OpenFileTool, vec!["file_ops".to_string()]);
    registry.register_with_categories(ReadFileTool, vec!["file_ops".to_string()]);
    registry.register_with_categories(WriteFileTool, vec!["file_ops".to_string()]);
    registry.register_with_categories(EditFileTool, vec!["file_ops".to_string()]);
    registry.register_with_categories(ListFilesTool, vec!["file_ops".to_string()]);
    registry.register_with_categories(FileCurlyGlanceTool, vec!["file_ops".to_string()]);
    registry.register_with_categories(ReadPdfTool, vec!["file_ops".to_string()]);

    // Register search tools
    registry.register_with_categories(SearchFilesTool, vec!["search".to_string()]);

    // Register system tools
    registry.register_with_categories(RunCommandTool, vec!["system".to_string()]);

    // Register web tools
    registry.register_with_categories(FetchUrlTool, vec!["web".to_string()]);

    // Register model management tools
    registry.register_with_categories(SwitchModelTool::new(), vec!["model_management".to_string()]);
    registry.register_with_categories(PlanEditsTool, vec!["model_management".to_string()]);
    registry.register_with_categories(ApplyEditPlanTool, vec!["model_management".to_string()]);

    // Register LLM tools
    registry.register_with_categories(LlmCallTool, vec!["llm".to_string(), "ai".to_string(), "model".to_string()]);

    // Register iteration control tools
    registry.register_with_categories(RequestMoreIterationsTool, vec!["agent_control".to_string()]);

    // Register skill tools
    registry.register_with_categories(LoadSkillTool, vec!["skills".to_string()]);
    registry.register_with_categories(ListSkillsTool, vec!["skills".to_string()]);
    registry.register_with_categories(FindRelevantSkillsTool, vec!["skills".to_string()]);

    // Register subagent tools
    registry.register_with_categories(LaunchSubagentTool, vec!["agent_control".to_string()]);
    registry.register_with_categories(LaunchSubagentPrettyTool, vec!["agent_control".to_string()]);

    // Register todo/task tracking tools
    registry.register_with_categories(TodoWriteTool::new(), vec!["task_tracking".to_string()]);
    registry.register_with_categories(TodoListTool::new(), vec!["task_tracking".to_string()]);

    // Register PTY terminal tools
    registry.register_with_categories(PtyLaunchTool, vec!["terminal".to_string()]);
    registry.register_with_categories(PtySendKeysTool, vec!["terminal".to_string()]);
    registry.register_with_categories(PtyGetScreenTool, vec!["terminal".to_string()]);
    registry.register_with_categories(PtyGetCursorTool, vec!["terminal".to_string()]);
    registry.register_with_categories(PtyResizeTool, vec!["terminal".to_string()]);
    registry.register_with_categories(PtySetScrollbackTool, vec!["terminal".to_string()]);
    registry.register_with_categories(PtyStartCaptureTool, vec!["terminal".to_string()]);
    registry.register_with_categories(PtyStopCaptureTool, vec!["terminal".to_string()]);
    registry.register_with_categories(PtyListTool, vec!["terminal".to_string()]);
    registry.register_with_categories(PtyKillTool, vec!["terminal".to_string()]);
    registry.register_with_categories(PtyRequestUserInputTool, vec!["terminal".to_string()]);
    registry.register_with_categories(PtySendCredentialKeysTool, vec!["terminal".to_string()]);

    // Register memory tools
    registry.register_with_categories(StoreMemoryTool, vec!["memory".to_string()]);
    registry.register_with_categories(QueryMemoryTool, vec!["memory".to_string()]);
    registry.register_with_categories(UpdateMemoryTool, vec!["memory".to_string()]);
    registry.register_with_categories(DeleteMemoryTool, vec!["memory".to_string()]);
    registry.register_with_categories(ListMemoriesTool, vec!["memory".to_string()]);

    // Register wait/sleep tools
    registry.register_with_categories(LongWaitTool, vec!["system".to_string()]);

    // Register RLM context chunking tool
    registry.register_with_categories(RlmContextChunkTool, vec!["file_ops".to_string(), "rlm".to_string()]);

    // Register scheduled instruction tools (only if enabled via CLI flag)
    if flags.delayed_instructions {
        registry.register_with_categories(AddScheduledInstructionTool, vec!["scheduled_instruction".to_string(), "memory".to_string()]);
        registry.register_with_categories(ListScheduledInstructionsTool, vec!["scheduled_instruction".to_string(), "memory".to_string()]);
        registry.register_with_categories(DeleteScheduledInstructionTool, vec!["scheduled_instruction".to_string(), "memory".to_string()]);
    }

    // Register metacognitive tools (only if enabled via --metacog-tools CLI flag)
    if flags.metacog_tools {
        registry.register_with_categories(BecomeTool, vec!["metacog".to_string()]);
        registry.register_with_categories(DrugsTool, vec!["metacog".to_string()]);
        registry.register_with_categories(RitualTool, vec!["metacog".to_string()]);
    }

    // Register self-regulate tool (only if enabled via --self-regulate CLI flag)
    if flags.self_regulate {
        registry.register_with_categories(SelfRegulateTool, vec!["self_regulate".to_string()]);
    }

    // Register context editing tools (only if enabled via --self-edit CLI flag)
    if flags.self_edit {
        registry.register_with_categories(DeleteItemsTool, vec!["self_edit".to_string()]);
        registry.register_with_categories(EditItemTool, vec!["self_edit".to_string()]);
    }

    // Register differential fuzzing tool (only if enabled via --diff-fuzz CLI flag)
    if flags.diff_fuzz {
        registry.register_with_categories(DiffFuzzTool, vec!["diff_fuzz".to_string(), "testing".to_string()]);
    }

    // Register forecasting tool (only if enabled via --forecasting CLI flag AND compiled with feature)
    #[cfg(feature = "forecasting")]
    if flags.forecasting {
        registry.register_with_categories(
            apchat_tools::forecasting::ForecastTool,
            vec!["forecasting".to_string()],
        );
    }
    #[cfg(not(feature = "forecasting"))]
    if flags.forecasting {
        apchat_vty::print_heart_yellow(
            "Warning: --forecasting flag requires the 'forecasting' feature. Build with `cargo build --features forecasting`",
            true,
        );
    }

    // Register SearXNG web search tool (only if --searxng URL is provided)
    if let Some(ref searxng_url) = flags.searxng_url {
        registry.register_with_categories(
            apchat_tools::searxng::SearxngSearchTool::new(searxng_url.clone()),
            vec!["web".to_string(), "search".to_string()],
        );
        print_heart_red(
            &format!("✓ SearXNG web search enabled ({})", searxng_url),
            true,
        );
    }

    registry
}

/// Register MCP server tools in the tool registry.
///
/// This function is async because it needs to spawn MCP server processes
/// and perform JSON-RPC initialization. It should be called after the
/// tool registry is created and before the first user interaction.
///
/// Returns the list of active MCP clients for lifecycle management.
pub async fn register_mcp_tools(
    registry: &mut ToolRegistry,
    flags: &FeatureFlags,
) -> Vec<std::sync::Arc<apchat_tools::mcp_client::McpClient>> {
    let mut clients = Vec::new();

    // Register context-mode MCP server if enabled
    if flags.context_mode {
        match apchat_tools::mcp_client::start_mcp_server(
            "npx",
            &["-y", "context-mode"],
            "context-mode",
            "ctx_",
        )
        .await
        {
            Ok((client, tools)) => {
                let tool_count = tools.len();
                for tool in tools {
                    registry.register_with_categories(
                        tool,
                        vec!["mcp".to_string(), "context_mode".to_string()],
                    );
                }
                print_heart_red(
                    &format!("✓ context-mode MCP server started ({} tools registered with ctx_ prefix)", tool_count),
                    true,
                );
                clients.push(client);
            }
            Err(e) => {
                print_heart_yellow(
                    &format!("⚠️  Failed to start context-mode MCP server: {}. Is Node.js 18+ installed?", e),
                    true,
                );
            }
        }
    }

    // Register generic MCP servers
    // Note: Command is split on whitespace. tokio::process::Command does NOT use a shell,
    // so shell metacharacters (;, |, &&, $()) are NOT interpreted - they are passed
    // literally as arguments. This is safe from shell injection.
    for (idx, cmd) in flags.mcp_servers.iter().enumerate() {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            print_heart_yellow(
                &format!("⚠️  Empty MCP server command, skipping"),
                true,
            );
            continue;
        }

        let command = parts[0];
        let args: Vec<&str> = parts[1..].to_vec();
        let server_name = format!("mcp-{}", idx);
        let prefix = format!("mcp{}_", idx);

        match apchat_tools::mcp_client::start_mcp_server(
            command,
            &args,
            &server_name,
            &prefix,
        )
        .await
        {
            Ok((client, tools)) => {
                let tool_count = tools.len();
                for tool in tools {
                    registry.register_with_categories(
                        tool,
                        vec!["mcp".to_string()],
                    );
                }
                print_heart_red(
                    &format!("✓ MCP server '{}' started ({} tools registered with {} prefix)", cmd, tool_count, prefix),
                    true,
                );
                clients.push(client);
            }
            Err(e) => {
                print_heart_yellow(
                    &format!("⚠️  Failed to start MCP server '{}': {}", cmd, e),
                    true,
                );
            }
        }
    }

    clients
}
