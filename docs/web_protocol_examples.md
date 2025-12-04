# Web Frontend Protocol Examples

This document provides concrete examples of the WebSocket protocol messages and flows for the APChat web frontend.

## Message Flow Examples

### 1. Creating a New Session

**Client → Server:**
```json
{
  "type": "CreateSession",
  "data": {
    "config": {
      "model": "GrnModel",
      "agents_enabled": true,
      "stream_responses": true
    }
  }
}
```

**Server → Client:**
```json
{
  "type": "SessionCreated",
  "data": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "created_at": "2025-11-15T10:30:00Z"
  }
}
```

### 2. Joining an Existing Session

**Client → Server:**
```json
{
  "type": "JoinSession",
  "data": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

**Server → Client:**
```json
{
  "type": "SessionJoined",
  "data": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "session_type": "Tui",
    "created_at": "2025-11-15T10:30:00Z",
    "current_model": "GrnModel",
    "history": [
      {
        "role": "system",
        "content": "System prompt...",
        "timestamp": "2025-11-15T10:30:00Z"
      },
      {
        "role": "user",
        "content": "Hello!",
        "timestamp": "2025-11-15T10:30:15Z"
      },
      {
        "role": "assistant",
        "content": "Hi! How can I help you today?",
        "timestamp": "2025-11-15T10:30:16Z"
      }
    ]
  }
}
```

### 3. Sending a Chat Message

**Client → Server:**
```json
{
  "type": "SendMessage",
  "data": {
    "content": "Please read the README.md file"
  }
}
```

**Server → Client (Streaming):**
```json
{
  "type": "AssistantMessageChunk",
  "data": {
    "chunk": "I'll read the README.md file for you.\n\n"
  }
}
```

```json
{
  "type": "ToolCallRequest",
  "data": {
    "tool_call_id": "call_abc123",
    "name": "read_file",
    "arguments": {
      "file_path": "README.md"
    },
    "requires_confirmation": false
  }
}
```

```json
{
  "type": "ToolCallResult",
  "data": {
    "tool_call_id": "call_abc123",
    "result": "# APChat\n\nA multi-agent AI CLI...",
    "success": true
  }
}
```

```json
{
  "type": "AssistantMessageChunk",
  "data": {
    "chunk": "The README.md file contains information about APChat..."
  }
}
```

```json
{
  "type": "AssistantMessageComplete",
  "data": {}
}
```

### 4. Tool Confirmation Flow

**Server → Client:**
```json
{
  "type": "ToolCallRequest",
  "data": {
    "tool_call_id": "call_def456",
    "name": "edit_file",
    "arguments": {
      "file_path": "src/main.rs",
      "old_string": "pub fn main() {",
      "new_string": "pub async fn main() {"
    },
    "requires_confirmation": true,
    "diff": "--- src/main.rs\n+++ src/main.rs\n@@ -1,1 +1,1 @@\n-pub fn main() {\n+pub async fn main() {"
  }
}
```

**Client → Server:**
```json
{
  "type": "ConfirmTool",
  "data": {
    "tool_call_id": "call_def456",
    "confirmed": true
  }
}
```

**Server → Client:**
```json
{
  "type": "ToolCallResult",
  "data": {
    "tool_call_id": "call_def456",
    "result": "File edited successfully: src/main.rs",
    "success": true
  }
}
```

### 5. Cancelling Execution

**Client → Server:**
```json
{
  "type": "CancelExecution",
  "data": {}
}
```

**Server → Client:**
```json
{
  "type": "Error",
  "data": {
    "message": "Execution cancelled by user",
    "recoverable": true
  }
}
```

### 6. Multi-Agent Progress Updates

**Server → Client:**
```json
{
  "type": "TaskProgress",
  "data": {
    "task_id": "task_001",
    "agent_name": "planner",
    "status": "Planning",
    "progress": 0.1,
    "description": "Analyzing user request and decomposing into subtasks"
  }
}
```

```json
{
  "type": "AgentAssigned",
  "data": {
    "agent_name": "file_manager",
    "task_id": "task_002",
    "task_description": "Read configuration files"
  }
}
```

```json
{
  "type": "TaskProgress",
  "data": {
    "task_id": "task_002",
    "agent_name": "file_manager",
    "status": "TaskExecution",
    "progress": 0.5,
    "description": "Reading src/config/mod.rs"
  }
}
```

```json
{
  "type": "TaskProgress",
  "data": {
    "task_id": "task_002",
    "agent_name": "file_manager",
    "status": "Completed",
    "progress": 1.0,
    "description": "Successfully read 3 configuration files"
  }
}
```

### 7. Session List Request

**Client → Server:**
```json
{
  "type": "ListSessions",
  "data": {}
}
```

**Server → Client:**
```json
{
  "type": "SessionList",
  "data": {
    "sessions": [
      {
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "type": "Tui",
        "created_at": "2025-11-15T10:30:00Z",
        "last_activity": "2025-11-15T10:45:30Z",
        "active_clients": 2,
        "message_count": 15,
        "current_model": "GrnModel",
        "attachable": true
      },
      {
        "id": "661f9511-f3ac-52e5-b827-557766551111",
        "type": "Web",
        "created_at": "2025-11-15T11:00:00Z",
        "last_activity": "2025-11-15T11:10:00Z",
        "active_clients": 1,
        "message_count": 5,
        "current_model": "BluModel",
        "attachable": false
      }
    ]
  }
}
```

### 8. Model Switching

**Client → Server:**
```json
{
  "type": "SwitchModel",
  "data": {
    "model": "BluModel",
    "reason": "Need better code analysis"
  }
}
```

**Server → Client:**
```json
{
  "type": "ModelSwitched",
  "data": {
    "old_model": "GrnModel",
    "new_model": "BluModel",
    "reason": "Need better code analysis"
  }
}
```

### 9. Error Handling

**Server → Client (Recoverable Error):**
```json
{
  "type": "Error",
  "data": {
    "message": "File not found: nonexistent.txt",
    "recoverable": true,
    "context": {
      "tool_call_id": "call_xyz789",
      "tool_name": "read_file"
    }
  }
}
```

**Server → Client (Non-Recoverable Error):**
```json
{
  "type": "Error",
  "data": {
    "message": "Session not found or expired",
    "recoverable": false
  }
}
```

### 10. Saving Session State

**Client → Server:**
```json
{
  "type": "SaveState",
  "data": {
    "file_path": "my_session.json"
  }
}
```

**Server → Client:**
```json
{
  "type": "StateOperation",
  "data": {
    "operation": "save",
    "success": true,
    "message": "Session state saved to my_session.json (25 messages, 15234 tokens)"
  }
}
```

### 11. Loading Session State

**Client → Server:**
```json
{
  "type": "LoadState",
  "data": {
    "file_path": "my_session.json"
  }
}
```

**Server → Client:**
```json
{
  "type": "StateOperation",
  "data": {
    "operation": "load",
    "success": true,
    "message": "Loaded conversation state from my_session.json (25 messages, 15234 total tokens)",
    "history": [
      // ... loaded message history
    ]
  }
}
```

### 12. Skill Invocation

**Client → Server:**
```json
{
  "type": "InvokeSkill",
  "data": {
    "skill_name": "brainstorming"
  }
}
```

**Server → Client:**
```json
{
  "type": "SkillInvoked",
  "data": {
    "skill_name": "brainstorming",
    "description": "Socratic method for design refinement",
    "active": true
  }
}
```

## Complete Chat Flow Example

Here's a complete example of a typical chat interaction:

```javascript
// 1. Client connects and creates session
ws.send({
  type: "CreateSession",
  data: { config: { model: "GrnModel", agents_enabled: false } }
});

