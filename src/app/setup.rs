use anyhow::{Context, Result};
use colored::Colorize;
use std::env;
use std::path::PathBuf;

use crate::cli::Cli;
use crate::config::ClientConfig;
use crate::policy::PolicyManager;

/// Application configuration derived from CLI arguments and environment
pub struct AppConfig {
    pub client_config: ClientConfig,
    pub policy_manager: PolicyManager,
    pub work_dir: PathBuf,
    pub api_key: String,
}

/// Set up application configuration from CLI arguments
pub fn setup_from_cli(cli: &Cli) -> Result<AppConfig> {
    // Determine API URLs for each model
    // Priority: specific flags (--api-url-blu-model, --api-url-grn-model, --api-url-red-model) override general flag (--llama-cpp-url)
    // Also check for Anthropic environment variables
    let api_url_blu_model = cli.api_url_blu_model
        .clone()
        .or_else(|| cli.llama_cpp_url.clone())
        .or_else(|| env::var("ANTHROPIC_BASE_URL_BLU").ok())
        .or_else(|| env::var("ANTHROPIC_BASE_URL").ok());

    let api_url_grn_model = cli.api_url_grn_model
        .clone()
        .or_else(|| cli.llama_cpp_url.clone())
        .or_else(|| env::var("ANTHROPIC_BASE_URL_GRN").ok())
        .or_else(|| env::var("ANTHROPIC_BASE_URL").ok());

    let api_url_red_model = cli.api_url_red_model
        .clone()
        .or_else(|| cli.llama_cpp_url.clone())
        .or_else(|| env::var("ANTHROPIC_BASE_URL_RED").ok())
        .or_else(|| env::var("ANTHROPIC_BASE_URL").ok());

    // Check for per-model API keys (for Anthropic or other services)
    let api_key_blu_model = env::var("ANTHROPIC_AUTH_TOKEN_BLU").ok()
        .or_else(|| env::var("ANTHROPIC_AUTH_TOKEN").ok());

    let api_key_grn_model = env::var("ANTHROPIC_AUTH_TOKEN_GRN").ok()
        .or_else(|| env::var("ANTHROPIC_AUTH_TOKEN").ok());

    let api_key_red_model = env::var("ANTHROPIC_AUTH_TOKEN_RED").ok()
        .or_else(|| env::var("ANTHROPIC_AUTH_TOKEN").ok());

    // Auto-detect Anthropic and set appropriate model names if not overridden
    let is_anthropic_blu = api_url_blu_model.as_ref()
        .map(|url| url.contains("anthropic"))
        .unwrap_or(false);
    let is_anthropic_grn = api_url_grn_model.as_ref()
        .map(|url| url.contains("anthropic"))
        .unwrap_or(false);
    let is_anthropic_red = api_url_red_model.as_ref()
        .map(|url| url.contains("anthropic"))
        .unwrap_or(false);

    let model_blu_override = cli.model_blu_model.clone()
        .or_else(|| cli.model.clone())
        .or_else(|| {
            if is_anthropic_blu {
                env::var("ANTHROPIC_MODEL_BLU").ok()
                    .or_else(|| env::var("ANTHROPIC_MODEL").ok())
                    .or(Some("claude-3-5-sonnet-20241022".to_string()))
            } else {
                None
            }
        });

    let model_grn_override = cli.model_grn_model.clone()
        .or_else(|| cli.model.clone())
        .or_else(|| {
            if is_anthropic_grn {
                env::var("ANTHROPIC_MODEL_GRN").ok()
                    .or_else(|| env::var("ANTHROPIC_MODEL").ok())
                    .or(Some("claude-3-5-sonnet-20241022".to_string()))
            } else {
                None
            }
        });

    let model_red_override = cli.model_red_model.clone()
        .or_else(|| cli.model.clone())
        .or_else(|| {
            if is_anthropic_red {
                env::var("ANTHROPIC_MODEL_RED").ok()
                    .or_else(|| env::var("ANTHROPIC_MODEL").ok())
                    .or(Some("claude-3-5-sonnet-20241022".to_string()))
            } else {
                None
            }
        });

    // API key is only required if at least one model uses Groq (no API URL specified and no per-model key)
    let needs_groq_key = (api_url_blu_model.is_none() && api_key_blu_model.is_none())
                      || (api_url_grn_model.is_none() && api_key_grn_model.is_none())
                      || (api_url_red_model.is_none() && api_key_red_model.is_none());

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
    // Priority: specific flags override general --model flag, with auto-detection for Anthropic
    let client_config = ClientConfig {
        api_key: api_key.clone(),
        api_url_blu_model: api_url_blu_model.clone(),
        api_url_grn_model: api_url_grn_model.clone(),
        api_url_red_model: api_url_red_model.clone(),
        api_key_blu_model,
        api_key_grn_model,
        api_key_red_model,
        model_blu_model_override: model_blu_override.clone(),
        model_grn_model_override: model_grn_override.clone(),
        model_red_model_override: model_red_override.clone(),
    };

    // Inform user about auto-detected Anthropic configuration
    if is_anthropic_blu {
        if let Some(model_name) = model_blu_override.as_ref() {
            eprintln!("{} Anthropic detected for blu_model: using model '{}'", "🤖".cyan(), model_name);
        }
    }
    if is_anthropic_grn {
        if let Some(model_name) = model_grn_override.as_ref() {
            eprintln!("{} Anthropic detected for grn_model: using model '{}'", "🤖".cyan(), model_name);
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
