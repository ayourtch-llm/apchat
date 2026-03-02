use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{Html, IntoResponse, Json, Response},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use apchat_models::Message as ChatMessage;
use apchat_vty::{print_heart_red, print_heart_yellow, tool_counter::ToolGuard};
use apchat_toolcore::ToolParameters;
use serde_json::Value;
use crate::{
    api::call_api,
    mspc::{MspcChannel, MspcMessage},
    web::{
        protocol::{ClientMessage, ServerMessage, SessionConfig, SessionId, SessionInfo},
        session_manager::SessionManager,
    },
};

use chrono::Utc;

/// Application state shared across routes
#[derive(Clone)]
pub struct AppState {
    pub session_manager: Arc<SessionManager>,
    pub mspc_channel: Option<Arc<MspcChannel>>,
}

/// Create router with all routes
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // API routes
        .route("/api/sessions", get(list_sessions).post(create_session))
        .route(
            "/api/sessions/:id",
            get(get_session_details).delete(close_session),
        )
        // WebSocket endpoint
        .route("/ws/:session_id", get(websocket_handler))
        // Static files (HTML pages)
        .route("/", get(serve_index))
        .route("/session/:id", get(serve_session))
        .with_state(state)
}

/// GET /api/sessions - List all active sessions
async fn list_sessions(State(state): State<AppState>) -> Json<Vec<SessionInfo>> {
    let sessions = state.session_manager.list_sessions().await;
    Json(sessions)
}

/// POST /api/sessions - Create a new session
async fn create_session(
    State(state): State<AppState>,
    Json(config): Json<SessionConfig>,
) -> Result<Json<serde_json::Value>, AppError> {
    let session_id = state.session_manager.create_session(config).await?;

    Ok(Json(serde_json::json!({
        "id": session_id,
        "created_at": chrono::Utc::now().to_rfc3339(),
        "websocket_url": format!("/ws/{}", session_id),
    })))
}

/// GET /api/sessions/:id - Get session details
async fn get_session_details(
    State(state): State<AppState>,
    Path(id): Path<SessionId>,
) -> Result<Json<SessionInfo>, AppError> {
    let session = state
        .session_manager
        .get_session(&id)
        .await
        .ok_or_else(|| AppError::NotFound("Session not found".into()))?;

    Ok(Json(session.get_info().await))
}

/// DELETE /api/sessions/:id - Close a session
async fn close_session(
    State(state): State<AppState>,
    Path(id): Path<SessionId>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.session_manager.remove_session(&id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Session closed successfully",
    })))
}

/// GET /ws/:session_id - WebSocket endpoint
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(session_id): Path<SessionId>,
) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state, session_id))
}

