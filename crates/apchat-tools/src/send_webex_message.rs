use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use apchat_webex::WebexClient;

/// Tool for sending Webex messages to authorized recipients
/// Only available when Webex is configured
pub struct SendWebexMessageTool {
    webex_client: Arc<WebexClient>,
    authorized_email: String,
}

impl SendWebexMessageTool {
    pub fn new(webex_client: Arc<WebexClient>, authorized_email: String) -> Self {
        Self {
            webex_client,
            authorized_email,
        }
    }
}

#[async_trait]
impl Tool for SendWebexMessageTool {
    fn name(&self) -> &str {
        "send_webex_message"
    }

    fn description(&self) -> &str {
        "Send a Webex message to an authorized recipient. Currently only allows sending to the configured authorized user email."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("recipient_email", "string", "Email address of the recipient (must be authorized)", required),
            param!("message_text", "string", "The message content to send", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let recipient_email = match params.get_required::<String>("recipient_email") {
            Ok(email) => email,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let message_text = match params.get_required::<String>("message_text") {
            Ok(text) => text,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        // Authorization check: only allow sending to authorized user
        if recipient_email != self.authorized_email {
            return ToolResult::error(format!(
                "Unauthorized recipient: {}. Only authorized to send to: {}",
                recipient_email, self.authorized_email
            ));
        }

        // Send message directly to email (automatically creates/uses direct room)
        match self.webex_client.send_message_to_email(&recipient_email, &message_text).await {
            Ok(msg) => ToolResult::success(format!(
                "Message sent to {} (room: {}, message_id: {})",
                recipient_email, msg.room_id, msg.id
            )),
            Err(e) => ToolResult::error(format!("Failed to send message: {}", e)),
        }
    }
}