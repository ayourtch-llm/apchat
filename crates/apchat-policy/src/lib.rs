use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use apchat_vty::print_heart_yellow;

/// Types of actions that can be governed by policies
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// Reading file contents
    FileRead,
    /// Writing to a file (create or overwrite)
    FileWrite,
    /// Editing an existing file
    FileEdit,
    /// Deleting a file
    FileDelete,
    /// Executing a shell command
    CommandExecution,
    /// Planning batch edits
    PlanEdits,
    /// Applying a batch edit plan
    ApplyEditPlan,
    /// Deleting a memory from persistent storage
    MemoryDelete,
    /// Storing a new memory
    MemoryStore,
    /// Querying/searching memories
    MemoryQuery,
    /// Updating an existing memory
    MemoryUpdate,
    /// Listing memories
    MemoryList,
    /// Making network requests
    NetworkRequest,
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionType::FileRead => write!(f, "file_read"),
            ActionType::FileWrite => write!(f, "file_write"),
            ActionType::FileEdit => write!(f, "file_edit"),
            ActionType::FileDelete => write!(f, "file_delete"),
            ActionType::CommandExecution => write!(f, "command_execution"),
            ActionType::PlanEdits => write!(f, "plan_edits"),
            ActionType::ApplyEditPlan => write!(f, "apply_edit_plan"),
            ActionType::MemoryDelete => write!(f, "memory_delete"),
            ActionType::MemoryStore => write!(f, "memory_store"),
            ActionType::MemoryQuery => write!(f, "memory_query"),
            ActionType::MemoryUpdate => write!(f, "memory_update"),
            ActionType::MemoryList => write!(f, "memory_list"),
            ActionType::NetworkRequest => write!(f, "network_request"),
        }
    }
}

/// Policy decision for an action
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    /// Allow the action without asking
    Allow,
    /// Deny the action without asking
    Deny,
    /// Ask the user for confirmation
    Ask,
}

impl std::fmt::Display for Decision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Decision::Allow => write!(f, "allow"),
            Decision::Deny => write!(f, "deny"),
            Decision::Ask => write!(f, "ask"),
        }
    }
}

/// A single policy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Type of action this rule applies to
    pub action: ActionType,
    /// Pattern to match against the target (glob for files, string pattern for commands)
    pub pattern: String,
    /// Decision to make when this rule matches
    pub decision: Decision,
    /// Optional description explaining the rule
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl PolicyRule {
    pub fn new(action: ActionType, pattern: String, decision: Decision) -> Self {
        Self {
            action,
            pattern,
            decision,
            description: None,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Check if this rule matches the given action and target
    pub fn matches(&self, action: &ActionType, target: &str) -> bool {
        if &self.action != action {
            return false;
        }

        // For file operations, use glob matching
        match action {
            ActionType::FileRead
            | ActionType::FileWrite
            | ActionType::FileEdit
            | ActionType::FileDelete => {
                // Simple glob matching - we can enhance this later
                glob_match(&self.pattern, target)
            }
            ActionType::CommandExecution => {
                // For commands, use prefix matching or wildcards
                command_match(&self.pattern, target)
            }
            ActionType::PlanEdits |
             ActionType::ApplyEditPlan |
             ActionType::MemoryStore | ActionType::MemoryQuery |
             ActionType::MemoryUpdate | ActionType::MemoryList |
             ActionType::MemoryDelete | ActionType::NetworkRequest => {
                // These don't have specific targets, match all
                true
            }
        }
    }
}

/// Policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// Default decision when no rules match
    #[serde(default = "default_decision")]
    pub default: Decision,
    /// List of policy rules
    #[serde(default)]
    pub rules: Vec<PolicyRule>,
}

fn default_decision() -> Decision {
    Decision::Ask
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            default: Decision::Ask,
            rules: Vec::new(),
        }
    }
}

impl PolicyConfig {
    /// Create a policy config that allows everything
    pub fn allow_all() -> Self {
        Self {
            default: Decision::Allow,
            rules: Vec::new(),
        }
    }

    /// Load policy from TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: PolicyConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save policy to TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Evaluate an action against the policy
    pub fn evaluate(&self, action: &ActionType, target: &str) -> Decision {
        // Find the first matching rule
        for rule in &self.rules {
            if rule.matches(action, target) {
                return rule.decision.clone();
            }
        }
        // No matching rule, use default
        self.default.clone()
    }

