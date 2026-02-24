/// ForecastTool — Tool interface for Reverso time series forecasting.
///
/// This tool enables zero-shot univariate time series forecasting using the
/// Reverso foundation model (arXiv:2602.17634), ported to Rust with the
/// candle framework.
///
/// Attribution:
///   Original Python implementation by @oaustegard:
///   https://github.com/oaustegard/claude-skills/tree/main/forecasting-reverso
use async_trait::async_trait;
use candle_core::Device;
use std::collections::HashMap;

use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;

use super::config::ReversoConfig;
use super::model;

pub struct ForecastTool;

#[async_trait]
impl Tool for ForecastTool {
    fn name(&self) -> &str {
        "forecast"
    }

    fn description(&self) -> &str {
        "Zero-shot univariate time series forecasting using the Reverso foundation model \
         (arXiv:2602.17634). Ported to Rust with candle framework. \
         Based on https://github.com/oaustegard/claude-skills by @oaustegard."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("series", "array", "Historical time series observations as a 1-D array of numbers. \
                The model's context window is 2048 steps. Shorter series are left-padded; \
                longer series use only the most recent 2048 observations.", required),
            param!("prediction_length", "number", "Number of future time steps to predict. \
                The model produces 48 steps per forward pass; longer horizons \
                use autoregressive rollout.", required),
            param!("model_path", "string", "Path to the model weights file (safetensors format). \
                Download from HuggingFace: shinfxh/reverso, then convert to safetensors.", required),
            param!("model_size", "string", "Model size variant: nano, small (default), or full.", optional, "small"),
            param!("flip_equivariant", "boolean", "Use flip-equivariant averaging. Default: false.", optional, false),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        // Parse arguments
        let series: Vec<f64> = match params.get_required::<Vec<f64>>("series") {
            Ok(s) => s,
            Err(e) => return ToolResult::error(format!("Invalid 'series': {}", e)),
        };
        let series_f32: Vec<f32> = series.iter().map(|&v| v as f32).collect();

        let prediction_length: i64 = match params.get_required::<i64>("prediction_length") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(format!("Invalid 'prediction_length': {}", e)),
        };

        let model_path: String = match params.get_required::<String>("model_path") {
            Ok(s) => s,
            Err(e) => return ToolResult::error(format!("Invalid 'model_path': {}", e)),
        };

        let model_size = match params.get_optional::<String>("model_size") {
            Ok(Some(s)) => s,
            Ok(None) => "small".to_string(),
            Err(e) => return ToolResult::error(format!("Invalid 'model_size': {}", e)),
        };

        let flip_equivariant = match params.get_optional::<bool>("flip_equivariant") {
            Ok(Some(v)) => v,
            Ok(None) => false,
            Err(e) => return ToolResult::error(format!("Invalid 'flip_equivariant': {}", e)),
        };

        // Validate inputs
        if series_f32.is_empty() {
            return ToolResult::error("Series must not be empty".to_string());
        }
        if prediction_length <= 0 {
            return ToolResult::error("prediction_length must be > 0".to_string());
        }
        let prediction_length = prediction_length as usize;

        let config = match ReversoConfig::from_name(&model_size) {
            Some(c) => c,
            None => return ToolResult::error(
                format!("Unknown model_size '{}'. Use nano, small, or full.", model_size)
            ),
        };

        // Load weights
        let device = Device::Cpu;
        let weights = match model::load_safetensors(&model_path, &device) {
            Ok(w) => w,
            Err(e) => return ToolResult::error(
                format!("Failed to load weights from '{}': {}", model_path, e)
            ),
        };

        // Run forecast
        match model::forecast(&series_f32, prediction_length, &weights, &config, flip_equivariant, device) {
            Ok(predictions) => {
                let result = serde_json::json!({
                    "predictions": predictions,
                    "prediction_length": predictions.len(),
                    "model_size": model_size,
                    "input_length": series_f32.len(),
                    "flip_equivariant": flip_equivariant,
                });
                ToolResult::success(serde_json::to_string_pretty(&result).unwrap_or_default())
            }
            Err(e) => ToolResult::error(format!("Forecast failed: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apchat_toolcore::ToolParameters;

    #[test]
    fn test_tool_name() {
        let tool = ForecastTool;
        assert_eq!(tool.name(), "forecast");
    }

    #[test]
    fn test_tool_parameters_has_required() {
        let tool = ForecastTool;
        let params = tool.parameters();
        assert!(params.get("series").unwrap().required);
        assert!(params.get("prediction_length").unwrap().required);
        assert!(params.get("model_path").unwrap().required);
        assert!(!params.get("model_size").unwrap().required);
        assert!(!params.get("flip_equivariant").unwrap().required);
    }

    #[tokio::test]
    async fn test_tool_rejects_empty_series() {
        let tool = ForecastTool;
        let ctx = ToolContext::new(
            std::path::PathBuf::from("/tmp"),
            "test_session".to_string(),
            apchat_policy::PolicyManager::new(),
        );
        let mut params = ToolParameters::new();
        params.set("series", Vec::<f64>::new());
        params.set("prediction_length", 10);
        params.set("model_path", "/tmp/nonexistent.safetensors");
        let result = tool.execute(params, &ctx).await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_tool_rejects_zero_prediction_length() {
        let tool = ForecastTool;
        let ctx = ToolContext::new(
            std::path::PathBuf::from("/tmp"),
            "test_session".to_string(),
            apchat_policy::PolicyManager::new(),
        );
        let mut params = ToolParameters::new();
        params.set("series", vec![1.0f64, 2.0, 3.0]);
        params.set("prediction_length", 0i64);
        params.set("model_path", "/tmp/nonexistent.safetensors");
        let result = tool.execute(params, &ctx).await;
        assert!(!result.success);
    }
}