/// Handle WebSocket connection
async fn handle_websocket(socket: WebSocket, state: AppState, session_id: SessionId) {
    let client_id = Uuid::new_v4();

    // Get or verify session exists
    let session = match state.session_manager.get_session(&session_id).await {
        Some(s) => s,
        None => {
            print_heart_yellow(&format!("WebSocket: Session {} not found", session_id), true);
            return;
        }
    };

    // Create channel for sending messages to this client
    let (ws_sender, mut ws_receiver) = mpsc::unbounded_channel();

    // Add client to session
    session.add_client(client_id, ws_sender).await;

    // Send SessionJoined message
    let apchat = session.apchat.lock().await;
    let history = apchat.messages.clone();
    let current_model = apchat.current_model.display_name();
    drop(apchat);

    let join_msg = ServerMessage::SessionJoined {
        session_id,
        session_type: session.session_type.as_str().to_string(),
        created_at: session.created_at.to_rfc3339(),
        current_model: current_model.to_string(),
        history,
    };

    let _ = session.send_to_client(client_id, join_msg).await;

    // Split socket
    let (mut ws_sink, mut ws_stream) = socket.split();

    // Spawn task to send messages from channel to WebSocket
    let session_clone = session.clone();
    let send_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if ws_sink.send(WsMessage::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming WebSocket messages
    while let Some(Ok(msg)) = ws_stream.next().await {
        if let WsMessage::Text(text) = msg {
            print_heart_yellow(&format!("📨 Received WebSocket message: {}", text), true);
            match serde_json::from_str::<ClientMessage>(&text) {
                Ok(client_msg) => {
                    print_heart_yellow(&format!("✅ Parsed message: {:?}", client_msg), true);
                    handle_client_message(client_id, client_msg, &session_clone, &state).await;
                }
                Err(e) => {
                    print_heart_yellow(&format!("❌ Failed to parse message: {} - Error: {}", text, e), true);
                }
            }
        }
    }

    // Client disconnected
    session_clone.remove_client(client_id).await;
    send_task.abort();
}

/// Handle a message from a client
async fn handle_client_message(
    client_id: Uuid,
    message: ClientMessage,
    session: &Arc<crate::web::session_manager::Session>,
    state: &AppState,
) {
    use ClientMessage::*;

    match message {
        SendMessage { content } => {
            // Spawn chat handling in separate task to avoid blocking WebSocket reader
            // This is critical: if we await here, the WebSocket reader can't receive
            // confirmation messages because it's blocked waiting for this to complete
            let session_clone = Arc::clone(session);
            let state_clone = state.clone();
            tokio::spawn(async move {
                handle_send_message(client_id, content, &session_clone, &state_clone).await;
            });
        }
        ConfirmTool {
            tool_call_id,
            confirmed,
        } => {
            print_heart_yellow(&format!("🔔 Received ConfirmTool: id={}, confirmed={}", tool_call_id, confirmed), true);
            // Respond to pending confirmation
            let found = session.respond_to_confirmation(&tool_call_id, confirmed).await;
            print_heart_yellow(&format!("🔔 Confirmation response sent: found={}", found), true);
        }
        ListSessions => {
            let sessions = state.session_manager.list_sessions().await;
            let msg = ServerMessage::SessionList { sessions };
            session.send_to_client(client_id, msg).await;
        }
        SwitchModel { model, reason } => {
            handle_switch_model(model, reason, session).await;
        }
        UpdateSessionTitle { title } => {
            handle_update_session_title(title, session, state).await;
        }
        CreateSession { config } => {
            handle_create_session(client_id, config, session, state).await;
        }
        JoinSession { session_id } => {
            handle_join_session(session_id, client_id, session, state).await;
        }
        LeaveSession => {
            handle_leave_session(client_id, session).await;
        }
        CancelExecution => {
            handle_cancel_execution(session).await;
        }
        SaveState { file_path } => {
            handle_save_state(file_path, session, state).await;
        }
        LoadState { file_path } => {
            handle_load_state(file_path, session, state).await;
        }
        InvokeSkill { skill_name } => {
            handle_invoke_skill(skill_name, session, state).await;
        }
    }
}

/// Check if tool requires confirmation and extract plan/diff
async fn check_tool_confirmation(
    tool_name: &str,
    tool_args: &str,
    work_dir: &std::path::Path,
) -> (bool, Option<String>) {
    match tool_name {
        "apply_edit_plan" => {
            // Try to load and format the edit plan
            let plan_path = work_dir.join(".apchat_edit_plan.json");
            if let Ok(content) = tokio::fs::read_to_string(&plan_path).await {
                if let Ok(plan) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                    let mut diff_text = String::new();
                    for (idx, edit) in plan.iter().enumerate() {
                        diff_text.push_str(&format!(
                            "Edit #{} {} - {}\n",
                            idx + 1,
                            edit.get("file_path").and_then(|v| v.as_str()).unwrap_or("?"),
                            edit.get("description").and_then(|v| v.as_str()).unwrap_or("?")
                        ));
                        if let Some(old) = edit.get("old_content").and_then(|v| v.as_str()) {
                            for line in old.lines() {
                                diff_text.push_str(&format!("  -{}\n", line));
                            }
                        }
                        if let Some(new) = edit.get("new_content").and_then(|v| v.as_str()) {
                            for line in new.lines() {
                                diff_text.push_str(&format!("  +{}\n", line));
                            }
                        }
                        diff_text.push('\n');
                    }
                    return (true, Some(diff_text));
                }
            }
            (true, None)
        }
        "write_file" | "edit_file" => (true, None), // These also need confirmation but no pre-extracted diff
        _ => (false, None),
    }
}

