use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{Html, IntoResponse, Json, Response},
    routing::{delete, get, post},
    Router,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::web::{
    protocol::{ClientMessage, ServerMessage, SessionConfig, SessionId, SessionInfo},
    session_manager::SessionManager,
};

/// Application state shared across routes
#[derive(Clone)]
pub struct AppState {
    pub session_manager: Arc<SessionManager>,
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
async fn list_sessions(State(state): State<AppState>) -> Json<serde_json::Value> {
    let sessions = state.session_manager.list_sessions().await;
    Json(serde_json::json!({ "sessions": sessions }))
}

/// POST /api/sessions - Create a new session
async fn create_session(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let config: SessionConfig = serde_json::from_value(
        payload
            .get("config")
            .cloned()
            .unwrap_or(serde_json::json!({})),
    )?;

    let session_id = state.session_manager.create_session(config).await?;

    Ok(Json(serde_json::json!({
        "session_id": session_id,
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
            eprintln!("WebSocket: Session {} not found", session_id);
            return;
        }
    };

    // Create channel for sending messages to this client
    let (ws_sender, mut ws_receiver) = mpsc::unbounded_channel();

    // Add client to session
    session.add_client(client_id, ws_sender).await;

    // Send SessionJoined message
    let kimichat = session.kimichat.lock().await;
    let history = kimichat.messages.clone();
    let current_model = kimichat.current_model.display_name();
    drop(kimichat);

    let join_msg = ServerMessage::SessionJoined {
        session_id,
        session_type: session.session_type.as_str().to_string(),
        created_at: session.created_at.to_rfc3339(),
        current_model,
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
            if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                handle_client_message(client_id, client_msg, &session_clone, &state).await;
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
            handle_send_message(client_id, content, session).await;
        }
        ListSessions => {
            let sessions = state.session_manager.list_sessions().await;
            let msg = ServerMessage::SessionList { sessions };
            session.send_to_client(client_id, msg).await;
        }
        SwitchModel { model, reason } => {
            handle_switch_model(model, reason, session).await;
        }
        _ => {
            // TODO: Implement other message handlers
            eprintln!("Unhandled client message: {:?}", message);
        }
    }
}

/// Handle SendMessage
async fn handle_send_message(
    _client_id: Uuid,
    content: String,
    session: &Arc<crate::web::session_manager::Session>,
) {
    let mut kimichat = session.kimichat.lock().await;

    // Add user message
    kimichat.messages.push(crate::models::Message {
        role: "user".to_string(),
        content: content.clone(),
        tool_calls: None,
        tool_call_id: None,
        name: None,
    });

    // Call chat session (simplified for now - no streaming)
    let result = if kimichat.use_agents {
        match kimichat
            .process_with_agents(&content, None)
            .await
        {
            Ok(response) => response,
            Err(e) => {
                let error_msg = ServerMessage::Error {
                    message: format!("Agent processing failed: {}", e),
                    recoverable: true,
                };
                session.broadcast(error_msg).await;
                return;
            }
        }
    } else {
        match crate::chat::session::chat(&mut kimichat, &content, None).await {
            Ok(response) => response,
            Err(e) => {
                let error_msg = ServerMessage::Error {
                    message: format!("Chat failed: {}", e),
                    recoverable: true,
                };
                session.broadcast(error_msg).await;
                return;
            }
        }
    };

    // Broadcast response
    let msg = ServerMessage::AssistantMessage {
        content: result,
        streaming: false,
    };
    session.broadcast(msg).await;
    session.broadcast(ServerMessage::AssistantMessageComplete).await;
}

/// Handle SwitchModel
async fn handle_switch_model(
    model: String,
    reason: String,
    session: &Arc<crate::web::session_manager::Session>,
) {
    let mut kimichat = session.kimichat.lock().await;
    let old_model = kimichat.current_model.display_name();

    match kimichat.switch_model(&model, &reason) {
        Ok(_) => {
            let new_model = kimichat.current_model.display_name();
            let msg = ServerMessage::ModelSwitched {
                old_model,
                new_model,
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

/// GET / - Serve index page
async fn serve_index() -> Html<&'static str> {
    Html(include_str!("../../web/index.html"))
}

/// GET /session/:id - Serve session page
async fn serve_session(Path(_id): Path<SessionId>) -> Html<&'static str> {
    Html(include_str!("../../web/session.html"))
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
