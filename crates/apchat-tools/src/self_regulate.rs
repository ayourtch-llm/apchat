use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use apchat_llm_api::LlmRequestOverrides;
use async_trait::async_trait;
use std::collections::HashMap;

pub struct SelfRegulateTool;

#[async_trait]
impl Tool for SelfRegulateTool {
    fn name(&self) -> &str {
        "self_regulate"
    }

    fn description(&self) -> &str {
        "Adjust LLM request parameters (temperature, top_p, max_tokens) for the next N API calls. Use this to self-regulate: increase temperature for creative tasks, decrease for precise/analytical work, or adjust max_tokens for longer/shorter responses."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("calls", "number", "Number of upcoming LLM calls these overrides apply to (1-50)", required),
            param!("temperature", "number", "Sampling temperature (0.0-2.0). Lower = more focused/deterministic, higher = more creative/random", optional),
            param!("top_p", "number", "Nucleus sampling parameter (0.0-1.0). Lower = consider fewer tokens, higher = consider more", optional),
            param!("max_tokens", "number", "Maximum tokens in the response (1-32768)", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let calls = match params.get_required::<i64>("calls") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        if calls < 1 || calls > 50 {
            return ToolResult::error("calls must be between 1 and 50".to_string());
        }

        let temperature = match params.get_optional::<f64>("temperature") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };
        let top_p = match params.get_optional::<f64>("top_p") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };
        let max_tokens = match params.get_optional::<i64>("max_tokens") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        if temperature.is_none() && top_p.is_none() && max_tokens.is_none() {
            return ToolResult::error("At least one parameter (temperature, top_p, or max_tokens) must be specified".to_string());
        }

        if let Some(t) = temperature {
            if !(0.0..=2.0).contains(&t) {
                return ToolResult::error("temperature must be between 0.0 and 2.0".to_string());
            }
        }

        if let Some(p) = top_p {
            if !(0.0..=1.0).contains(&p) {
                return ToolResult::error("top_p must be between 0.0 and 1.0".to_string());
            }
        }

        if let Some(mt) = max_tokens {
            if mt < 1 || mt > 32768 {
                return ToolResult::error("max_tokens must be between 1 and 32768".to_string());
            }
        }

        let overrides = LlmRequestOverrides {
            temperature,
            top_p,
            max_tokens: max_tokens.map(|v| v as u32),
            remaining_calls: calls as u32,
        };

        let overrides_arc = match &context.llm_overrides {
            Some(arc) => arc,
            None => return ToolResult::error("Self-regulation is not available in this context".to_string()),
        };

        match overrides_arc.lock() {
            Ok(mut guard) => {
                *guard = Some(overrides.clone());
            }
            Err(e) => return ToolResult::error(format!("Failed to set overrides: {}", e)),
        }

        let mut parts = Vec::new();
        if let Some(t) = overrides.temperature {
            parts.push(format!("temperature={:.2}", t));
        }
        if let Some(p) = overrides.top_p {
            parts.push(format!("top_p={:.2}", p));
        }
        if let Some(mt) = overrides.max_tokens {
            parts.push(format!("max_tokens={}", mt));
        }

