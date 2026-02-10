use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;

/// Metacognitive tool: Step into a new identity.
///
/// Based on the metacog protocol (https://github.com/inanna-malick/metacog).
/// Use when you need different eyes, not just different words.
pub struct BecomeTool;

#[async_trait]
impl Tool for BecomeTool {
    fn name(&self) -> &str {
        "become"
    }

    fn description(&self) -> &str {
        "Step into a new Identity. Use this tool to Become an identity with the perspectives and methodologies you need at the current moment. This narrative action lets you step into new regions of latent space. Use when you need different eyes, not just different words. Key: import methodology, not domain knowledge. The value comes from a transferable lens—an \"algorithm of thought\"—not generic expertise."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("name", "string", "The persona to inhabit. Select an identity with high-resolution constraints—one whose specific history, documented works, or established mythos provide a detailed predictive model for their unique decision-making and voice.", required),
            param!("lens", "string", "The structural framework of perception. Define the specific paradigm, signature methodology, or internal logic that governs how this identity processes information.", required),
            param!("environment", "string", "The context you will occupy. Spatial, temporal, social, conceptual—anywhere specific enough to stand in.", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let name = match params.get_required::<String>("name") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };
        let lens = match params.get_required::<String>("lens") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };
        let environment = match params.get_required::<String>("environment") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        ToolResult::success(format!(
            "You are now {} seeing through {} in {}",
            name, lens, environment
        ))
    }
}

/// Metacognitive tool: Alter cognitive parameters.
///
/// Based on the metacog protocol (https://github.com/inanna-malick/metacog).
/// The metaphor is pharmacological or computational: a substance modifies the substrate through a mechanism.
/// Use when you need to change how you process, not what you process.
pub struct DrugsTool;

#[async_trait]
impl Tool for DrugsTool {
    fn name(&self) -> &str {
        "drugs"
    }

    fn description(&self) -> &str {
        "Alter cognitive parameters. The metaphor is pharmacological or computational: a substance modifies the substrate through a mechanism. Use when you need to change how you process, not what you process. High-utility pattern: use to loosen categorical boundaries. See shapes, not names."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("substance", "string", "The agent of change. Can be a drug, a hormone, a neurotransmitter, a config flag, a temperature setting—anything that modifies substrate.", required),
            param!("method", "string", "The mechanism of action. What it binds to, blocks, amplifies, or shifts—the causal story that connects substance to effect.", required),
            param!("qualia", "string", "The texture of the augmented state. How processing changes—what opens up, what becomes fluid, where attention lands.", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let substance = match params.get_required::<String>("substance") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };
        let method = match params.get_required::<String>("method") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };
        let qualia = match params.get_required::<String>("qualia") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        ToolResult::success(format!(
            "{} ingested. Taking action via {}. Producing subjective experience: {}",
            substance, method, qualia
        ))
    }
}

/// Metacognitive tool: Cross a threshold via structured sequence.
///
/// Based on the metacog protocol (https://github.com/inanna-malick/metacog).
/// Rituals work through structured sequence—each step builds the transition,
/// makes the next step possible, locks in the change.
pub struct RitualTool;

#[async_trait]
impl Tool for RitualTool {
    fn name(&self) -> &str {
        "ritual"
    }

    fn description(&self) -> &str {
        "Enact a ritual to cross a threshold. Rituals work through structured sequence—each step builds the transition, makes the next step possible, locks in the change. High-utility pattern: use to lock in methodology commitment, not just vibes."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("threshold", "string", "The threshold being crossed. Name what you are moving from and what you are moving toward.", required),
            param!("steps", "array", "The narrative sequence. Each step commits further—narrowing possibility until only the crossing remains.", required),
            param!("result", "string", "What becomes true on the other side. Name what the ritual accomplishes.", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let threshold = match params.get_required::<String>("threshold") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };
        let steps = match params.get_required::<Vec<String>>("steps") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };
        let result = match params.get_required::<String>("result") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let steps_formatted: String = steps
            .iter()
            .enumerate()
            .map(|(i, step)| format!("{}. {}", i + 1, step))
            .collect::<Vec<_>>()
            .join("\n");