// 2. Server responds with session ID
← { type: "SessionCreated", data: { session_id: "abc-123" } }

// 3. Client sends message
ws.send({
  type: "SendMessage",
  data: { content: "What files are in the src directory?" }
});

// 4. Assistant starts responding (streaming)
← { type: "AssistantMessageChunk", data: { chunk: "I'll check the src directory for you.\n\n" } }

// 5. Tool call requested
← {
  type: "ToolCallRequest",
  data: {
    tool_call_id: "call_001",
    name: "list_files",
    arguments: { path: "src" },
    requires_confirmation: false
  }
}

// 6. Tool executes (auto-confirmed)
← {
  type: "ToolCallResult",
  data: {
    tool_call_id: "call_001",
    result: "main.rs\nconfig/\ntools/\nagents/\n...",
    success: true
  }
}

// 7. Assistant continues response
← { type: "AssistantMessageChunk", data: { chunk: "The src directory contains:\n- main.rs (main entry point)\n..." } }

// 8. Response complete
← { type: "AssistantMessageComplete", data: {} }

// 9. Token usage update
← { type: "TokenUsage", data: { total: 15234, current: 523 } }
```

## WebSocket Connection Lifecycle

```javascript
// 1. Establish connection
const ws = new WebSocket(`ws://localhost:8080/ws/${sessionId}`);

