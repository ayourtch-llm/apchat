use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition, ContextEdit};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;

pub struct DeleteItemsTool;

#[async_trait]
impl Tool for DeleteItemsTool {
    fn name(&self) -> &str {
        "delete_items"
    }

    fn description(&self) -> &str {
        "Delete items from the conversation context window by their indices. Messages are automatically grouped: deleting an assistant message with tool calls will also delete its tool results, and vice versa. System messages cannot be deleted. Role ordering is preserved."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("indices", "array", "Array of 0-based message indices to delete from the context window. Related messages (assistant messages with tool_calls and their tool results) are automatically included to preserve role ordering.", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let indices: Vec<usize> = match params.get_required::<Vec<usize>>("indices") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(format!("Failed to parse indices: {}", e)),
        };

        if indices.is_empty() {
            return ToolResult::error("indices array must not be empty".to_string());
        }

        let edits_arc = match &context.context_edits {
            Some(arc) => arc,
            None => return ToolResult::error("Context editing is not available in this context".to_string()),
        };

        match edits_arc.lock() {
            Ok(mut guard) => {
                guard.push(ContextEdit::DeleteItems { indices: indices.clone() });
            }
            Err(e) => return ToolResult::error(format!("Failed to queue context edit: {}", e)),
        }

        ToolResult::success(format!(
            "Queued deletion of items at indices: {:?}. The deletion will be applied after this tool call completes. Related messages (assistant+tool pairs) will be automatically included to preserve role ordering.",
            indices
        ))
    }
}

pub struct EditItemTool;

#[async_trait]
impl Tool for EditItemTool {
    fn name(&self) -> &str {
        "edit_item"
    }

    fn description(&self) -> &str {
        "Edit the content of a message in the context window that is marked as **EDITABLE**. This allows updating knowledge entries, notes, or other mutable content in the conversation history."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("index", "number", "0-based index of the message to edit in the context window. The message must contain the **EDITABLE** marker.", required),
            param!("new_content", "string", "The new content to replace the message with. Should include the **EDITABLE** marker if you want the item to remain editable.", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let index = match params.get_required::<usize>("index") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(format!("Failed to parse index: {}", e)),
        };

        let new_content = match params.get_required::<String>("new_content") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(format!("Failed to parse new_content: {}", e)),
        };

        let edits_arc = match &context.context_edits {
            Some(arc) => arc,
            None => return ToolResult::error("Context editing is not available in this context".to_string()),
        };

        match edits_arc.lock() {
            Ok(mut guard) => {
                guard.push(ContextEdit::EditItem { index, new_content: new_content.clone() });
            }
            Err(e) => return ToolResult::error(format!("Failed to queue context edit: {}", e)),
        }

        ToolResult::success(format!(
            "Queued edit of item at index {}. The edit will be applied after this tool call completes. Note: the target message must contain the **EDITABLE** marker.",
            index
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apchat_policy::PolicyManager;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    fn test_context_with_edits() -> ToolContext {
        let mut ctx = ToolContext::new(
            PathBuf::from("/tmp"),
            "test_session".to_string(),
            PolicyManager::new(),
        );
        ctx.context_edits = Some(Arc::new(Mutex::new(Vec::new())));
        ctx
    }

    fn test_context_without_edits() -> ToolContext {
        ToolContext::new(
            PathBuf::from("/tmp"),
            "test_session".to_string(),
            PolicyManager::new(),
        )
    }

    #[tokio::test]
    async fn test_delete_items_queues_edit() {
        let tool = DeleteItemsTool;
        let ctx = test_context_with_edits();

        let mut params = ToolParameters::new();
        params.set("indices", vec![2usize, 3usize]);

        let result = tool.execute(params, &ctx).await;
        assert!(result.success, "Expected success, got: {:?}", result.error);
        assert!(result.content.contains("Queued deletion"));

        let edits = ctx.context_edits.unwrap();
        let guard = edits.lock().unwrap();
        assert_eq!(guard.len(), 1);
        match &guard[0] {
            ContextEdit::DeleteItems { indices } => {
                assert_eq!(indices, &vec![2, 3]);
            }
            _ => panic!("Expected DeleteItems"),
        }
    }

    #[tokio::test]
    async fn test_delete_items_empty_indices() {
        let tool = DeleteItemsTool;
        let ctx = test_context_with_edits();

        let mut params = ToolParameters::new();
        params.set("indices", Vec::<usize>::new());

        let result = tool.execute(params, &ctx).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("must not be empty"));
    }

    #[tokio::test]
    async fn test_delete_items_no_context() {
        let tool = DeleteItemsTool;
        let ctx = test_context_without_edits();

        let mut params = ToolParameters::new();
        params.set("indices", vec![1usize]);

        let result = tool.execute(params, &ctx).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("not available"));
    }

    #[tokio::test]
    async fn test_edit_item_queues_edit() {
        let tool = EditItemTool;
        let ctx = test_context_with_edits();

        let mut params = ToolParameters::new();
        params.set("index", 5usize);
        params.set("new_content", "**EDITABLE** Updated knowledge");

        let result = tool.execute(params, &ctx).await;
        assert!(result.success, "Expected success, got: {:?}", result.error);
        assert!(result.content.contains("Queued edit"));

        let edits = ctx.context_edits.unwrap();
        let guard = edits.lock().unwrap();
        assert_eq!(guard.len(), 1);
        match &guard[0] {
            ContextEdit::EditItem { index, new_content } => {
                assert_eq!(*index, 5);
                assert_eq!(new_content, "**EDITABLE** Updated knowledge");
            }
            _ => panic!("Expected EditItem"),
        }
    }

    #[tokio::test]
    async fn test_edit_item_no_context() {
        let tool = EditItemTool;
        let ctx = test_context_without_edits();

        let mut params = ToolParameters::new();
        params.set("index", 1usize);
        params.set("new_content", "test");

        let result = tool.execute(params, &ctx).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("not available"));
    }

    #[test]
    fn test_delete_items_metadata() {
        let tool = DeleteItemsTool;
        assert_eq!(tool.name(), "delete_items");
        assert!(tool.description().contains("Delete"));
        let params = tool.parameters();
        assert!(params.contains_key("indices"));
        assert!(params["indices"].required);
    }

    #[test]
    fn test_edit_item_metadata() {
        let tool = EditItemTool;
        assert_eq!(tool.name(), "edit_item");
        assert!(tool.description().contains("EDITABLE"));
        let params = tool.parameters();
        assert!(params.contains_key("index"));
        assert!(params.contains_key("new_content"));
        assert!(params["index"].required);
        assert!(params["new_content"].required);
    }
}