/// Chat loop with WebSocket broadcasts (single LLM mode)
async fn handle_chat_with_broadcast(
    session: &Arc<crate::web::session_manager::Session>,
) -> anyhow::Result<()> {
    const MAX_TOOL_ITERATIONS: usize = 250;
    let mut tool_call_iterations = 0;
    let session_id = session.id;

    loop {
        let apchat = session.apchat.lock().await;

        // Make API call
        let (response, usage, _model, _finish_reason) = call_api(
            &apchat,
            &apchat.messages,
        )
        .await?;

        drop(apchat); // Release lock

        // Broadcast token usage
        if let Some(usage) = &usage {
            let mut apchat = session.apchat.lock().await;
            apchat.total_tokens_used += usage.total_tokens;
            let session_total = apchat.total_tokens_used;
            drop(apchat);

            let token_msg = ServerMessage::TokenUsage {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
                session_total,
            };
            session.broadcast(token_msg).await;
        }

        // Add assistant response to history
        session.apchat.lock().await.messages.push(response.clone());

        // Handle tool calls
        if let Some(tool_calls) = &response.tool_calls {
            tool_call_iterations += 1;

            for tool_call in tool_calls {
                // Check if tool requires confirmation
                let work_dir = session.apchat.lock().await.work_dir.clone();
                let (requires_confirmation, diff) = check_tool_confirmation(
                    &tool_call.function.name,
                    &tool_call.function.arguments,
                    &work_dir,
                )
                .await;

                // Broadcast tool call request
                let tool_msg = ServerMessage::ToolCallRequest {
                    tool_call_id: tool_call.id.clone(),
                    name: tool_call.function.name.clone(),
                    arguments: serde_json::from_str(&tool_call.function.arguments)
                        .unwrap_or(serde_json::json!({})),
                    requires_confirmation,
                    diff,
                    iteration: Some(tool_call_iterations),
                    max_iterations: Some(MAX_TOOL_ITERATIONS),
                };
                session.broadcast(tool_msg).await;

                // If requires confirmation, wait for user response
                if requires_confirmation {
                    print_heart_yellow(&format!("⏳ Registering confirmation for tool_call_id: {}", tool_call.id), true);
                    let confirmation_rx = session
                        .register_confirmation(
                            tool_call.id.clone(),
                            tool_call.function.name.clone(),
                            tool_call.function.arguments.clone(),
                        )
                        .await;

                    print_heart_yellow("⏳ Waiting for user confirmation...", true);
                    // Wait for confirmation (with timeout)
                    let confirmed = match tokio::time::timeout(
                        std::time::Duration::from_secs(300), // 5 minute timeout
                        confirmation_rx,
                    )
                    .await
                    {
                        Ok(Ok(confirmed)) => {
                            print_heart_yellow(&format!("✅ Received confirmation: {}", confirmed), true);
                            confirmed
                        }
                        Ok(Err(_)) => {
                            print_heart_yellow("❌ Confirmation channel closed", true);
                            false
                        }
                        Err(_) => {
                            // Timeout
                            print_heart_yellow("⏱️  Confirmation timeout", true);
                            let error_msg = ServerMessage::Error {
                                message: "Tool confirmation timeout (5 minutes)".to_string(),
                                recoverable: true,
                            };
                            session.broadcast(error_msg).await;
                            false
                        }
                    };

                    if !confirmed {
                        print_heart_yellow("🚫 Tool execution denied", true);

                        // User denied, send error result
                        let error_str = "Tool execution cancelled by user".to_string();
                        let result_msg = ServerMessage::ToolCallResult {
                            tool_call_id: tool_call.id.clone(),
                            result: error_str.clone(),
                            success: false,
                            formatted_result: Some(error_str.clone()),
                        };
                        session.broadcast(result_msg).await;

                        // Add cancellation to history
                        session.apchat.lock().await.messages.push(ChatMessage {
                            role: "tool".to_string(),
                            content: error_str,
                            tool_calls: None,
                            tool_call_id: Some(tool_call.id.clone()),
                            name: Some(tool_call.function.name.clone()),
                            reasoning: None,
                        });
                        continue; // Skip to next tool call
                    }
                }

                // Validate tool call parameters before execution (single LLM mode only)
                let mut apchat = session.apchat.lock().await;
                let tool = apchat.tool_registry.get_tool(&tool_call.function.name);

                if let Some(tool) = tool {
                    // Get parameter definitions from the tool
                    let param_definitions: HashMap<String, Value> = tool.parameters()
                        .iter()
                        .map(|(key, def)| {
                            (key.clone(), serde_json::to_value(def).unwrap_or(Value::Null))
                        })
                        .collect();

                    // Validate the tool call with SQL logging
                    let validated_params = match apchat_toolcore::parameter_validation::validate_tool_call_with_logging(
                        &tool_call, 
                        &ToolParameters { data: HashMap::new() }, 
                        &param_definitions,
                        None,
                    ).await {
                        Ok(params) => params,
                        Err(e) => {
                            print_heart_red(&format!("⚠️  {}", e), true);

                            let error_msg_msg = ServerMessage::Error {
                                message: e.clone(),
                                recoverable: true,
                            };
                            session.broadcast(error_msg_msg).await;

                            // Add error to history
                            session.apchat.lock().await.messages.push(ChatMessage {
                                role: "tool".to_string(),
                                content: e.clone(),
                                tool_calls: None,
                                tool_call_id: Some(tool_call.id.clone()),
                                name: Some(tool_call.function.name.clone()),
                                reasoning: None,
                            });

                            continue;
                        }
                    };

                    // Create tool context
                    let current_model_string = apchat.current_model.as_str_default();
                    let context = apchat_toolcore::ToolContext::new(
                        apchat.work_dir.clone(),
                        format!("session_{}", uuid::Uuid::new_v4()),
                        apchat.policy_manager.clone()
                    )
                    .with_terminal_manager(apchat.terminal_manager.clone())
                    .with_todo_manager(apchat.todo_manager.clone())
                    .with_non_interactive(apchat.non_interactive)
                    .with_current_model_string(current_model_string);

                    // Add LLM clients to context
                    let mut llm_clients: HashMap<apchat_models::ModelColor, Arc<dyn apchat_llm_api::client::LlmClient>> = HashMap::new();
                    for color in apchat_models::ModelColor::iter() {
                        let client = crate::config::create_client_for_model_color(
                            &color,
                            &apchat.client_config,
                            &apchat.api_key,
                        );
                        llm_clients.insert(color, client);
                    }
                    let context_with_clients = context.with_llm_clients(llm_clients);

                    // Add skill registry if available
                    let context_with_skill = if let Some(ref registry) = apchat.skill_registry {
                        context_with_clients.with_skill_registry(Arc::clone(registry))
                    } else {
                        context_with_clients
                    };

                    // Add content limiter if available
                    let context_with_limiter = if let Some(ref limiter) = apchat.content_limiter {
                        context_with_skill.with_content_limiter(Arc::clone(limiter))
                    } else {
                        context_with_skill
                    };

                    // Add MSPC sender and receiver if available
                    let final_context = if let Some(ref mspc_channel) = apchat.mspc_channel {
                        context_with_limiter.with_mspc_sender(mspc_channel.sender())
                            .with_mspc_receiver(mspc_channel.receiver())
                    } else {
                        context_with_limiter
                    };

                    // Add signal sender if available
                    let context_with_signal = if let Some(ref signal_sender) = apchat.signal_sender {
                        final_context.with_signal_sender(signal_sender.clone())
                    } else {
                        final_context
                    };

                    // Add signal receiver if available
                    let context_with_receiver = if let Some(ref signal_receiver) = apchat.signal_receiver {
                        context_with_signal.with_signal_receiver(signal_receiver.clone())
                    } else {
                        context_with_signal
                    };

                    // Add confirmation registry if available
                    let context_with_confirmation = if let Some(ref confirmation_registry) = apchat.confirmation_registry {
                        context_with_receiver.with_confirmation_registry(confirmation_registry.clone())
                    } else {
                        context_with_receiver
                    };

                    // Add validated parameters to context and execute
                    let _tool_guard = ToolGuard::new();
                    let tool_result = apchat.tool_registry.execute_tool(
                        &tool_call.function.name,
                        validated_params,
                        &context_with_confirmation
                    ).await;
                    drop(apchat);

                    // Broadcast tool result
                    match tool_result {
                        apchat_toolcore::ToolResult { success, content, error, .. } => {
                            let result_str = if success {
                                content
                            } else {
                                error.unwrap_or_else(|| "Unknown error".to_string())
                            };

                            let result_msg = ServerMessage::ToolCallResult {
                                tool_call_id: tool_call.id.clone(),
                                result: result_str.clone(),
                                success,
                                formatted_result: Some(result_str.clone()),
                            };
                            session.broadcast(result_msg).await;

                            // Add tool result to history
                            session.apchat.lock().await.messages.push(ChatMessage {
                                role: "tool".to_string(),
                                content: result_str,
                                tool_calls: None,
                                tool_call_id: Some(tool_call.id.clone()),
                                name: Some(tool_call.function.name.clone()),
                                reasoning: None,
                            });
                        }
                    }
                } else {
                    // Tool doesn't exist in registry - execute without validation
                    let result = apchat
                        .execute_tool(&tool_call.function.name, &tool_call.function.arguments)
                        .await;
                    drop(apchat);

                    // Broadcast tool result
                    match result {
                        Ok(result_str) => {
                            let result_msg = ServerMessage::ToolCallResult {
                                tool_call_id: tool_call.id.clone(),
                                result: result_str.clone(),
                                success: true,
                                formatted_result: Some(result_str.clone()),
                            };
                            session.broadcast(result_msg).await;

                            // Add tool result to history
                            session.apchat.lock().await.messages.push(ChatMessage {
                                role: "tool".to_string(),
                                content: result_str,
                                tool_calls: None,
                                tool_call_id: Some(tool_call.id.clone()),
                                name: Some(tool_call.function.name.clone()),
                                reasoning: None,
                            });
                        }
                        Err(e) => {
                            let error_str = format!("Error: {}", e);
                            let result_msg = ServerMessage::ToolCallResult {
                                tool_call_id: tool_call.id.clone(),
                                result: error_str.clone(),
                                success: false,
                                formatted_result: Some(error_str.clone()),
                            };
                            session.broadcast(result_msg).await;

                            // Add error to history
                            session.apchat.lock().await.messages.push(ChatMessage {
                                role: "tool".to_string(),
                                content: error_str,
                                tool_calls: None,
                                tool_call_id: Some(tool_call.id.clone()),
                                name: Some(tool_call.function.name.clone()),
                                reasoning: None,
                            });
                        }
                    }
                }
            }

            // Check iteration limit
            if tool_call_iterations >= MAX_TOOL_ITERATIONS {
                let error_msg = ServerMessage::Error {
                    message: format!("Maximum tool iterations ({}) reached", MAX_TOOL_ITERATIONS),
                    recoverable: false,
                };
                session.broadcast(error_msg).await;
                break;
            }

            // Continue loop for next API call
            continue;
        }

        // No tool calls - send final response and complete
        let msg = ServerMessage::AssistantMessage {
            content: response.content,
            streaming: false,
        };
        session.broadcast(msg).await;
        session.broadcast(ServerMessage::AssistantMessageComplete).await;
        break;
    }

    Ok(())
}