    /// Add a new rule to the policy
    pub fn add_rule(&mut self, rule: PolicyRule) {
        self.rules.push(rule);
    }

    /// Check if a rule already exists for the given action and target
    pub fn has_rule_for(&self, action: &ActionType, target: &str) -> bool {
        self.rules.iter().any(|rule| rule.matches(action, target))
    }
}

/// Policy manager that handles policy loading, evaluation, and learning
#[derive(Clone, Debug)]
pub struct PolicyManager {
    config: Arc<RwLock<PolicyConfig>>,
    policy_file: Option<PathBuf>,
    learn_mode: bool,
}

impl PolicyManager {
    /// Create a new policy manager with default (ask everything) policy
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(PolicyConfig::default())),
            policy_file: None,
            learn_mode: false,
        }
    }

    /// Create a policy manager that allows everything (auto-pilot mode)
    pub fn allow_all() -> Self {
        Self {
            config: Arc::new(RwLock::new(PolicyConfig::allow_all())),
            policy_file: None,
            learn_mode: false,
        }
    }

    /// Create a policy manager from a file
    pub fn from_file<P: AsRef<Path>>(path: P, learn_mode: bool) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        let config = if path_buf.exists() {
            PolicyConfig::load_from_file(&path_buf)?
        } else {
            // Create default policy file if it doesn't exist
            let config = PolicyConfig::default();
            if let Some(parent) = path_buf.parent() {
                std::fs::create_dir_all(parent)?;
            }
            config.save_to_file(&path_buf)?;
            print_heart_yellow(&format!("📋 Created default policy file: {}", path_buf.display()), true);
            config
        };

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            policy_file: Some(path_buf),
            learn_mode,
        })
    }

    /// Evaluate an action against the policy
    pub fn evaluate(&self, action: &ActionType, target: &str) -> Decision {
        let config = self.config.read().unwrap();
        config.evaluate(action, target)
    }

    /// Learn from a user decision (saves to policy file if in learn mode)
    pub fn learn(&self, action: ActionType, target: String, decision: Decision, reason: Option<String>) -> Result<()> {
        if !self.learn_mode {
            return Ok(());
        }

        let mut config = self.config.write().unwrap();

        // Don't add duplicate rules
        if config.has_rule_for(&action, &target) {
            return Ok(());
        }

        // Create a new rule based on the user's decision
        let description = if let Some(ref reason_text) = reason {
            format!("Learned from user decision: {}", reason_text)
        } else {
            "Learned from user decision".to_string()
        };

        let rule = PolicyRule::new(action.clone(), target.clone(), decision.clone())
            .with_description(description);

        config.add_rule(rule);

        // Save to file if we have a policy file path
        if let Some(ref path) = self.policy_file {
            config.save_to_file(path)?;
            if let Some(reason_text) = reason {
                print_heart_yellow(
                    &format!("📚 Learned policy: {} {} -> {} (reason: {})",
                    action, target, decision, reason_text),
                    true
                );
            } else {
                print_heart_yellow(
                    &format!("📚 Learned policy: {} {} -> {}",
                    action, target, decision),
                    true
                );
            }
        }

        Ok(())
    }

    /// Check if the policy manager is in allow-all mode (auto-confirm enabled)
    pub fn is_allow_all(&self) -> bool {
        let config = self.config.read().unwrap();
        matches!(config.default, Decision::Allow)
    }

    /// Check if learning mode is enabled
    pub fn is_learning(&self) -> bool {
        self.learn_mode
    }

    /// Export current policy to a file
    pub fn export_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let config = self.config.read().unwrap();
        config.save_to_file(path)
    }
}

