use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use apchat_models::types::ModelColor;
use apchat_llm_api::client::ChatMessage;
use async_trait::async_trait;
use std::collections::HashMap;

/// Differential fuzzing tool inspired by ctrlcode's derived-context architecture.
///
/// Pipeline:
/// 1. Derive behavioral context from spec + code (system placement, invariants, contracts)
/// 2. Generate fuzz test cases (input + environment scenarios)
/// 3. Check invariants against generated test expectations
/// 4. Analyze divergences and classify as MODEL_BUG / ORACLE_BUG / SPEC_GAP / ENVIRONMENT_MISMATCH
///
/// See: https://codeberg.org/canoozie/ctrlcode
pub struct DiffFuzzTool;

const CONTEXT_DERIVATION_PROMPT: &str = r#"You are a senior systems analyst. Examine the generated code and its specification, then derive the full operational context.

Produce a CONTEXT DERIVATION REPORT with these sections as JSON:

{
  "system_placement": {
    "system_type": "web service | CLI tool | library | etc.",
    "layer": "HTTP handler | business logic | data access | etc.",
    "callers": "what likely calls this code",
    "callees": "what this code likely calls"
  },
  "environmental_constraints": {
    "language": "...",
    "concurrency_model": "async | threaded | single-threaded",
    "resource_sensitivity": "memory | latency | throughput | none"
  },
  "integration_contracts": [
    {
      "system": "External API | Database | etc.",
      "contract": "description of the contract",
      "implicit_requirements": ["timeout handling", "retry semantics", "..."]
    }
  ],
  "behavioral_invariants": [
    "invariant 1: ...",
    "invariant 2: ..."
  ],
  "edge_case_surface": [
    "edge case 1: ...",
    "edge case 2: ..."
  ],
  "implicit_assumptions": [
    {
      "assumption": "...",
      "risk": "SAFE | RISKY | DANGEROUS",
      "explanation": "..."
    }
  ]
}

Be thorough. Derive invariants from both the spec AND what the code implies about its operational context. Output ONLY valid JSON."#;

const FUZZ_GENERATION_PROMPT: &str = r#"You are a fuzz test generator for differential testing. Generate test cases that probe the code against its derived behavioral context.

Generate TWO kinds of test cases:
A) Input Fuzzing — diverse inputs targeting behavioral invariants
B) Environment Fuzzing — simulated environmental conditions (dependency failures, timing, concurrency)

Output a JSON array:
[
  {
    "id": "fuzz_001",
    "type": "input | environment | combined",
    "description": "what this test checks",
    "input": { "key": "value" },
    "expected_behavior": "description of expected outcome derived from invariants",
    "invariants_tested": ["invariant name 1", "invariant name 2"],
    "category": "exploit | explore | cover | stress | environment"
  }
]

CRITICAL: expected_behavior is the DERIVED ORACLE. Derive it strictly from the specification and behavioral invariants. Output ONLY valid JSON."#;

const DIVERGENCE_ANALYSIS_PROMPT: &str = r#"You are a senior software engineer performing root cause analysis on differential fuzzing results.

For each potential divergence between code behavior and derived expectations, determine:

1. SOURCE classification:
   - MODEL_BUG: Code doesn't match what spec and context require
   - ORACLE_BUG: Derived expectation was wrong — code is actually fine
   - SPEC_GAP: Specification doesn't address this case
   - ENVIRONMENT_MISMATCH: Environmental assumptions don't match reality

2. For each finding, provide:
   - diagnosis: what's wrong
   - source: MODEL_BUG | ORACLE_BUG | SPEC_GAP | ENVIRONMENT_MISMATCH
   - confidence: 0.0-1.0
   - impact: what could break in production
   - suggested_fix: minimal change to resolve

Output as JSON:
{
  "findings": [
    {
      "test_id": "fuzz_001",
      "diagnosis": "...",
      "source": "MODEL_BUG",
      "confidence": 0.85,
      "impact": "...",
      "suggested_fix": "..."
    }
  ],
  "summary": {
    "model_bugs": 0,
    "oracle_bugs": 0,
    "spec_gaps": 0,
    "environment_mismatches": 0,
    "quality_score": 0.95
  }
}