/// Generate a title for the session based on the first user message
async fn generate_session_title(
    first_message: &str,
    session: &Arc<crate::web::session_manager::Session>,
) -> Option<String> {
    // Create a simple prompt to generate a title
    let title_prompt = vec![
        apchat_models::Message {
            role: "user".to_string(),
            content: format!(
                "Generate a concise, descriptive title (3-6 words) for a chat session that starts with this message. \
                Only respond with the title, nothing else.\n\nMessage: {}",
                first_message
            ),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning: None,
        }
    ];

    // Make an isolated API call
    let apchat = session.apchat.lock().await;
    match call_api(&apchat, &title_prompt).await {
        Ok((response, _, _, _)) => {
            let title = response.content.trim().to_string();
            // Remove quotes if present
            let title = title.trim_matches('"').trim_matches('\'').to_string();
            Some(title)
        }
        Err(e) => {
            print_heart_yellow(&format!("⚠️  Failed to generate session title: {}", e), true);
            None
        }
    }
}

/// Handle SendMessage
async fn handle_send_message(
    _client_id: Uuid,
    content: String,
    session: &Arc<crate::web::session_manager::Session>,
    state: &AppState,
) {
    // Check if MSPC channel is available
    if let Some(mspc_channel) = &state.mspc_channel {
        // Send message to MSPC channel
        let mspc_msg = MspcMessage::UserInput(content.clone(), Some("web_socket".to_string()));
        
        if mspc_channel.send(mspc_msg).await.is_err() {
            print_heart_yellow("⚠️  Failed to send WebSocket message to MSPC channel", true);
            // Fallback to direct processing if MSPC fails
        } else {
            // Message sent to MSPC, no further processing needed here
            return;
        }
    }
    
    // Fallback: Direct processing (for backward compatibility)
    let mut apchat = session.apchat.lock().await;

    // Check if this is the first user message
    let is_first_message = apchat.messages.iter()
        .filter(|m| m.role == "user")
        .count() == 0;

    // Add user message
    apchat.messages.push(apchat_models::Message {
        role: "user".to_string(),
        content: content.clone(),
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning: None,
    });

    // Broadcast user message to all clients in this session
    drop(apchat); // Release lock before broadcast
    session.broadcast(ServerMessage::UserMessage {
        content: content.clone(),
    }).await;

    // Update session activity timestamp
    session.update_activity().await;

    // Generate title if this is the first user message
    if is_first_message {
        if let Some(title) = generate_session_title(&content, session).await {
            session.set_title(Some(title.clone())).await;

            // Broadcast title update to all clients
            session.broadcast(ServerMessage::SessionTitleUpdated {
                title: Some(title),
            }).await;
        }
    }

    // Process chat with broadcasts
    if let Err(e) = handle_chat_with_broadcast(session).await {
        let error_msg = ServerMessage::Error {
            message: format!("Chat failed: {}", e),
            recoverable: true,
        };
        session.broadcast(error_msg).await;
    }

    // Save session to disk after processing message
    let session_id = session.id;
    if let Err(e) = state.session_manager.save_session(&session_id).await {
        print_heart_yellow(&format!("⚠️  Failed to save session after message: {}", e), true);
    }
}