impl Default for PolicyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple glob matching implementation
fn glob_match(pattern: &str, target: &str) -> bool {
    // Handle common glob patterns
    if pattern == "**" || pattern == "*" {
        return true;
    }

    // Convert to consistent path format
    let pattern = pattern.replace('\\', "/");
    let target = target.replace('\\', "/");

    // Handle ** wildcard for recursive matching
    if pattern.contains("**") {
        let parts: Vec<&str> = pattern.split("**").collect();
        if parts.len() == 2 {
            let prefix = parts[0];
            let suffix = parts[1].trim_start_matches('/');

            let prefix_match = if prefix.is_empty() {
                true
            } else {
                target.starts_with(prefix)
            };

            let suffix_match = if suffix.is_empty() {
                true
            } else if suffix == "*.rs" && prefix.is_empty() {
                // Pattern like "**/*.rs" - match anything ending with ".rs"
                target.ends_with(".rs")
            } else if suffix.starts_with('.') && prefix.is_empty() {
                // Pattern like "**/*.ext" - match anything ending with extension
                target.ends_with(suffix)
            } else if suffix.starts_with('*') && suffix.contains('.') && prefix.is_empty() {
                // Handle patterns like "**/*.ext" where suffix after trim is "*.ext"
                let ext = suffix.split('.').nth(1).unwrap_or("");
                target.ends_with(&format!(".{}", ext))
            } else {
                // Check if target ends with suffix OR contains suffix after a slash
                target.ends_with(&suffix) || 
                target.contains(&format!("/{}", suffix))
            };

            return prefix_match && suffix_match;
        }
    }

    // Simple * wildcard matching
    if pattern.contains('*') && !pattern.contains("**") {
        if let Some(star_pos) = pattern.find('*') {
            let prefix = &pattern[..star_pos];
            let suffix = &pattern[star_pos + 1..];
            
            return target.starts_with(prefix) && target.ends_with(suffix);
        }
    }

    // Exact match
    pattern == target
}

/// Simple command matching implementation
fn command_match(pattern: &str, command: &str) -> bool {
    // Support wildcards in command patterns
    if pattern == "*" {
        return true;
    }

    // Handle "command *" pattern (e.g., "cargo *")
    if pattern.ends_with(" *") {
        let prefix = pattern.trim_end_matches(" *");
        return command.starts_with(prefix);
    }

    // Handle "* command" pattern
    if pattern.starts_with("* ") {
        let suffix = pattern.trim_start_matches("* ");
        return command.ends_with(suffix);
    }

    // Exact match
    pattern == command
}