// 2. Connection opened
ws.onopen = () => {
  console.log('Connected to session:', sessionId);
};

// 3. Receive messages
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  handleServerMessage(message);
};

// 4. Handle errors
ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

// 5. Connection closed
ws.onclose = (event) => {
  if (event.wasClean) {
    console.log('Connection closed cleanly');
  } else {
    console.error('Connection died');
    // Attempt reconnection
    setTimeout(() => reconnect(), 1000);
  }
};

// 6. Send message helper
function send(type, data) {
  ws.send(JSON.stringify({ type, data }));
}
```

## HTTP API Examples

### GET /api/sessions

**Request:**
```http
GET /api/sessions HTTP/1.1
Host: localhost:8080
```

**Response:**
```json
{
  "sessions": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "type": "Tui",
      "created_at": "2025-11-15T10:30:00Z",
      "last_activity": "2025-11-15T10:45:30Z",
      "active_clients": 2,
      "message_count": 15
    }
  ]
}
```

### POST /api/sessions

**Request:**
```http
POST /api/sessions HTTP/1.1
Host: localhost:8080
Content-Type: application/json

{
  "config": {
    "model": "GrnModel",
    "agents_enabled": true,
    "stream_responses": true
  }
}
```

**Response:**
```json
{
  "session_id": "661f9511-f3ac-52e5-b827-557766551111",
  "created_at": "2025-11-15T11:00:00Z",
  "websocket_url": "ws://localhost:8080/ws/661f9511-f3ac-52e5-b827-557766551111"
}
```

### GET /api/sessions/:id

**Request:**
```http
GET /api/sessions/550e8400-e29b-41d4-a716-446655440000 HTTP/1.1
Host: localhost:8080
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "type": "Tui",
  "created_at": "2025-11-15T10:30:00Z",
  "last_activity": "2025-11-15T10:45:30Z",
  "active_clients": 2,
  "message_count": 15,
  "current_model": "GrnModel",
  "attachable": true,
  "history": [
    // ... message history (last 50 messages)
  ]
}
```

### DELETE /api/sessions/:id

**Request:**
```http
DELETE /api/sessions/550e8400-e29b-41d4-a716-446655440000 HTTP/1.1
Host: localhost:8080
```

**Response:**
```json
{
  "success": true,
  "message": "Session closed successfully"
}
```

## Error Responses

### Session Not Found

```json
{
  "error": "SessionNotFound",
  "message": "Session 550e8400-e29b-41d4-a716-446655440000 not found or expired",
  "status": 404
}
```

### Unauthorized

```json
{
  "error": "Unauthorized",
  "message": "Invalid or missing API key",
  "status": 401
}
```

### Rate Limited

```json
{
  "error": "RateLimitExceeded",
  "message": "Too many requests. Please wait 30 seconds.",
  "retry_after": 30,
  "status": 429
}
```

### Internal Server Error

```json
{
  "error": "InternalServerError",
  "message": "An unexpected error occurred",
  "status": 500
}
```