Output ONLY valid JSON."#;

#[async_trait]
impl Tool for DiffFuzzTool {
    fn name(&self) -> &str {
        "diff_fuzz"
    }

    fn description(&self) -> &str {
        "Run differential fuzzing on AI-generated code. Derives behavioral invariants from the specification, generates fuzz test cases, and analyzes divergences. Classifies issues as MODEL_BUG, ORACLE_BUG, SPEC_GAP, or ENVIRONMENT_MISMATCH. Inspired by ctrlcode's derived-context fuzzing architecture."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("spec", "string", "The specification or requirement that the code was generated from", required),
            param!("code", "string", "The generated code to test via differential fuzzing", required),
            param!("model_color", "string", "Model color to use for analysis (red, grn, blu). Default: blu", optional),
            param!("max_iterations", "number", "Maximum fuzzing iterations (1-5). Default: 3", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let spec = match params.get_required::<String>("spec") {
            Ok(s) => s,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let code = match params.get_required::<String>("code") {
            Ok(s) => s,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let model_color_str = match params.get_optional::<String>("model_color") {
            Ok(Some(s)) => s,
            Ok(None) => "blu".to_string(),
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let max_iterations = match params.get_optional::<i64>("max_iterations") {
            Ok(Some(v)) => v.clamp(1, 5) as usize,
            Ok(None) => 3,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let model_color = match model_color_str.to_lowercase().as_str() {
            "red" => ModelColor::RedModel,
            "grn" => ModelColor::GrnModel,
            "blu" => ModelColor::BluModel,
            _ => return ToolResult::error(format!(
                "Invalid model color: '{}'. Use 'red', 'grn', or 'blu'",
                model_color_str
            )),
        };

        let client = match context.get_llm_client(&model_color) {
            Some(c) => c,
            None => return ToolResult::error(format!(
                "No LLM client configured for model color: {:?}",
                model_color
            )),
        };

        let mut report = String::new();

        // Stage 1: Context Derivation
        report.push_str("# Differential Fuzzing Report\n\n");
        report.push_str("## Stage 1: Context Derivation\n\n");

        let context_derivation_msg = ChatMessage {
            role: "user".to_string(),
            content: format!(
                "{}\n\n## Specification\n{}\n\n## Generated Code\n```\n{}\n```",
                CONTEXT_DERIVATION_PROMPT, spec, code
            ),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning: None,
        };

        let derived_context = match client.chat_completion(&[context_derivation_msg]).await {
            Ok(response) => {
                report.push_str(&response);
                report.push_str("\n\n");
                response
            }
            Err(e) => return ToolResult::error(format!("Context derivation failed: {}", e)),
        };

        // Stage 2: Fuzz Test Generation (iterative)
        report.push_str("## Stage 2: Fuzz Test Generation & Analysis\n\n");

        let mut all_findings = Vec::new();

        for iteration in 1..=max_iterations {
            report.push_str(&format!("### Iteration {}\n\n", iteration));

            // Generate test cases
            let previous_context = if iteration > 1 {
                let summary: String = all_findings.iter()
                    .map(|f: &String| {
                        if f.len() > 500 { format!("{}...", &f[..500]) } else { f.clone() }
                    })
                    .collect::<Vec<_>>()
                    .join("\n---\n");
                format!("\n\nPrevious findings from earlier iterations (summarized):\n{}", summary)
            } else {
                String::new()
            };

            let fuzz_gen_msg = ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "{}\n\n## Specification\n{}\n\n## Generated Code\n```\n{}\n```\n\n## Derived Context\n{}{}\n\nGenerate 5 test cases.",
                    FUZZ_GENERATION_PROMPT, spec, code, derived_context, previous_context
                ),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                reasoning: None,
            };

            let test_cases = match client.chat_completion(&[fuzz_gen_msg]).await {
                Ok(response) => {
                    report.push_str("**Generated Test Cases:**\n");
                    report.push_str(&response);
                    report.push_str("\n\n");
                    response
                }
                Err(e) => {
                    report.push_str(&format!("Test generation failed: {}\n\n", e));
                    continue;
                }
            };

            // Stage 3: Divergence Analysis
            let analysis_msg = ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "{}\n\n## Specification\n{}\n\n## Generated Code\n```\n{}\n```\n\n## Derived Context\n{}\n\n## Test Cases\n{}\n\nAnalyze these test cases against the code and derived context. Identify any divergences.",
                    DIVERGENCE_ANALYSIS_PROMPT, spec, code, derived_context, test_cases
                ),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                reasoning: None,
            };

            match client.chat_completion(&[analysis_msg]).await {
                Ok(response) => {
                    report.push_str("**Divergence Analysis:**\n");
                    report.push_str(&response);
                    report.push_str("\n\n");
                    all_findings.push(response);
                }
                Err(e) => {
                    report.push_str(&format!("Analysis failed: {}\n\n", e));
                }
            }
        }

        report.push_str("---\n*Differential fuzzing complete. Review findings above for MODEL_BUG, ORACLE_BUG, SPEC_GAP, and ENVIRONMENT_MISMATCH classifications.*\n");

        ToolResult::success(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_fuzz_metadata() {
        let tool = DiffFuzzTool;
        assert_eq!(tool.name(), "diff_fuzz");
        assert!(tool.description().contains("differential fuzzing"));
        assert!(tool.description().contains("MODEL_BUG"));

        let params = tool.parameters();
        assert!(params.contains_key("spec"));
        assert!(params.contains_key("code"));
        assert!(params.contains_key("model_color"));
        assert!(params.contains_key("max_iterations"));
        assert!(params["spec"].required);
        assert!(params["code"].required);
        assert!(!params["model_color"].required);
        assert!(!params["max_iterations"].required);
    }

    #[test]
    fn test_diff_fuzz_openai_definition() {
        let tool = DiffFuzzTool;
        let def = tool.to_openai_definition();
        assert_eq!(def["type"], "function");
        assert_eq!(def["function"]["name"], "diff_fuzz");
        assert!(def["function"]["parameters"]["properties"].get("spec").is_some());
        assert!(def["function"]["parameters"]["properties"].get("code").is_some());
    }

    #[tokio::test]
    async fn test_diff_fuzz_missing_spec() {
        let tool = DiffFuzzTool;
        let ctx = ToolContext::new(
            std::path::PathBuf::from("/tmp"),
            "test_session".to_string(),
            apchat_policy::PolicyManager::new(),
        );

        let mut params = ToolParameters::new();
        params.set("code", "fn main() {}");

        let result = tool.execute(params, &ctx).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("spec"));
    }

    #[tokio::test]
    async fn test_diff_fuzz_missing_code() {
        let tool = DiffFuzzTool;
        let ctx = ToolContext::new(
            std::path::PathBuf::from("/tmp"),
            "test_session".to_string(),
            apchat_policy::PolicyManager::new(),
        );

        let mut params = ToolParameters::new();
        params.set("spec", "write a hello world");

        let result = tool.execute(params, &ctx).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("code"));
    }

    #[tokio::test]
    async fn test_diff_fuzz_invalid_model_color() {
        let tool = DiffFuzzTool;
        let ctx = ToolContext::new(
            std::path::PathBuf::from("/tmp"),
            "test_session".to_string(),
            apchat_policy::PolicyManager::new(),
        );

        let mut params = ToolParameters::new();
        params.set("spec", "write a hello world");
        params.set("code", "fn main() { println!(\"hello\"); }");
        params.set("model_color", "purple");

        let result = tool.execute(params, &ctx).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Invalid model color"));
    }

    #[tokio::test]
    async fn test_diff_fuzz_no_llm_client() {
        let tool = DiffFuzzTool;
        let ctx = ToolContext::new(
            std::path::PathBuf::from("/tmp"),
            "test_session".to_string(),
            apchat_policy::PolicyManager::new(),
        );

        let mut params = ToolParameters::new();
        params.set("spec", "write a hello world");
        params.set("code", "fn main() { println!(\"hello\"); }");
        params.set("model_color", "blu");

        let result = tool.execute(params, &ctx).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("No LLM client"));
    }
}