/// Simple URL matching implementation
fn url_match(pattern: &str, url: &str) -> bool {
    // Support wildcards in URL patterns
    if pattern == "*" {
        return true;
    }

    // Handle domain patterns (e.g., "*.example.com")
    if pattern.starts_with("*.") {
        let domain = pattern.trim_start_matches("*.");
        // Check if URL contains the domain
        return url.contains(domain);
    }

    // Handle prefix patterns (e.g., "https://api.example.com/*")
    if pattern.ends_with("/*") || pattern.ends_with("*") {
        let prefix = pattern.trim_end_matches('*').trim_end_matches('/');
        return url.starts_with(prefix);
    }

    // Exact match
    pattern == url
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // glob_match tests
    // ---------------------------------------------------------------

    #[test]
    fn test_glob_match_double_star_extension() {
        assert!(glob_match("**/*.rs", "src/main.rs"));
        assert!(glob_match("**/*.rs", "src/tools/system.rs"));
        assert!(glob_match("**/*.rs", "deeply/nested/path/file.rs"));
        assert!(!glob_match("**/*.rs", "README.md"));
        assert!(!glob_match("**/*.rs", "src/main.py"));
    }

    #[test]
    fn test_glob_match_prefix_double_star() {
        assert!(glob_match("src/**", "src/main.rs"));
        assert!(glob_match("src/**", "src/tools/system.rs"));
        assert!(!glob_match("src/**", "tests/test.rs"));
    }

    #[test]
    fn test_glob_match_single_star() {
        assert!(glob_match("*.md", "README.md"));
        assert!(glob_match("*.toml", "Cargo.toml"));
        assert!(!glob_match("*.md", "src/main.rs"));
        // Note: the simple glob implementation treats * as prefix+suffix match,
        // so "*.md" matches "dir/README.md" (starts_with("") && ends_with(".md"))
        assert!(glob_match("*.md", "dir/README.md"));
    }

    #[test]
    fn test_glob_match_universal_wildcards() {
        assert!(glob_match("**", "anything/at/all.txt"));
        assert!(glob_match("*", "anything"));
    }

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("Cargo.toml", "Cargo.toml"));
        assert!(!glob_match("Cargo.toml", "cargo.toml"));
        assert!(!glob_match("Cargo.toml", "sub/Cargo.toml"));
    }

    #[test]
    fn test_glob_match_backslash_normalization() {
        // Windows-style paths in targets are normalized to forward slashes
        assert!(glob_match("**/*.rs", "src\\main.rs"));
        // Simple pattern with backslash in prefix
        assert!(glob_match("src\\**", "src/main.rs"));
    }

    #[test]
    fn test_glob_match_double_star_other_extensions() {
        assert!(glob_match("**/*.toml", "crates/foo/Cargo.toml"));
        assert!(glob_match("**/*.json", "agents/configs/planner.json"));
        assert!(!glob_match("**/*.json", "agents/configs/planner.toml"));
    }

    // ---------------------------------------------------------------
    // command_match tests
    // ---------------------------------------------------------------

    #[test]
    fn test_command_match_prefix_wildcard() {
        assert!(command_match("cargo *", "cargo build"));
        assert!(command_match("cargo *", "cargo test --all"));
        assert!(command_match("cargo *", "cargo run -- -i"));
        assert!(!command_match("cargo *", "rustc main.rs"));
    }

    #[test]
    fn test_command_match_suffix_wildcard() {
        assert!(command_match("* --force", "git push --force"));
        assert!(command_match("* --force", "rm --force"));
        assert!(!command_match("* --force", "cargo build"));
    }

    #[test]
    fn test_command_match_universal_wildcard() {
        assert!(command_match("*", "any command"));
        assert!(command_match("*", ""));
    }

    #[test]
    fn test_command_match_exact() {
        assert!(command_match("cargo build", "cargo build"));
        assert!(!command_match("cargo build", "cargo test"));
        assert!(!command_match("cargo build", "cargo build --release"));
    }

    // ---------------------------------------------------------------
    // url_match tests
    // ---------------------------------------------------------------

    #[test]
    fn test_url_match_wildcard() {
        assert!(url_match("*", "https://example.com/api/v1"));
    }

    #[test]
    fn test_url_match_domain_wildcard() {
        assert!(url_match("*.example.com", "https://api.example.com/v1"));
        assert!(url_match("*.example.com", "https://example.com/path"));
        assert!(!url_match("*.example.com", "https://other.com/path"));
    }

    #[test]
    fn test_url_match_prefix_pattern() {
        assert!(url_match("https://api.example.com/*", "https://api.example.com/v1/data"));
        assert!(!url_match("https://api.example.com/*", "https://other.com/v1"));
    }

    #[test]
    fn test_url_match_exact() {
        assert!(url_match("https://example.com", "https://example.com"));
        assert!(!url_match("https://example.com", "https://other.com"));
    }

    // ---------------------------------------------------------------
    // PolicyRule tests
    // ---------------------------------------------------------------

    #[test]
    fn test_policy_rule_new() {
        let rule = PolicyRule::new(
            ActionType::FileRead,
            "*.rs".to_string(),
            Decision::Allow,
        );
        assert_eq!(rule.action, ActionType::FileRead);
        assert_eq!(rule.pattern, "*.rs");
        assert_eq!(rule.decision, Decision::Allow);
        assert!(rule.description.is_none());
    }

    #[test]
    fn test_policy_rule_with_description() {
        let rule = PolicyRule::new(
            ActionType::FileWrite,
            "*.md".to_string(),
            Decision::Allow,
        )
        .with_description("Allow markdown writes".to_string());

        assert_eq!(rule.description, Some("Allow markdown writes".to_string()));
    }

    #[test]
    fn test_policy_rule_matches_file_operations() {
        let rule = PolicyRule::new(
            ActionType::FileEdit,
            "**/*.rs".to_string(),
            Decision::Allow,
        );

        assert!(rule.matches(&ActionType::FileEdit, "src/main.rs"));
        assert!(!rule.matches(&ActionType::FileRead, "src/main.rs")); // wrong action type
        assert!(!rule.matches(&ActionType::FileEdit, "README.md")); // wrong extension
    }

    #[test]
    fn test_policy_rule_matches_command_execution() {
        let rule = PolicyRule::new(
            ActionType::CommandExecution,
            "cargo *".to_string(),
            Decision::Allow,
        );

        assert!(rule.matches(&ActionType::CommandExecution, "cargo build"));
        assert!(!rule.matches(&ActionType::CommandExecution, "rm -rf /"));
        assert!(!rule.matches(&ActionType::FileRead, "cargo build")); // wrong action type
    }

    #[test]
    fn test_policy_rule_matches_actionless_types() {
        // PlanEdits, ApplyEditPlan, Memory*, NetworkRequest match all targets
        let rule = PolicyRule::new(
            ActionType::PlanEdits,
            "anything".to_string(),
            Decision::Allow,
        );
        assert!(rule.matches(&ActionType::PlanEdits, "something"));
        assert!(rule.matches(&ActionType::PlanEdits, ""));

        let rule = PolicyRule::new(
            ActionType::MemoryStore,
            "x".to_string(),
            Decision::Deny,
        );
        assert!(rule.matches(&ActionType::MemoryStore, "any target"));

        let rule = PolicyRule::new(
            ActionType::NetworkRequest,
            "x".to_string(),
            Decision::Ask,
        );
        assert!(rule.matches(&ActionType::NetworkRequest, "https://example.com"));
    }

    #[test]
    fn test_policy_rule_matches_file_delete() {
        let rule = PolicyRule::new(
            ActionType::FileDelete,
            "**/*.tmp".to_string(),
            Decision::Allow,
        );
        assert!(rule.matches(&ActionType::FileDelete, "build/output.tmp"));
        assert!(!rule.matches(&ActionType::FileDelete, "src/main.rs"));
    }

    // ---------------------------------------------------------------
    // PolicyConfig tests
    // ---------------------------------------------------------------

    #[test]
    fn test_policy_config_default() {
        let config = PolicyConfig::default();
        assert_eq!(config.default, Decision::Ask);
        assert!(config.rules.is_empty());
    }

    #[test]
    fn test_policy_config_allow_all() {
        let config = PolicyConfig::allow_all();
        assert_eq!(config.default, Decision::Allow);
        assert!(config.rules.is_empty());
    }

    #[test]
    fn test_policy_config_evaluate_default_when_no_rules() {
        let config = PolicyConfig::default();
        assert_eq!(config.evaluate(&ActionType::FileRead, "any_file.rs"), Decision::Ask);

        let config = PolicyConfig::allow_all();
        assert_eq!(config.evaluate(&ActionType::FileRead, "any_file.rs"), Decision::Allow);
    }

    #[test]
    fn test_policy_config_evaluate_first_matching_rule_wins() {
        let mut config = PolicyConfig::default();
        config.add_rule(PolicyRule::new(
            ActionType::FileWrite,
            "**/*.rs".to_string(),
            Decision::Allow,
        ));
        config.add_rule(PolicyRule::new(
            ActionType::FileWrite,
            "**/*.rs".to_string(),
            Decision::Deny,
        ));

        // First rule should win
        assert_eq!(config.evaluate(&ActionType::FileWrite, "src/main.rs"), Decision::Allow);
    }

    #[test]
    fn test_policy_config_evaluate_multiple_action_types() {
        let mut config = PolicyConfig::default();
        config.add_rule(PolicyRule::new(
            ActionType::FileWrite,
            "**/*.md".to_string(),
            Decision::Allow,
        ));
        config.add_rule(PolicyRule::new(
            ActionType::CommandExecution,
            "rm *".to_string(),
            Decision::Deny,
        ));
        config.add_rule(PolicyRule::new(
            ActionType::FileRead,
            "**".to_string(),
            Decision::Allow,
        ));

        assert_eq!(config.evaluate(&ActionType::FileWrite, "README.md"), Decision::Allow);
        assert_eq!(config.evaluate(&ActionType::CommandExecution, "rm file.txt"), Decision::Deny);
        assert_eq!(config.evaluate(&ActionType::FileRead, "src/main.rs"), Decision::Allow);
        // Falls through to default
        assert_eq!(config.evaluate(&ActionType::FileEdit, "src/main.rs"), Decision::Ask);
    }

    #[test]
    fn test_policy_config_has_rule_for() {
        let mut config = PolicyConfig::default();
        assert!(!config.has_rule_for(&ActionType::FileRead, "src/main.rs"));

        config.add_rule(PolicyRule::new(
            ActionType::FileRead,
            "**/*.rs".to_string(),
            Decision::Allow,
        ));
        assert!(config.has_rule_for(&ActionType::FileRead, "src/main.rs"));
        assert!(!config.has_rule_for(&ActionType::FileWrite, "src/main.rs"));
        assert!(!config.has_rule_for(&ActionType::FileRead, "README.md"));
    }

    #[test]
    fn test_policy_config_add_rule() {
        let mut config = PolicyConfig::default();
        assert_eq!(config.rules.len(), 0);

        config.add_rule(PolicyRule::new(
            ActionType::FileRead,
            "*.rs".to_string(),
            Decision::Allow,
        ));
        assert_eq!(config.rules.len(), 1);

        config.add_rule(PolicyRule::new(
            ActionType::FileWrite,
            "*.md".to_string(),
            Decision::Deny,
        ));
        assert_eq!(config.rules.len(), 2);
    }

    // ---------------------------------------------------------------
    // PolicyConfig serialization / file round-trip tests
    // ---------------------------------------------------------------

    #[test]
    fn test_policy_config_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("policy.toml");

        let mut config = PolicyConfig::default();
        config.add_rule(PolicyRule::new(
            ActionType::FileWrite,
            "**/*.rs".to_string(),
            Decision::Allow,
        ));
        config.add_rule(PolicyRule::new(
            ActionType::CommandExecution,
            "cargo *".to_string(),
            Decision::Allow,
        ));

        config.save_to_file(&path).unwrap();
        assert!(path.exists());

        let loaded = PolicyConfig::load_from_file(&path).unwrap();
        assert_eq!(loaded.default, Decision::Ask);
        assert_eq!(loaded.rules.len(), 2);
        assert_eq!(loaded.rules[0].action, ActionType::FileWrite);
        assert_eq!(loaded.rules[0].pattern, "**/*.rs");
        assert_eq!(loaded.rules[0].decision, Decision::Allow);
        assert_eq!(loaded.rules[1].action, ActionType::CommandExecution);
        assert_eq!(loaded.rules[1].pattern, "cargo *");
    }

    #[test]
    fn test_policy_config_load_from_file_nonexistent() {
        let result = PolicyConfig::load_from_file("/nonexistent/path/policy.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_config_load_from_toml_string() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.toml");

        let toml_content = r#"
default = "allow"

[[rules]]
action = "file_read"
pattern = "**/*.rs"
decision = "allow"
description = "Allow reading Rust files"

[[rules]]
action = "command_execution"
pattern = "rm *"
decision = "deny"
"#;
        std::fs::write(&path, toml_content).unwrap();

        let config = PolicyConfig::load_from_file(&path).unwrap();
        assert_eq!(config.default, Decision::Allow);
        assert_eq!(config.rules.len(), 2);
        assert_eq!(config.rules[0].description, Some("Allow reading Rust files".to_string()));
        assert!(config.rules[1].description.is_none());
    }

    #[test]
    fn test_policy_config_load_allow_all_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("allow_all.toml");

        let toml_content = r#"
