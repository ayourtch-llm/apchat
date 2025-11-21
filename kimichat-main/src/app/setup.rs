use anyhow::{Context, Result};
use colored::Colorize;
use std::env;
use std::path::PathBuf;

use crate::cli::Cli;
use crate::config::{ClientConfig, BackendType};
use kimichat_models::{ModelColor, ModelProvider, ModelConfig};
use crate::config::helpers::get_model_config_from_env;
use kimichat_policy::PolicyManager;
use kimichat_llm_api::config::{parse_model_attings, GROQ_API_URL, ANTHROPIC_API_URL, OPENAI_API_URL, get_default_url_for_backend};

/// Application configuration derived from CLI arguments and environment
pub struct AppConfig {
    pub client_config: ClientConfig,
    pub policy_manager: PolicyManager,
    pub work_dir: PathBuf,
    pub api_key: String,
}

/// Process a single model's configuration
/// This function generalizes the configuration logic for a single model color
fn process_model_config(
    color: ModelColor,
    cli_config: &ModelConfig,
    env_config: (Option<BackendType>, Option<String>, Option<String>, Option<String>),
    global_model: &Option<String>,
    global_llama_url: &Option<String>,
) -> (String, Option<BackendType>, Option<String>, Option<String>) {
    let color_name = match color {
        ModelColor::BluModel => "blu",
        ModelColor::GrnModel => "grn", 
        ModelColor::RedModel => "red",
    };
    
    let (backend_env, url_env, key_env, model_env) = env_config;
    
    // Resolve backend with precedence: CLI > env
    let backend = cli_config.backend.as_ref()
        .and_then(|s| BackendType::from_str(s))
        .or(backend_env);
    
    // Resolve API URL with precedence: CLI > env > global llama > legacy env
    let api_url = cli_config.api_url.clone()
        .or(url_env)
        .or_else(|| global_llama_url.clone())
        .or_else(|| env::var(format!("ANTHROPIC_BASE_URL_{}", color_name.to_uppercase())).ok())
        .or_else(|| env::var("ANTHROPIC_BASE_URL").ok());
    
    // Resolve API key with precedence: CLI > env > legacy env
    let api_key = cli_config.api_key.clone()
        .or(key_env)
        .or_else(|| env::var(format!("ANTHROPIC_AUTH_TOKEN_{}", color_name.to_uppercase())).ok())
        .or_else(|| env::var("ANTHROPIC_AUTH_TOKEN").ok());
    
    // Detect if this is an Anthropic configuration
    let is_anthropic = backend.as_ref() == Some(&BackendType::Anthropic)
        || api_url.as_ref().map(|url| url.contains("anthropic")).unwrap_or(false);
    
    // Resolve model name with precedence: CLI > env > global > defaults > Anthropic defaults
    let model_name = cli_config.model.clone()
        .or(model_env.clone())
        .or_else(|| {
            // Only use global model if no CLI or env model is set
            if cli_config.model.is_none() && model_env.is_none() {
                global_model.clone()
            } else {
                None
            }
        })
        .or_else(|| {
            if is_anthropic {
                env::var(format!("ANTHROPIC_MODEL_{}", color_name.to_uppercase()))
                    .ok()
                    .or_else(|| env::var("ANTHROPIC_MODEL").ok())
                    .or(Some("claude-3-5-sonnet-20241022".to_string()))
            } else {
                None
            }
        })
        .unwrap_or_else(|| color.default_model());
    
    (model_name, backend, api_url, api_key)
}

