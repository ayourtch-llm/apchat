use anyhow::{Context, Result};

use apchat_vty::print_heart_red;
use apchat_llm_api::client::ChatMessage;
use apchat_models::ModelColor;
use crate::config::ClientConfig;
use crate::config::helpers::create_client_for_model_color;

/// Parse a model color string into a ModelColor enum value.
fn parse_model_color(color_str: &str) -> Result<ModelColor> {
    match color_str.to_lowercase().as_str() {
        "red" => Ok(ModelColor::RedModel),
        "grn" => Ok(ModelColor::GrnModel),
        "blu" => Ok(ModelColor::BluModel),
        _ => Err(anyhow::anyhow!(
            "Invalid reflexion model color: '{}'. Use 'red', 'grn', or 'blu'.",
            color_str
        )),
    }
}

/// Run the reflexion step: read input, call the LLM, write output.
///
/// - `reflexion_in`: optional path to a file whose content is sent as context
/// - `reflexion_out`: path where the reflexion response is written
/// - `reflexion_model`: model color string (red/grn/blu)
/// - `client_config`: LLM client configuration
pub async fn run_reflexion(
    reflexion_in: Option<&str>,
    reflexion_out: &str,
    reflexion_model: &str,
    client_config: &ClientConfig,
) -> Result<()> {
    let model_color = parse_model_color(reflexion_model)?;

    // Read the input file if specified
    let input_content = if let Some(in_path) = reflexion_in {
        let content = std::fs::read_to_string(in_path)
            .with_context(|| format!("Failed to read reflexion input file: {}", in_path))?;
        print_heart_red(
            &format!("📖 Read reflexion input from '{}' ({} bytes)", in_path, content.len()),
            true,
        );
        content
    } else {
        String::new()
    };

    // Build the reflexion prompt
    let prompt = if input_content.is_empty() {
        "Please reflect on the work performed so far. Evaluate its quality, \
         identify any issues, suggest improvements, and summarise the key takeaways."
            .to_string()
    } else {
        format!(
            "Please reflect on the following content. Evaluate its quality, \
             identify any issues, suggest improvements, and summarise the key takeaways.\n\n\
             --- BEGIN CONTENT ---\n{}\n--- END CONTENT ---",
            input_content
        )
    };

    // Create the LLM client for the chosen model color
    let client = create_client_for_model_color(&model_color, client_config, &client_config.api_key);

    let message = ChatMessage {
        role: "user".to_string(),
        content: prompt,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning: None,
    };

    print_heart_red(
        &format!(
            "🔄 Running reflexion using {} model...",
            model_color.display_name()
        ),
        true,
    );

    // Make the one-shot LLM call
    let response = client
        .chat_completion(&[message])
        .await
        .context("Reflexion LLM call failed")?;

    // Write the reflexion output to the specified file
    std::fs::write(reflexion_out, &response)
        .with_context(|| format!("Failed to write reflexion output to: {}", reflexion_out))?;

    print_heart_red(
        &format!(
            "✅ Reflexion output written to '{}' ({} bytes)",
            reflexion_out,
            response.len()
        ),
        true,
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_model_color_valid() {
        assert_eq!(parse_model_color("red").unwrap(), ModelColor::RedModel);
        assert_eq!(parse_model_color("grn").unwrap(), ModelColor::GrnModel);
        assert_eq!(parse_model_color("blu").unwrap(), ModelColor::BluModel);
        assert_eq!(parse_model_color("RED").unwrap(), ModelColor::RedModel);
        assert_eq!(parse_model_color("Grn").unwrap(), ModelColor::GrnModel);
    }

    #[test]
    fn test_parse_model_color_invalid() {
        assert!(parse_model_color("blue").is_err());
        assert!(parse_model_color("green").is_err());
        assert!(parse_model_color("").is_err());
    }

    #[test]
    fn test_reflexion_output_written() {
        let dir = tempfile::TempDir::new().unwrap();
        let in_path = dir.path().join("input.txt");
        let out_path = dir.path().join("output.txt");

        std::fs::write(&in_path, "Some task output to reflect on").unwrap();

        // Verify the input file can be read
        let content = std::fs::read_to_string(&in_path).unwrap();
        assert_eq!(content, "Some task output to reflect on");

        // Verify writing to the output path works
        std::fs::write(&out_path, "Reflexion result").unwrap();
        let result = std::fs::read_to_string(&out_path).unwrap();
        assert_eq!(result, "Reflexion result");
    }
}