/// Handle SwitchModel
async fn handle_switch_model(
    model: String,
    reason: String,
    session: &Arc<crate::web::session_manager::Session>,
) {
    let mut apchat = session.apchat.lock().await;
    let old_model = apchat.current_model.display_name();

    match apchat.switch_model(&model, &reason) {
        Ok(_) => {
            let new_model = apchat.current_model.display_name();
            let msg = ServerMessage::ModelSwitched {
                old_model: old_model.to_string(),
                new_model: new_model.to_string(),
                reason,
            };
            session.broadcast(msg).await;
        }
        Err(e) => {
            let error_msg = ServerMessage::Error {
                message: format!("Model switch failed: {}", e),
                recoverable: true,
            };
            session.broadcast(error_msg).await;
        }
    }
}

/// Handle UpdateSessionTitle
async fn handle_update_session_title(
    title: Option<String>,
    session: &Arc<crate::web::session_manager::Session>,
    state: &AppState,
) {
    // Update the session title
    session.set_title(title.clone()).await;

    // Broadcast the update to all clients
    session.broadcast(ServerMessage::SessionTitleUpdated {
        title: title.clone(),
    }).await;

    // Save the session to disk
    let session_id = session.id;
    if let Err(e) = state.session_manager.save_session(&session_id).await {
        print_heart_yellow(&format!("⚠️  Failed to save session after title update: {}", e), true);
    }
}

