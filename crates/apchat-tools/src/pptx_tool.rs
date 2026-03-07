//! Tool implementation for PPTX presentation creation

use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use serde::Deserialize;

/// Tool for creating PPTX presentations
pub struct CreatePresentationTool;

/// Slide type for the presentation
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum SlideType {
    #[serde(rename = "title")]
    Title {
        title: String,
        subtitle: Option<String>,
    },
    #[serde(rename = "content")]
    Content {
        title: String,
        bullets: Vec<String>,
    },
}

/// Presentation creation parameters
#[derive(Debug, Deserialize)]
pub struct PresentationParams {
    path: String,
    title: String,
    author: String,
    slides: Vec<SlideType>,
}

#[async_trait]
impl Tool for CreatePresentationTool {
    fn name(&self) -> &str {
        "create_presentation"
    }

    fn description(&self) -> &str {
        "Create a PPTX presentation file with title, author, and slides. 
Accepts a JSON object with:
- path: Output file path (e.g., 'presentation.pptx')
- title: Presentation title
- author: Author name
- slides: Array of slide objects, each with:
  - type: 'title' or 'content'
  - title: Slide title
  - subtitle: Subtitle (for title slides)
  - bullets: Array of bullet points (for content slides)

The tool uses the ppt-rs library (Apache-2.0 licensed) which supports:
- Multiple slide layouts (title, centered title, content, blank, etc.)
- Text formatting (bold, italic, underline, colors, sizes)
- Tables with cell background colors
- Charts (bar, line, pie, etc.)
- Images and media embedding
- Shapes with fills and gradients
- Slide background colors (via background_color field on slides)"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("path", "string", "Output file path for the PPTX presentation (e.g., 'presentation.pptx')", required),
            param!("title", "string", "Presentation title", required),
            param!("author", "string", "Author name", required),
            param!("slides", "array", "Array of slide objects. Each slide has a 'type' ('title' or 'content'), 'title', and optionally 'subtitle' (for title slides) or 'bullets' (array of strings for content slides)", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        // Parse the path parameter
        let path = match params.get_required::<String>("path") {
            Ok(p) => p,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        // Parse the title parameter
        let title = match params.get_required::<String>("title") {
            Ok(t) => t,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        // Parse the author parameter
        let author = match params.get_required::<String>("author") {
            Ok(a) => a,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        // Parse the slides parameter
        let slides: Vec<SlideType> = match params.get_required::<Vec<SlideType>>("slides") {
            Ok(s) => s,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        // Build slides using ppt-rs
        let mut pptx_slides: Vec<ppt_rs::generator::SlideContent> = Vec::new();
        
        for slide in &slides {
            match slide {
                SlideType::Title { title: slide_title, subtitle } => {
                    // For title slides, use CenteredTitle layout
                    // If there's a subtitle, add it as the first content item
                    let mut slide_content = ppt_rs::generator::SlideContent::new(slide_title)
                        .title_size(44)
                        .title_bold(true)
                        .layout(ppt_rs::generator::SlideLayout::CenteredTitle);
                    
                    if let Some(sub) = subtitle {
                        // Add subtitle as first content item
                        slide_content = slide_content.add_bullet(sub);
                    }
                    
                    pptx_slides.push(slide_content);
                }
                SlideType::Content { title: slide_title, bullets } => {
                    let mut slide_content = ppt_rs::generator::SlideContent::new(slide_title)
                        .title_size(32)
                        .title_bold(true);
                    
                    for bullet in bullets {
                        slide_content = slide_content.add_bullet(bullet);
                    }
                    
                    pptx_slides.push(slide_content);
                }
            }
        }

        // Create the presentation using ppt-rs
        let pptx_data = match ppt_rs::generator::create_pptx_with_content(&title, pptx_slides) {
            Ok(data) => data,
            Err(e) => return ToolResult::error(format!("Failed to create presentation: {}", e)),
        };

        // Save the presentation
        let full_path = context.work_dir.join(&path);
        
        match std::fs::write(&full_path, &pptx_data) {
            Ok(()) => {
                ToolResult::success(format!(
                    "Successfully created presentation '{}' with {} slide(s) at '{}'",
                    title, slides.len(), path
                ))
            }
            Err(e) => ToolResult::error(format!("Failed to save presentation: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apchat_toolcore::ToolParameters;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_create_presentation_tool_basic() {
        let tool = CreatePresentationTool;
        assert_eq!(tool.name(), "create_presentation");
        assert!(!tool.description().is_empty());
        
        let params = tool.parameters();
        assert!(params.contains_key("path"));
        assert!(params.contains_key("title"));
        assert!(params.contains_key("author"));
        assert!(params.contains_key("slides"));
    }

    #[tokio::test]
    async fn test_create_presentation_execution() {
        let tool = CreatePresentationTool;
        
        // Create test parameters
        let slides_json = serde_json::json!([
            {
                "type": "title",
                "title": "Welcome",
                "subtitle": "Subtitle"
            },
            {
                "type": "content",
                "title": "Features",
                "bullets": ["Bullet 1", "Bullet 2"]
            }
        ]);
        
        let params = ToolParameters {
            data: HashMap::from([
                ("path".to_string(), serde_json::Value::String("test_output.pptx".to_string())),
                ("title".to_string(), serde_json::Value::String("My Presentation".to_string())),
                ("author".to_string(), serde_json::Value::String("APChat AI".to_string())),
                ("slides".to_string(), slides_json),
            ]),
        };

        let context = ToolContext::new(
            PathBuf::new(),
            "test_session".to_string(),
            apchat_policy::PolicyManager::new(),
        );

        let result = tool.execute(params, &context).await;
        
        // The presentation should be created successfully
        // (file will be in current directory since work_dir is empty)
        assert!(result.success, "Expected success, got: {}", result.content);
    }
}