default = "allow"
rules = []
"#;
        std::fs::write(&path, toml_content).unwrap();

        let config = PolicyConfig::load_from_file(&path).unwrap();
        assert_eq!(config.default, Decision::Allow);
        assert!(config.rules.is_empty());
    }

    // ---------------------------------------------------------------
    // PolicyManager tests
    // ---------------------------------------------------------------

    #[test]
    fn test_policy_manager_new_defaults_to_ask() {
        let pm = PolicyManager::new();
        assert_eq!(pm.evaluate(&ActionType::FileRead, "anything"), Decision::Ask);
        assert!(!pm.is_allow_all());
        assert!(!pm.is_learning());
    }

    #[test]
    fn test_policy_manager_default_trait() {
        let pm = PolicyManager::default();
        assert_eq!(pm.evaluate(&ActionType::FileRead, "anything"), Decision::Ask);
        assert!(!pm.is_allow_all());
    }

    #[test]
    fn test_policy_manager_allow_all() {
        let pm = PolicyManager::allow_all();
        assert_eq!(pm.evaluate(&ActionType::FileRead, "anything"), Decision::Allow);
        assert_eq!(pm.evaluate(&ActionType::CommandExecution, "rm -rf /"), Decision::Allow);
        assert!(pm.is_allow_all());
        assert!(!pm.is_learning());
    }

    #[test]
    fn test_policy_manager_from_file_creates_default_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("subdir/policy.toml");

        // File doesn't exist yet
        assert!(!path.exists());

        let pm = PolicyManager::from_file(&path, false).unwrap();
        // File should now exist with default config
        assert!(path.exists());
        assert_eq!(pm.evaluate(&ActionType::FileRead, "test"), Decision::Ask);
        assert!(!pm.is_learning());
    }

    #[test]
    fn test_policy_manager_from_file_loads_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("policy.toml");

        let toml_content = r#"