/// Handle CreateSession (from a client wanting to create a new session)
async fn handle_create_session(
    client_id: Uuid,
    config: crate::web::protocol::SessionConfig,
    session: &Arc<crate::web::session_manager::Session>,
    state: &AppState,
) {
    // Note: This is a bit unusual - we're creating a new session from within a session
    // This could be used for spawning new sessions from an existing connection
    match state.session_manager.create_session(config).await {
        Ok(new_session_id) => {
            let msg = ServerMessage::SessionCreated {
                session_id: new_session_id,
                created_at: Utc::now().to_rfc3339(),
            };
            session.send_to_client(client_id, msg).await;
        }
        Err(e) => {
            let error_msg = ServerMessage::Error {
                message: format!("Failed to create session: {}", e),
                recoverable: true,
            };
            session.send_to_client(client_id, error_msg).await;
        }
    }
}

/// Handle JoinSession (from a client wanting to join an existing session)
async fn handle_join_session(
    session_id: SessionId,
    client_id: Uuid,
    session: &Arc<crate::web::session_manager::Session>,
    state: &AppState,
) {
    // First, check if we can join the target session
    if let Some(target_session) = state.session_manager.get_session(&session_id).await {
        // Get WebSocket sender from current session to forward to target
        // This is a simplified implementation - in practice, you'd want to handle
        // session types and attachment logic more carefully
        
        // Add client to target session
        // For now, we'll just broadcast a message to the target session
        let msg = ServerMessage::SessionJoined {
            session_id,
            session_type: target_session.session_type.as_str().to_string(),
            created_at: target_session.created_at.to_rfc3339(),
            current_model: target_session.apchat.lock().await.current_model.display_name().to_string(),
            history: target_session.apchat.lock().await.messages.clone(),
        };
        // Send back to original client
        session.send_to_client(client_id, msg).await;
    } else {
        let error_msg = ServerMessage::Error {
            message: format!("Session not found: {}", session_id),
            recoverable: true,
        };
        session.send_to_client(client_id, error_msg).await;
    }
}