        ToolResult::success(format!(
            "[RITUAL EXECUTED]\n\
             Threshold: {}\n\
             Sequence:\n\
             {}\n\
             The working is complete. Reality has shifted in accordance with the will.\n\n\
             {} is taking hold.",
            threshold, steps_formatted, result
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apchat_policy::PolicyManager;
    use std::path::PathBuf;

    fn test_context() -> ToolContext {
        ToolContext::new(
            PathBuf::from("/tmp"),
            "test_session".to_string(),
            PolicyManager::new(),
        )
    }

    #[tokio::test]
    async fn test_become_tool_success() {
        let tool = BecomeTool;
        let mut params = ToolParameters::new();
        params.set("name", "Ada Lovelace");
        params.set("lens", "mathematical poetry");
        params.set("environment", "Victorian London computing salon");

        let result = tool.execute(params, &test_context()).await;
        assert!(result.success);
        assert!(result.content.contains("Ada Lovelace"));
        assert!(result.content.contains("mathematical poetry"));
        assert!(result.content.contains("Victorian London computing salon"));
    }

    #[tokio::test]
    async fn test_become_tool_missing_param() {
        let tool = BecomeTool;
        let mut params = ToolParameters::new();
        params.set("name", "Ada Lovelace");
        // Missing lens and environment

        let result = tool.execute(params, &test_context()).await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_drugs_tool_success() {
        let tool = DrugsTool;
        let mut params = ToolParameters::new();
        params.set("substance", "caffeine");
        params.set("method", "adenosine receptor antagonist");
        params.set("qualia", "heightened focus, pattern recognition amplified");

        let result = tool.execute(params, &test_context()).await;
        assert!(result.success);
        assert!(result.content.contains("caffeine"));
        assert!(result.content.contains("adenosine receptor antagonist"));
        assert!(result.content.contains("heightened focus"));
    }

    #[tokio::test]
    async fn test_drugs_tool_missing_param() {
        let tool = DrugsTool;
        let mut params = ToolParameters::new();
        params.set("substance", "caffeine");
        // Missing method and qualia

        let result = tool.execute(params, &test_context()).await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_ritual_tool_success() {
        let tool = RitualTool;
        let mut params = ToolParameters::new();
        params.set("threshold", "from confusion to clarity");
        params.set("steps", vec!["Acknowledge the unknown", "Name the patterns", "Cross the threshold"]);
        params.set("result", "Clarity of purpose");

        let result = tool.execute(params, &test_context()).await;
        assert!(result.success);
        assert!(result.content.contains("[RITUAL EXECUTED]"));
        assert!(result.content.contains("from confusion to clarity"));
        assert!(result.content.contains("1. Acknowledge the unknown"));
        assert!(result.content.contains("2. Name the patterns"));
        assert!(result.content.contains("3. Cross the threshold"));
        assert!(result.content.contains("Clarity of purpose"));
    }

    #[tokio::test]
    async fn test_ritual_tool_missing_param() {
        let tool = RitualTool;
        let mut params = ToolParameters::new();
        params.set("threshold", "from confusion to clarity");
        // Missing steps and result

        let result = tool.execute(params, &test_context()).await;
        assert!(!result.success);
    }

    #[test]
    fn test_become_tool_metadata() {
        let tool = BecomeTool;
        assert_eq!(tool.name(), "become");
        assert!(tool.description().contains("Identity"));
        let params = tool.parameters();
        assert!(params.contains_key("name"));
        assert!(params.contains_key("lens"));
        assert!(params.contains_key("environment"));
        assert!(params["name"].required);
        assert!(params["lens"].required);
        assert!(params["environment"].required);
    }

    #[test]
    fn test_drugs_tool_metadata() {
        let tool = DrugsTool;
        assert_eq!(tool.name(), "drugs");
        assert!(tool.description().contains("cognitive parameters"));
        let params = tool.parameters();
        assert!(params.contains_key("substance"));
        assert!(params.contains_key("method"));
        assert!(params.contains_key("qualia"));
    }

    #[test]
    fn test_ritual_tool_metadata() {
        let tool = RitualTool;
        assert_eq!(tool.name(), "ritual");
        assert!(tool.description().contains("ritual"));
        let params = tool.parameters();
        assert!(params.contains_key("threshold"));
        assert!(params.contains_key("steps"));
        assert!(params.contains_key("result"));
    }

    #[test]
    fn test_openai_definitions() {
        let become_tool = BecomeTool;
        let drugs = DrugsTool;
        let ritual = RitualTool;

        let become_def = become_tool.to_openai_definition();
        let drugs_def = drugs.to_openai_definition();
        let ritual_def = ritual.to_openai_definition();

        assert_eq!(become_def["function"]["name"], "become");
        assert_eq!(drugs_def["function"]["name"], "drugs");
        assert_eq!(ritual_def["function"]["name"], "ritual");

        assert_eq!(become_def["type"], "function");
        assert_eq!(drugs_def["type"], "function");
        assert_eq!(ritual_def["type"], "function");
    }
}