default = "allow"

[[rules]]
action = "command_execution"
pattern = "rm *"
decision = "deny"
"#;
        std::fs::write(&path, toml_content).unwrap();

        let pm = PolicyManager::from_file(&path, true).unwrap();
        assert_eq!(pm.evaluate(&ActionType::FileRead, "test.rs"), Decision::Allow);
        assert_eq!(pm.evaluate(&ActionType::CommandExecution, "rm -rf /"), Decision::Deny);
        assert_eq!(pm.evaluate(&ActionType::CommandExecution, "cargo build"), Decision::Allow);
        assert!(pm.is_learning());
    }

    #[test]
    fn test_policy_manager_learn_disabled() {
        let pm = PolicyManager::new(); // learn_mode = false
        let result = pm.learn(
            ActionType::FileRead,
            "src/main.rs".to_string(),
            Decision::Allow,
            None,
        );
        assert!(result.is_ok());
        // Rule should NOT be added because learn mode is off
        assert_eq!(pm.evaluate(&ActionType::FileRead, "src/main.rs"), Decision::Ask);
    }

    #[test]
    fn test_policy_manager_learn_enabled_adds_rule() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("policy.toml");

        let pm = PolicyManager::from_file(&path, true).unwrap();

        pm.learn(
            ActionType::FileRead,
            "**/*.rs".to_string(),
            Decision::Allow,
            Some("Trust Rust files".to_string()),
        ).unwrap();

        // Rule should now be in effect
        assert_eq!(pm.evaluate(&ActionType::FileRead, "src/main.rs"), Decision::Allow);
    }

    #[test]
    fn test_policy_manager_learn_persists_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("policy.toml");

        let pm = PolicyManager::from_file(&path, true).unwrap();
        pm.learn(
            ActionType::CommandExecution,
            "cargo *".to_string(),
            Decision::Allow,
            None,
        ).unwrap();

        // Load a fresh manager from the same file and verify the rule persisted
        let pm2 = PolicyManager::from_file(&path, false).unwrap();
        assert_eq!(pm2.evaluate(&ActionType::CommandExecution, "cargo build"), Decision::Allow);
    }

    #[test]
    fn test_policy_manager_learn_no_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("policy.toml");

        let pm = PolicyManager::from_file(&path, true).unwrap();

        pm.learn(
            ActionType::FileRead,
            "**/*.rs".to_string(),
            Decision::Allow,
            None,
        ).unwrap();

        // Try to learn the same rule again
        pm.learn(
            ActionType::FileRead,
            "**/*.rs".to_string(),
            Decision::Deny,
            None,
        ).unwrap();

        // Should still be Allow (first rule), because the duplicate was skipped
        assert_eq!(pm.evaluate(&ActionType::FileRead, "src/main.rs"), Decision::Allow);

        // Reload and check only one rule
        let loaded = PolicyConfig::load_from_file(&path).unwrap();
        assert_eq!(loaded.rules.len(), 1);
    }

    #[test]
    fn test_policy_manager_export_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let original_path = dir.path().join("original.toml");
        let export_path = dir.path().join("exported.toml");

        let toml_content = r#"