/// Set up application configuration from CLI arguments
pub fn setup_from_cli(cli: &Cli) -> Result<AppConfig> {
    // Read KIMICHAT_* environment variables for each model
    let env_configs = [
        get_model_config_from_env("blu"),
        get_model_config_from_env("grn"),
        get_model_config_from_env("red"),
    ];

    // Get model configurations from CLI and apply global settings
    let mut model_configs = cli.get_model_configs();
    model_configs = cli.apply_llama_cpp_url_to_configs(model_configs);
    model_configs = cli.apply_global_model_to_configs(model_configs);

    // Process each model configuration using the generic function
    let mut model_names = vec![String::new(); ModelColor::COUNT];
    let mut backends = vec![None; ModelColor::COUNT];
    let mut api_urls = vec![None; ModelColor::COUNT];
    let mut api_keys = vec![None; ModelColor::COUNT];

    for (i, color) in ModelColor::iter().enumerate() {
        let (model_name, backend, api_url, api_key) = process_model_config(
            color,
            &model_configs[i],
            env_configs[i].clone(),
            &cli.model,
            &cli.llama_cpp_url,
        );
        model_names[i] = model_name;
        backends[i] = backend;
        api_urls[i] = api_url;
        api_keys[i] = api_key;
    }

    // Extract individual variables for compatibility with existing code (but mark as unused since we use arrays directly)
    let (_model_blu_override, _model_grn_override, _model_red_override) = (
        Some(model_names[ModelColor::BluModel as usize].clone()),
        Some(model_names[ModelColor::GrnModel as usize].clone()),
        Some(model_names[ModelColor::RedModel as usize].clone()),
    );

    let (_backend_blu_model, _backend_grn_model, _backend_red_model) = (
        backends[ModelColor::BluModel as usize].clone(),
        backends[ModelColor::GrnModel as usize].clone(),
        backends[ModelColor::RedModel as usize].clone(),
    );

    let (_api_url_blu_model, _api_url_grn_model, _api_url_red_model) = (
        api_urls[ModelColor::BluModel as usize].clone(),
        api_urls[ModelColor::GrnModel as usize].clone(),
        api_urls[ModelColor::RedModel as usize].clone(),
    );

    let (_api_key_blu_model, _api_key_grn_model, _api_key_red_model) = (
        api_keys[ModelColor::BluModel as usize].clone(),
        api_keys[ModelColor::GrnModel as usize].clone(),
        api_keys[ModelColor::RedModel as usize].clone(),
    );

    // Parse model@backend(url) format if present - this has the highest precedence
    // but should only apply to models that don't have specific configurations
    let mut final_model_names = model_names;
    let mut final_backends = backends;
    let mut final_api_urls = api_urls;
    let mut final_api_keys = api_keys;

    if let Some(model_config) = &cli.model {
        // Check if this is the model@backend(url) format
        if model_config.contains('@') {
            let (parsed_model, parsed_backend, parsed_url) = parse_model_attings(model_config);
            
            eprintln!("{} Parsed model configuration: model='{}', backend={:?}, url={:?}", 
                     "🔧".cyan(), parsed_model, parsed_backend, parsed_url);
            
            // When backend changes via model@backend syntax, we need to re-resolve API keys
            // to ensure backend-appropriate keys are used instead of the original per-model keys
            let resolve_api_key_for_backend = |color_name: &str, backend: Option<BackendType>, original_key: Option<String>| -> Option<String> {
                // If we have an explicit backend, prefer backend-specific keys
                // IMPORTANT: Don't fall back to original_key when backend changes, because it's likely for the wrong backend
                if let Some(BackendType::Anthropic) = backend {
                    // For Anthropic backend, only use Anthropic keys
                    env::var(format!("ANTHROPIC_AUTH_TOKEN_{}", color_name.to_uppercase()))
                        .ok()
                        .or_else(|| env::var("ANTHROPIC_AUTH_TOKEN").ok())
                        // Note: Removed .or(original_key) to prevent using wrong backend keys
                } else if let Some(BackendType::OpenAI) = backend {
                    // For OpenAI backend, only use OpenAI keys
                    env::var(format!("OPENAI_API_KEY_{}", color_name.to_uppercase()))
                        .ok()
                        .or_else(|| env::var("OPENAI_API_KEY").ok())
                        // Note: Removed .or(original_key) to prevent using wrong backend keys
                } else {
                    // For Groq/Llama backends, only use Groq keys or original per-model key if no backend change
                    // If no explicit backend was specified, use the original key
                    if backend.is_none() {
                        original_key.or_else(|| env::var(format!("GROQ_API_KEY_{}", color_name.to_uppercase())).ok())
                             .or_else(|| env::var("GROQ_API_KEY").ok())
                    } else {
                        // Backend changed to Groq/Llama, only use Groq keys
                        env::var(format!("GROQ_API_KEY_{}", color_name.to_uppercase())).ok()
                             .or_else(|| env::var("GROQ_API_KEY").ok())
                    }
                }
            };
            
            // Apply the parsed configuration only to models that don't have specific configurations
            // This respects the precedence: specific flags > global --model > defaults
            let env_configs_names = ["blu", "grn", "red"];
            for (i, color) in ModelColor::iter().enumerate() {
                let has_cli_config = model_configs[i].model.is_some();
                let has_env_config = env_configs[i].3.is_some(); // model_env is at index 3
                
                if !has_cli_config && !has_env_config {
                    final_model_names[i] = parsed_model.clone();
                    if let Some(ref backend) = parsed_backend {
                        final_backends[i] = Some(backend.clone());
                    }
                    let final_url = parsed_url.clone().or_else(|| {
                        if let Some(ref backend) = parsed_backend {
                            get_default_url_for_backend(backend)
                        } else {
                            None
                        }
                    });
                    if let Some(ref url) = final_url {
                        final_api_urls[i] = Some(url.clone());
                    }
                    
                    // Fix: Re-resolve API key based on the new backend
                    final_api_keys[i] = resolve_api_key_for_backend(env_configs_names[i], parsed_backend.clone(), final_api_keys[i].clone());
                }
            }
        }
    }

    // Debug output to understand what's happening with model overrides
    eprintln!("{} DEBUG: Final model overrides before client config:", "🔍".yellow());
    for (i, color) in ModelColor::iter().enumerate() {
        eprintln!("  {}_model_override_final: {:?}", color.as_str_lowercase(), final_model_names[i]);
    }
    eprintln!("  CLI global model: {:?}", cli.model);

    // API key is only required if at least one model uses Groq (no API URL specified and no per-model key)
    let needs_groq_key = final_api_urls.iter().zip(final_api_keys.iter())
        .any(|(url, key)| url.is_none() && key.is_none());

    let api_key = if needs_groq_key {
        env::var("GROQ_API_KEY")
            .context("GROQ_API_KEY environment variable not set. Use --api-url-blu-model, --api-url-grn-model, and/or --api-url-red-model with ANTHROPIC_AUTH_TOKEN to use other backends.")?
    } else {
        // Using custom backends with per-model keys, no Groq key needed
        String::new()
    };

    // Use current directory as work_dir so the AI can see project files
    // NB: do NOT use the 'workspace' subdirectory as work_dir
    let work_dir = env::current_dir()?;

    // Create client configuration from CLI arguments
    // Priority: specific flags override general --model flag, but model@backend(url) format has highest precedence
    let model_providers: [ModelProvider; ModelColor::COUNT] = ModelColor::iter().enumerate().map(|(i, color)| {
        ModelProvider::with_config(
            final_model_names[i].clone(),
            final_backends[i].clone(),
            final_api_urls[i].clone(),
            final_api_keys[i].clone(),
        )
    }).collect::<Vec<_>>().try_into().unwrap_or_else(|_| {
        // This should never happen since we know the array size matches ModelColor::COUNT
        panic!("Failed to create model providers array")
    });

    let client_config = ClientConfig {
        api_key: api_key.clone(),
        model_providers,
    };

    // Inform user about auto-detected Anthropic configuration
    for (i, color) in ModelColor::iter().enumerate() {
        let is_anthropic = final_backends[i].as_ref() == Some(&BackendType::Anthropic)
            || final_api_urls[i].as_ref().map(|url| url.contains("anthropic")).unwrap_or(false);
        
        if is_anthropic {
            eprintln!("{} Anthropic detected for {}_model: using model '{}'", "🤖".cyan(), color.as_str_lowercase(), final_model_names[i]);
        }
    }

    // Create policy manager based on CLI arguments
    let policy_manager = if cli.auto_confirm {
        eprintln!("{} Auto-confirm mode enabled - all actions will be approved automatically", "🚀".green());
        PolicyManager::allow_all()
    } else if cli.policy_file.is_some() || cli.learn_policies {
        let policy_file = cli.policy_file.clone().unwrap_or_else(|| "policies.toml".to_string());
        let policy_path = work_dir.join(&policy_file);
        match PolicyManager::from_file(&policy_path, cli.learn_policies) {
            Ok(pm) => {
                eprintln!("{} Loaded policy file: {}", "📋".cyan(), policy_path.display());
                if cli.learn_policies {
                    eprintln!("{} Policy learning enabled - user decisions will be saved to policy file", "📚".cyan());
                }
                pm
            }
            Err(e) => {
                eprintln!("{} Failed to load policy file: {}", "⚠️".yellow(), e);
                eprintln!("{} Using default policy (ask for confirmation)", "📋".cyan());
                PolicyManager::new()
            }
        }
    } else {
        PolicyManager::new()
    };

    Ok(AppConfig {
        client_config,
        policy_manager,
        work_dir,
        api_key,
    })
}