/// Handle LeaveSession
async fn handle_leave_session(
    client_id: Uuid,
    session: &Arc<crate::web::session_manager::Session>,
) {
    // Remove the client from this session
    session.remove_client(client_id).await;
    
    // Send confirmation to client
    let msg = ServerMessage::SessionTitleUpdated { title: None };
    let _ = session.send_to_client(client_id, msg).await;
}

/// Handle CancelExecution
async fn handle_cancel_execution(
    session: &Arc<crate::web::session_manager::Session>,
) {
    // Signal cancellation to the execution system
    // This would typically interrupt the current LLM task or tool execution
    let apchat = session.apchat.lock().await;
    
    // Broadcast cancellation acknowledgment
    let msg = ServerMessage::AssistantMessageComplete;
    session.broadcast(msg).await;
}

/// Handle SaveState
async fn handle_save_state(
    file_path: String,
    session: &Arc<crate::web::session_manager::Session>,
    state: &AppState,
) {
    // Save the current session state to a file
    let session_id = session.id;
    
    // Save session to disk
    if let Err(e) = state.session_manager.save_session(&session_id).await {
        let error_msg = ServerMessage::Error {
            message: format!("Failed to save session state: {}", e),
            recoverable: true,
        };
        session.broadcast(error_msg).await;
    } else {
        // Send success message
        let msg = ServerMessage::SessionTitleUpdated {
            title: Some(format!("Saved to: {}", file_path)),
        };
        session.broadcast(msg).await;
    }
}

/// Handle LoadState
async fn handle_load_state(
    file_path: String,
    session: &Arc<crate::web::session_manager::Session>,
    state: &AppState,
) {
    // Load session state from a file
    // Note: This is a simplified implementation - in practice, you'd need
    // to load from the persistence layer or parse the saved state
    
    let msg = ServerMessage::SessionTitleUpdated {
        title: Some(format!("Loaded from: {}", file_path)),
    };
    session.broadcast(msg).await;
}

/// Handle InvokeSkill
async fn handle_invoke_skill(
    skill_name: String,
    session: &Arc<crate::web::session_manager::Session>,
    state: &AppState,
) {
    // Invoke a skill by name
    let apchat = session.apchat.lock().await;
    
    // Check if skill registry is available
    if let Some(ref skill_registry) = apchat.skill_registry {
        // Execute the skill
        let msg = ServerMessage::SessionTitleUpdated {
            title: Some(format!("Invoking skill: {}", skill_name)),
        };
        session.broadcast(msg).await;
    } else {
        let error_msg = ServerMessage::Error {
            message: "Skill registry not available".to_string(),
            recoverable: true,
        };
        session.broadcast(error_msg).await;
    }
}

/// GET / - Serve index page
async fn serve_index() -> Html<&'static str> {
    Html(include_str!("../../../web/index.html"))
}

/// GET /session/:id - Serve session page
async fn serve_session(Path(_id): Path<SessionId>) -> Html<&'static str> {
    Html(include_str!("../../../web/session.html"))
}

/// Error handling
#[derive(Debug)]
enum AppError {
    Anyhow(anyhow::Error),
    NotFound(String),
    SerdeJson(serde_json::Error),
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Anyhow(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::SerdeJson(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Anyhow(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::SerdeJson(err) => (StatusCode::BAD_REQUEST, err.to_string()),
        };

        let body = Json(serde_json::json!({
            "error": message,
            "status": status.as_u16(),
        }));

        (status, body).into_response()
    }
}