default = "deny"

[[rules]]
action = "file_read"
pattern = "**"
decision = "allow"
"#;
        std::fs::write(&original_path, toml_content).unwrap();

        let pm = PolicyManager::from_file(&original_path, false).unwrap();
        pm.export_to_file(&export_path).unwrap();

        let exported = PolicyConfig::load_from_file(&export_path).unwrap();
        assert_eq!(exported.default, Decision::Deny);
        assert_eq!(exported.rules.len(), 1);
        assert_eq!(exported.rules[0].decision, Decision::Allow);
    }

    #[test]
    fn test_policy_manager_is_allow_all() {
        let pm_default = PolicyManager::new();
        assert!(!pm_default.is_allow_all());

        let pm_allow = PolicyManager::allow_all();
        assert!(pm_allow.is_allow_all());
    }

    // ---------------------------------------------------------------
    // ActionType & Decision Display tests
    // ---------------------------------------------------------------

    #[test]
    fn test_action_type_display() {
        assert_eq!(format!("{}", ActionType::FileRead), "file_read");
        assert_eq!(format!("{}", ActionType::FileWrite), "file_write");
        assert_eq!(format!("{}", ActionType::FileEdit), "file_edit");
        assert_eq!(format!("{}", ActionType::FileDelete), "file_delete");
        assert_eq!(format!("{}", ActionType::CommandExecution), "command_execution");
        assert_eq!(format!("{}", ActionType::PlanEdits), "plan_edits");
        assert_eq!(format!("{}", ActionType::ApplyEditPlan), "apply_edit_plan");
        assert_eq!(format!("{}", ActionType::MemoryDelete), "memory_delete");
        assert_eq!(format!("{}", ActionType::MemoryStore), "memory_store");
        assert_eq!(format!("{}", ActionType::MemoryQuery), "memory_query");
        assert_eq!(format!("{}", ActionType::MemoryUpdate), "memory_update");
        assert_eq!(format!("{}", ActionType::MemoryList), "memory_list");
        assert_eq!(format!("{}", ActionType::NetworkRequest), "network_request");
    }

    #[test]
    fn test_decision_display() {
        assert_eq!(format!("{}", Decision::Allow), "allow");
        assert_eq!(format!("{}", Decision::Deny), "deny");
        assert_eq!(format!("{}", Decision::Ask), "ask");
    }

    // ---------------------------------------------------------------
    // Serde round-trip tests
    // ---------------------------------------------------------------

    #[test]
    fn test_action_type_serde_roundtrip() {
        let actions = vec![
            ActionType::FileRead,
            ActionType::FileWrite,
            ActionType::FileEdit,
            ActionType::FileDelete,
            ActionType::CommandExecution,
            ActionType::PlanEdits,
            ActionType::ApplyEditPlan,
            ActionType::MemoryDelete,
            ActionType::MemoryStore,
            ActionType::MemoryQuery,
            ActionType::MemoryUpdate,
            ActionType::MemoryList,
            ActionType::NetworkRequest,
        ];

        for action in actions {
            let json = serde_json::to_string(&action).unwrap();
            let deserialized: ActionType = serde_json::from_str(&json).unwrap();
            assert_eq!(action, deserialized);
        }
    }

    #[test]
    fn test_decision_serde_roundtrip() {
        for decision in [Decision::Allow, Decision::Deny, Decision::Ask] {
            let json = serde_json::to_string(&decision).unwrap();
            let deserialized: Decision = serde_json::from_str(&json).unwrap();
            assert_eq!(decision, deserialized);
        }
    }

    #[test]
    fn test_policy_config_toml_roundtrip() {
        let mut config = PolicyConfig {
            default: Decision::Deny,
            rules: vec![],
        };
        config.add_rule(
            PolicyRule::new(ActionType::FileRead, "**".to_string(), Decision::Allow)
                .with_description("Read anything".to_string()),
        );
        config.add_rule(
            PolicyRule::new(ActionType::CommandExecution, "cargo *".to_string(), Decision::Allow),
        );

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let deserialized: PolicyConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(deserialized.default, Decision::Deny);
        assert_eq!(deserialized.rules.len(), 2);
        assert_eq!(deserialized.rules[0].description, Some("Read anything".to_string()));
        assert!(deserialized.rules[1].description.is_none());
    }

    // ---------------------------------------------------------------
    // Edge cases
    // ---------------------------------------------------------------

    #[test]
    fn test_evaluate_empty_target() {
        let mut config = PolicyConfig::default();
        config.add_rule(PolicyRule::new(
            ActionType::CommandExecution,
            "*".to_string(),
            Decision::Allow,
        ));
        assert_eq!(config.evaluate(&ActionType::CommandExecution, ""), Decision::Allow);
    }

    #[test]
    fn test_multiple_rules_different_actions_same_pattern() {
        let mut config = PolicyConfig::default();
        config.add_rule(PolicyRule::new(
            ActionType::FileRead,
            "**/*.rs".to_string(),
            Decision::Allow,
        ));
        config.add_rule(PolicyRule::new(
            ActionType::FileWrite,
            "**/*.rs".to_string(),
            Decision::Deny,
        ));

        assert_eq!(config.evaluate(&ActionType::FileRead, "src/main.rs"), Decision::Allow);
        assert_eq!(config.evaluate(&ActionType::FileWrite, "src/main.rs"), Decision::Deny);
    }

    #[test]
    fn test_policy_manager_clone() {
        let pm = PolicyManager::allow_all();
        let pm2 = pm.clone();
        assert!(pm2.is_allow_all());
        assert_eq!(pm2.evaluate(&ActionType::FileRead, "test"), Decision::Allow);
    }

    #[test]
    fn test_policy_config_save_to_file_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new_policy.toml");

        assert!(!path.exists());
        PolicyConfig::default().save_to_file(&path).unwrap();
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("ask"));
    }
}
