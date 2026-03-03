use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use apchat_webex::WebexClient;

/// Tool for deleting Webex messages
/// Only available when Webex is configured
pub struct DeleteWebexMessageTool {
    webex_client: Arc<WebexClient>,
    authorized_email: String,
}

impl DeleteWebexMessageTool {
    pub fn new(webex_client: Arc<WebexClient>, authorized_email: String) -> Self {
        Self {
            webex_client,
            authorized_email,
        }
    }
}

#[async_trait]
impl Tool for DeleteWebexMessageTool {
    fn name(&self) -> &str {
        "delete_webex_message"
    }

    fn description(&self) -> &str {
        "Delete a Webex message by ID or by text content. Use message_id if you have it, otherwise provide text to search for."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("message_id", "string", "Message ID to delete (if known)", optional),
            param!("text", "string", "Text content to search for and delete (if message_id not known)", optional),
            param!("room_id", "string", "Room ID where the message is located", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        // Get room_id (required)
        let room_id = match params.get_optional::<String>("room_id") {
            Ok(Some(room)) => room,
            Ok(None) => {
                return ToolResult::error("room_id is required. Provide the room ID where the message is located.".to_string());
            }
            Err(e) => {
                return ToolResult::error(format!("Failed to parse room_id: {}", e));
            }
        };

        // Get message_id or text (at least one required)
        let message_id: Option<String> = match params.get_optional::<String>("message_id") {
            Ok(Some(val)) => Some(val),
            Ok(None) | Err(_) => None,
        };
        let text: Option<String> = match params.get_optional::<String>("text") {
            Ok(Some(val)) => Some(val),
            Ok(None) | Err(_) => None,
        };

        if message_id.is_none() && text.is_none() {
            return ToolResult::error("Either message_id or text must be provided to delete a message.".to_string());
        }

        let result = if let Some(message_id) = message_id {
            // Delete by ID
            match self.webex_client.delete_message_by_id(&room_id, &message_id).await {
                Ok(_) => ToolResult::success(format!("Message {} deleted successfully", message_id)),
                Err(e) => ToolResult::error(format!("Failed to delete message {}: {}", message_id, e)),
            }
        } else if let Some(text) = text {
            // Delete by text content
            match self.webex_client.delete_message_by_text(&room_id, &text).await {
                Ok(count) => ToolResult::success(format!("Deleted {} message(s) matching text: {}", count, text.chars().take(50).collect::<String>())),
                Err(e) => ToolResult::error(format!("Failed to delete message: {}", e)),
            }
        } else {
            unreachable!()
        };

        result
    }
}