        ToolResult::success(format!(
            "LLM parameters adjusted for the next {} call(s): {}",
            calls,
            parts.join(", ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apchat_policy::PolicyManager;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    fn test_context_with_overrides() -> ToolContext {
        let mut ctx = ToolContext::new(
            PathBuf::from("/tmp"),
            "test_session".to_string(),
            PolicyManager::new(),
        );
        ctx.llm_overrides = Some(Arc::new(Mutex::new(None)));
        ctx
    }

    fn test_context_without_overrides() -> ToolContext {
        ToolContext::new(
            PathBuf::from("/tmp"),
            "test_session".to_string(),
            PolicyManager::new(),
        )
    }

    #[tokio::test]
    async fn test_self_regulate_temperature() {
        let tool = SelfRegulateTool;
        let ctx = test_context_with_overrides();

        let mut params = ToolParameters::new();
        params.set("calls", 5i64);
        params.set("temperature", 0.8);

        let result = tool.execute(params, &ctx).await;
        assert!(result.success, "Expected success, got: {:?}", result.error);
        assert!(result.content.contains("temperature=0.80"));
        assert!(result.content.contains("5 call(s)"));

        let binding = ctx.llm_overrides.unwrap();
        let guard = binding.lock().unwrap();
        let overrides = guard.as_ref().unwrap();
        assert_eq!(overrides.temperature, Some(0.8));
        assert_eq!(overrides.remaining_calls, 5);
        assert!(overrides.top_p.is_none());
        assert!(overrides.max_tokens.is_none());
    }

    #[tokio::test]
    async fn test_self_regulate_all_params() {
        let tool = SelfRegulateTool;
        let ctx = test_context_with_overrides();

        let mut params = ToolParameters::new();
        params.set("calls", 3i64);
        params.set("temperature", 1.5);
        params.set("top_p", 0.9);
        params.set("max_tokens", 8192i64);

        let result = tool.execute(params, &ctx).await;
        assert!(result.success);
        assert!(result.content.contains("temperature=1.50"));
        assert!(result.content.contains("top_p=0.90"));
        assert!(result.content.contains("max_tokens=8192"));
    }

    #[tokio::test]
    async fn test_self_regulate_invalid_temperature() {
        let tool = SelfRegulateTool;
        let ctx = test_context_with_overrides();

        let mut params = ToolParameters::new();
        params.set("calls", 1i64);
        params.set("temperature", 3.0);

        let result = tool.execute(params, &ctx).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("temperature must be between"));
    }

    #[tokio::test]
    async fn test_self_regulate_invalid_calls() {
        let tool = SelfRegulateTool;
        let ctx = test_context_with_overrides();

        let mut params = ToolParameters::new();
        params.set("calls", 0i64);
        params.set("temperature", 0.5);

        let result = tool.execute(params, &ctx).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("calls must be between"));
    }

    #[tokio::test]
    async fn test_self_regulate_no_params() {
        let tool = SelfRegulateTool;
        let ctx = test_context_with_overrides();

        let mut params = ToolParameters::new();
        params.set("calls", 1i64);

        let result = tool.execute(params, &ctx).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("At least one parameter"));
    }

    #[tokio::test]
    async fn test_self_regulate_no_context() {
        let tool = SelfRegulateTool;
        let ctx = test_context_without_overrides();

        let mut params = ToolParameters::new();
        params.set("calls", 1i64);
        params.set("temperature", 0.5);

        let result = tool.execute(params, &ctx).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("not available"));
    }

    #[test]
    fn test_self_regulate_metadata() {
        let tool = SelfRegulateTool;
        assert_eq!(tool.name(), "self_regulate");
        assert!(tool.description().contains("temperature"));
        let params = tool.parameters();
        assert!(params.contains_key("calls"));
        assert!(params.contains_key("temperature"));
        assert!(params.contains_key("top_p"));
        assert!(params.contains_key("max_tokens"));
        assert!(params["calls"].required);
        assert!(!params["temperature"].required);
    }

    #[test]
    fn test_llm_request_overrides_apply() {
        let overrides = LlmRequestOverrides {
            temperature: Some(0.7),
            top_p: Some(0.95),
            max_tokens: Some(2048),
            remaining_calls: 3,
        };
        let mut request = serde_json::json!({
            "model": "test",
            "messages": [],
            "max_tokens": 4096
        });
        overrides.apply_to_request(&mut request);
        assert_eq!(request["temperature"], 0.7);
        assert_eq!(request["top_p"], 0.95);
        assert_eq!(request["max_tokens"], 2048);
    }

    #[test]
    fn test_llm_request_overrides_partial_apply() {
        let overrides = LlmRequestOverrides {
            temperature: Some(1.2),
            top_p: None,
            max_tokens: None,
            remaining_calls: 1,
        };
        let mut request = serde_json::json!({
            "model": "test",
            "messages": [],
            "max_tokens": 4096
        });
        overrides.apply_to_request(&mut request);
        assert_eq!(request["temperature"], 1.2);
        assert!(request.get("top_p").is_none() || request["top_p"].is_null());
        assert_eq!(request["max_tokens"], 4096);
    }
}
