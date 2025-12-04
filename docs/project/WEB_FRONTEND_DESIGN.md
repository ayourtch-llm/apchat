# APChat Web Frontend Design

## Overview

This document outlines the design for a web-based frontend for APChat that provides the same functionality as the TUI while supporting:
- Multiple concurrent sessions
- Ability to join existing TUI sessions
- Create and manage independent web sessions
- Session switching via unique URLs
- Lightweight, mobile-responsive UI (desktop, iPhone, Android)

## Architecture

### High-Level Components

```
┌─────────────────────────────────────────────────────────────┐
│                     Web Clients                              │
│  (Desktop Browser / Mobile Safari / Mobile Chrome)          │
└─────────────────┬───────────────────────────────────────────┘
                  │ HTTP/WebSocket
                  ↓
┌─────────────────────────────────────────────────────────────┐
│              Web Server (Axum + WebSocket)                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ HTTP Routes  │  │   WebSocket  │  │   Session    │      │
│  │   Handler    │  │   Handler    │  │   Manager    │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ↓
┌─────────────────────────────────────────────────────────────┐
│                    Session Layer                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Web Session  │  │ TUI Session  │  │   Shared     │      │
│  │   (new)      │  │  (existing)  │  │   Session    │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ↓
┌─────────────────────────────────────────────────────────────┐
│              Core APChat Engine                            │
│     (Existing: chat, tools, agents, terminal, etc.)         │
└─────────────────────────────────────────────────────────────┘
```

### Component Breakdown

#### 1. Web Server (NEW)

**Technology:** Axum web framework (async, Tower-based)

**Location:** `src/web/server.rs`

**Responsibilities:**
- Serve static HTML/CSS/JS assets
- Handle HTTP routes for session management
- Manage WebSocket connections for real-time communication
- Route messages to appropriate sessions
- Handle authentication (optional, future)

**Dependencies to add:**
```toml
axum = { version = "0.7", features = ["ws", "macros"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "cors"] }
tokio-tungstenite = "0.21"
```

#### 2. Session Manager (NEW)

**Location:** `src/web/session_manager.rs`

**Responsibilities:**
- Create new web sessions
- Track active sessions (web + TUI)
- Enable session attachment/detachment
- Broadcast session state changes
- Handle session persistence
- Session lifecycle management

**Key Data Structures:**
```rust
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<SessionId, Arc<Session>>>>,
    tui_session_registry: Arc<RwLock<HashMap<SessionId, TuiSessionHandle>>>,
}

pub struct Session {
    id: SessionId,
    session_type: SessionType,
    apchat: Arc<Mutex<APChat>>,
    clients: Arc<RwLock<Vec<ClientConnection>>>,
    created_at: DateTime<Utc>,
    last_activity: Arc<Mutex<DateTime<Utc>>>,
}

pub enum SessionType {
    Web,           // Standalone web session
    Tui,           // TUI session (can be attached to)
    Shared,        // Multi-client session
}

pub struct ClientConnection {
    client_id: Uuid,
    ws_sender: mpsc::UnboundedSender<WebSocketMessage>,
    joined_at: DateTime<Utc>,
}
```

#### 3. WebSocket Protocol (NEW)

**Location:** `src/web/protocol.rs`

**Message Types:**

```rust
// Client → Server
pub enum ClientMessage {
    // Session management
    CreateSession { config: SessionConfig },
    JoinSession { session_id: SessionId },
    LeaveSession,
    ListSessions,

    // Chat interaction
    SendMessage { content: String },
    ConfirmTool { tool_call_id: String, confirmed: bool },
    CancelExecution,

    // Session control
    SwitchModel { model: String, reason: String },
    SaveState { file_path: String },
    LoadState { file_path: String },

    // Skill system
    InvokeSkill { skill_name: String },
}

// Server → Client
pub enum ServerMessage {
    // Session lifecycle
    SessionCreated { session_id: SessionId },
    SessionJoined { session_id: SessionId, history: Vec<Message> },
    SessionList { sessions: Vec<SessionInfo> },
    SessionError { error: String },

    // Chat responses
    AssistantMessage { content: String, streaming: bool },
    AssistantMessageChunk { chunk: String }, // For streaming
    AssistantMessageComplete,

    // Tool interactions
    ToolCallRequest { tool_call_id: String, name: String, arguments: Value, requires_confirmation: bool },
    ToolCallResult { tool_call_id: String, result: String, success: bool },

    // State updates
    ModelSwitched { new_model: String },
    TokenUsage { total: usize, current: usize },

    // Progress (multi-agent mode)
    TaskProgress { task_id: String, status: TaskStatus, progress: f32 },
    AgentAssigned { agent_name: String, task_description: String },

    // Errors
    Error { message: String, recoverable: bool },
}
```

#### 4. HTTP API Endpoints (NEW)

**Location:** `src/web/routes.rs`

```rust
// Session management
GET  /api/sessions              // List all active sessions
POST /api/sessions              // Create new session
GET  /api/sessions/:id          // Get session details
DELETE /api/sessions/:id        // Close session

// Session attachment
POST /api/sessions/:id/attach   // Attach to existing session
POST /api/sessions/:id/detach   // Detach from session

// State management
POST /api/sessions/:id/save     // Save session state
POST /api/sessions/:id/load     // Load session state

// WebSocket endpoint
GET  /ws/:session_id            // WebSocket connection for session

// Static assets
GET  /                          // Serve web UI
GET  /session/:id               // Serve web UI with session routing
```

#### 5. Frontend Web UI (NEW)

**Technology:** Vanilla JavaScript + Tailwind CSS (or Svelte for small bundle)

**Location:** `web/`

**File Structure:**
```
web/
├── index.html              // Main entry point
├── session.html            // Session view
├── styles/
│   └── main.css           // Tailwind CSS
├── js/
│   ├── app.js             // Main application logic
│   ├── websocket.js       // WebSocket client
│   ├── session.js         // Session management
│   ├── chat.js            // Chat UI components
│   └── mobile.js          // Mobile-specific handling
└── components/
    ├── chat-message.js    // Message component
    ├── tool-confirm.js    // Tool confirmation dialog
    └── session-list.js    // Session list component
```

**UI Features:**
- Responsive design (mobile-first)
- Touch-friendly controls for mobile
- Markdown rendering for messages
- Syntax highlighting for code blocks
- Tool confirmation modals
- Session switcher sidebar
- Streaming response display
- Loading states and error handling
- Offline detection

### Session Management Strategy

#### Creating New Web Sessions

```rust
// User visits: http://localhost:8080/
// Or clicks "New Session" button

1. Client sends: CreateSession { config: SessionConfig }
2. Server creates new APChat instance
3. Server generates unique SessionId (UUID)
4. Server stores session in SessionManager
5. Server redirects client to: /session/{session_id}
6. Client establishes WebSocket at: /ws/{session_id}
7. Server sends: SessionCreated { session_id }
```

#### Attaching to TUI Sessions

```rust
// TUI session runs with --web-attachable flag

1. TUI creates session with known ID
2. TUI registers with SessionManager
3. User visits: http://localhost:8080/sessions
4. Client fetches session list via: GET /api/sessions
5. Client selects TUI session
6. Client navigates to: /session/{tui_session_id}
7. Client establishes WebSocket
8. Server sends current conversation history
9. Both TUI and web clients receive updates
```

#### Session Attachment Modes

**Read-Only Attachment:**
- Multiple clients can view session
- Only original client (TUI) can send messages
- Useful for monitoring/observing

**Collaborative Attachment:**
- Multiple clients can send messages
- Tool confirmations require approval from any client
- Useful for pair programming

**Session Takeover:**
- Web client takes control from TUI
- TUI becomes read-only or disconnects
- Useful for transitioning from terminal to mobile

### State Synchronization

#### Message Broadcasting

```rust
// When assistant sends message or uses tool:

1. APChat generates response/tool call
2. Session broadcasts to all attached clients:
   - For streaming: Send chunks via AssistantMessageChunk
   - For complete: Send AssistantMessage
3. All clients update UI in real-time
```

#### Tool Confirmation Flow

```rust
// When tool requires confirmation:

1. APChat identifies tool needs confirmation
2. Session sends ToolCallRequest to all clients
3. First client to confirm/deny sends response
4. Server locks tool call (prevents duplicate confirmations)
5. Server executes or cancels tool
6. Server broadcasts ToolCallResult to all clients
```

### URL Routing Strategy

**Session URLs:**
```
http://localhost:8080/                    → Session list / Create new
http://localhost:8080/session/new         → Create new session (explicit)
http://localhost:8080/session/{uuid}      → Join/view specific session
http://localhost:8080/session/{uuid}?mode=attach → Attach to existing
http://localhost:8080/session/{uuid}?mode=readonly → Read-only view
```

**Deep Linking:**
- URLs can be shared across devices
- Opening URL on mobile joins same session as desktop
- Session state persists across client disconnects/reconnects

## Mobile Responsiveness

### Design Principles

**Mobile-First Approach:**
1. Design for smallest screen first
2. Progressive enhancement for larger screens
3. Touch-friendly tap targets (minimum 44px)
4. Simplified navigation for mobile

### Layout Breakpoints

```css
/* Mobile: 0-640px */
- Single column layout
- Bottom navigation bar
- Collapsible session list
- Full-screen chat view

/* Tablet: 641-1024px */
- Side panel for session list
- Landscape keyboard optimization
- Tool confirmation as bottom sheet

/* Desktop: 1025px+ */
- Traditional sidebar layout
- Multiple panels
- Keyboard shortcuts
- Tool confirmation as modal
```

### Mobile-Specific Features

**Touch Interactions:**
- Swipe left/right to switch sessions
- Pull-to-refresh for session history
- Long-press for message context menu
- Pinch-to-zoom for code blocks

**iOS Safari Optimizations:**
- Viewport meta tag with safe-area-inset
- Standalone web app mode (add to home screen)
- Disable zoom on input focus
- Handle notch/home indicator spacing

```html
<meta name="viewport" content="width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no, viewport-fit=cover">
<meta name="apple-mobile-web-app-capable" content="yes">
<meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">
```

**Android Chrome Optimizations:**
- Theme color for status bar
- Install prompt for PWA
- Handle back button navigation

```html
<meta name="theme-color" content="#000000">
<link rel="manifest" href="/manifest.json">
```

### Progressive Web App (PWA)

**Make it installable:**

```json
// web/manifest.json
{
  "name": "APChat",
  "short_name": "APChat",
  "description": "Multi-agent AI CLI with web interface",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#000000",
  "theme_color": "#000000",
  "orientation": "portrait-primary",
  "icons": [
    {
      "src": "/icons/icon-192.png",
      "sizes": "192x192",
      "type": "image/png"
    },
    {
      "src": "/icons/icon-512.png",
      "sizes": "512x512",
      "type": "image/png"
    }
  ]
}
```

**Service Worker (optional, future):**
- Offline support
- Background sync
- Push notifications for long-running tasks

## Security Considerations

### Authentication & Authorization

**Phase 1: Optional API Key**
```rust
// Environment variable or config file
WEB_API_KEY=your-secret-key

// Client sends in WebSocket handshake
GET /ws/{session_id}?api_key=your-secret-key
```

**Phase 2: Token-Based Auth (future)**
- JWT tokens
- OAuth integration
- Session-based cookies

### Session Access Control

**Session Ownership:**
- Creator has full control
- Attached clients have limited permissions
- Read-only mode for observers

**Session Privacy:**
- Sessions are private by default
- Optional: Shareable links with tokens
- Session expiry after inactivity

### CORS Configuration

```rust
// Allow specific origins (configurable)
let cors = CorsLayer::new()
    .allow_origin(AllowOrigin::predicate(|origin, _request_parts| {
        // Allow localhost for development
        origin.as_bytes().starts_with(b"http://localhost")
        // Or allow specific domains
        // origin.as_bytes() == b"https://apchat.example.com"
    }))
    .allow_methods([Method::GET, Method::POST, Method::DELETE])
    .allow_headers([CONTENT_TYPE, AUTHORIZATION]);
```

### Rate Limiting

**Per-Session Limits:**
- Max messages per minute
- Max tool calls per hour
- Max concurrent sessions per IP

**Implementation:**
```rust
use tower::limit::RateLimitLayer;

let rate_limit = RateLimitLayer::new(
    100, // max requests
    std::time::Duration::from_secs(60), // per minute
);
```

## Configuration

### CLI Options

```rust
// Extend src/cli.rs

#[derive(Parser)]
pub struct Cli {
    // ... existing options ...

    /// Enable web server
    #[arg(long, default_value = "false")]
    pub web: bool,

    /// Web server port
    #[arg(long, default_value = "8080", env = "APCHAT_WEB_PORT")]
    pub web_port: u16,

    /// Web server bind address
    #[arg(long, default_value = "127.0.0.1", env = "APCHAT_WEB_BIND")]
    pub web_bind: String,

    /// Allow TUI session to be attached from web
    #[arg(long, default_value = "false")]
    pub web_attachable: bool,

    /// Web UI directory (for custom frontends)
    #[arg(long, env = "APCHAT_WEB_DIR")]
    pub web_dir: Option<PathBuf>,

    /// Web API key (optional authentication)
    #[arg(long, env = "APCHAT_WEB_API_KEY")]
    pub web_api_key: Option<String>,
}
```

### Environment Variables

```bash
# Web server configuration
APCHAT_WEB_PORT=8080
APCHAT_WEB_BIND=0.0.0.0        # Bind to all interfaces
APCHAT_WEB_API_KEY=secret123   # Optional authentication

# Session configuration
APCHAT_SESSION_TIMEOUT=3600    # Session timeout in seconds
APCHAT_MAX_SESSIONS=100        # Max concurrent sessions
```

### Config File (future)

```toml
# apchat.toml
[web]
enabled = true
port = 8080
bind = "127.0.0.1"
api_key = "secret123"

[web.sessions]
timeout = 3600
max_sessions = 100
allow_attachment = true

[web.ui]
theme = "dark"
streaming = true
mobile_optimized = true
```

## Implementation Phases

### Phase 1: Core Web Server (Week 1-2)

**Goals:**
- Basic Axum server serving static files
- WebSocket protocol implementation
- Session manager with web-only sessions
- Simple chat UI (desktop-focused)

**Deliverables:**
- [ ] Axum server with HTTP routes
- [ ] WebSocket handler with basic protocol
- [ ] SessionManager with session CRUD
- [ ] Simple HTML/JS chat interface
- [ ] Message sending and receiving
- [ ] Streaming response display

**Files to create:**
- `src/web/mod.rs`
- `src/web/server.rs`
- `src/web/session_manager.rs`
- `src/web/protocol.rs`
- `src/web/routes.rs`
- `web/index.html`
- `web/js/app.js`
- `web/js/websocket.js`

### Phase 2: TUI Session Attachment (Week 3)

**Goals:**
- Allow TUI sessions to be web-attachable
- Session discovery and listing
- Real-time synchronization between TUI and web

**Deliverables:**
- [ ] TUI session registration with SessionManager
- [ ] Session attachment protocol
- [ ] Bidirectional message broadcasting
- [ ] Session list UI
- [ ] Attach/detach functionality

**Files to modify:**
- `src/app/repl.rs` (add web attachment support)
- `src/web/session_manager.rs` (add TUI session handling)

### Phase 3: Tool Confirmation & Advanced Features (Week 4)

**Goals:**
- Tool confirmation UI on web
- File diffs display
- Multi-agent progress visualization
- Session state save/load

**Deliverables:**
- [ ] Tool confirmation modal/dialog
- [ ] Unified diff rendering
- [ ] Agent task progress UI
- [ ] Save/load session state via web
- [ ] Model switching UI

**Files to create:**
- `web/components/tool-confirm.js`
- `web/components/diff-viewer.js`
- `web/components/agent-progress.js`

### Phase 4: Mobile Optimization (Week 5)

**Goals:**
- Mobile-responsive UI
- Touch interactions
- iOS/Android optimizations
- PWA support

**Deliverables:**
- [ ] Responsive CSS (mobile-first)
- [ ] Touch gesture support
- [ ] Mobile navigation
- [ ] PWA manifest and icons
- [ ] iOS Safari optimizations
- [ ] Android Chrome optimizations

**Files to create:**
- `web/styles/mobile.css`
- `web/js/mobile.js`
- `web/manifest.json`

### Phase 5: Polish & Production (Week 6)

**Goals:**
- Security hardening
- Performance optimization
- Documentation
- Testing

**Deliverables:**
- [ ] Authentication/authorization
- [ ] Rate limiting
- [ ] Session cleanup/garbage collection
- [ ] Error handling improvements
- [ ] User documentation
- [ ] API documentation
- [ ] Integration tests

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_creation() {
        let manager = SessionManager::new();
        let session_id = manager.create_session(SessionConfig::default()).await;
        assert!(manager.get_session(&session_id).await.is_some());
    }

    #[tokio::test]
    async fn test_session_attachment() {
        // Test TUI session attachment
    }

    #[tokio::test]
    async fn test_message_broadcasting() {
        // Test message broadcast to multiple clients
    }
}
```

### Integration Tests

```rust
// Test WebSocket communication
#[tokio::test]
async fn test_websocket_chat_flow() {
    // 1. Start server
    // 2. Connect WebSocket client
    // 3. Send message
    // 4. Receive assistant response
    // 5. Verify message in session history
}

// Test session attachment
#[tokio::test]
async fn test_tui_web_attachment() {
    // 1. Create TUI session
    // 2. Register as attachable
    // 3. Connect web client
    // 4. Send message from TUI
    // 5. Verify web client receives update
}
```

### Manual Testing

**Desktop Browsers:**
- Chrome (latest)
- Firefox (latest)
- Safari (latest)
- Edge (latest)

**Mobile Browsers:**
- iOS Safari (iPhone 12+, iOS 15+)
- Android Chrome (Pixel, Samsung)
- Mobile Firefox
- Mobile Edge

**Test Scenarios:**
- [ ] Create new session via web
- [ ] Join existing TUI session
- [ ] Send messages and receive responses
- [ ] Confirm/deny tool calls
- [ ] Switch between sessions
- [ ] Session state persistence
- [ ] Mobile touch interactions
- [ ] Offline/online transitions
- [ ] Multi-client synchronization

## Performance Considerations

### Server Performance

**Connection Limits:**
- Max WebSocket connections per session: 10
- Max concurrent sessions: configurable (default 100)
- Connection timeout: 60 seconds idle

**Memory Management:**
- Session cleanup after inactivity
- Conversation history truncation (same as TUI)
- Streaming response chunking

### Frontend Performance

**Bundle Size:**
- Target: < 100KB initial load (gzipped)
- Code splitting for large components
- Lazy loading for syntax highlighting

**Rendering:**
- Virtual scrolling for long conversations
- Debounced search/filter
- Optimistic UI updates

**Mobile Performance:**
- Minimize reflows/repaints
- Use CSS transforms for animations
- Reduce JavaScript execution on scroll

## Future Enhancements

### Short-Term (3-6 months)

- [ ] Voice input for mobile
- [ ] Collaborative editing (multiple users in same session)
- [ ] Session recording and playback
- [ ] Export conversation as Markdown/PDF
- [ ] Dark/light theme toggle
- [ ] Customizable UI themes

### Medium-Term (6-12 months)

- [ ] Desktop app (Electron/Tauri wrapper)
- [ ] Native mobile apps (React Native/Flutter)
- [ ] Cloud session synchronization
- [ ] User accounts and profiles
- [ ] Shared session templates
- [ ] Analytics and usage insights

### Long-Term (12+ months)

- [ ] Multiplayer collaboration features
- [ ] Integration with IDE extensions
- [ ] Custom agent marketplace
- [ ] Workflow automation builder
- [ ] API for third-party integrations
- [ ] Enterprise features (SSO, audit logs)

## Migration Path

### Gradual Rollout

**Phase 1: Opt-in web server**
```bash
# Existing users continue with TUI
cargo run -- -i

# New users or opt-in users try web
cargo run -- --web --web-port 8080
```

**Phase 2: TUI + Web hybrid**
```bash
# Run TUI with web attachment enabled
cargo run -- -i --web-attachable

# Access from browser while TUI is running
open http://localhost:8080/sessions
```

**Phase 3: Web as default (optional)**
```bash
# Auto-start web server on REPL mode
cargo run -- -i  # Prints: "Web UI available at http://localhost:8080"
```

### Backward Compatibility

- All existing CLI commands continue to work
- TUI remains fully functional without web server
- Web server is optional and opt-in
- No breaking changes to core APChat API

## Deployment Scenarios

### Local Development

```bash
cargo run -- --web --web-bind 127.0.0.1 --web-port 8080
```

### Network Access (LAN)

```bash
# Allow access from other devices on network
cargo run -- --web --web-bind 0.0.0.0 --web-port 8080

# Access from phone/tablet
# http://192.168.1.100:8080
```

### Remote Server (VPS/Cloud)

```bash
# With reverse proxy (nginx/caddy) for HTTPS
cargo run -- --web --web-bind 127.0.0.1 --web-port 8080

# nginx config
server {
    listen 443 ssl;
    server_name apchat.example.com;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

### Docker Deployment

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/apchat /usr/local/bin/
COPY --from=builder /app/web /usr/local/share/apchat/web
EXPOSE 8080
CMD ["apchat", "--web", "--web-bind", "0.0.0.0", "--web-port", "8080"]
```

## Documentation Requirements

### User Documentation

- [ ] Getting started guide (web UI)
- [ ] Session management tutorial
- [ ] Mobile usage guide
- [ ] TUI attachment guide
- [ ] Tool confirmation workflow
- [ ] Troubleshooting common issues

### Developer Documentation

- [ ] Architecture overview
- [ ] WebSocket protocol specification
- [ ] API endpoint reference
- [ ] Frontend customization guide
- [ ] Adding new routes
- [ ] Contributing guidelines

## Success Metrics

### User Experience

- Page load time < 1 second
- WebSocket latency < 100ms
- Mobile responsiveness score > 90 (Lighthouse)
- Touch target size >= 44px
- Session switch time < 500ms

### Reliability

- Uptime > 99% (for long-running server)
- Session recovery on disconnect
- Error rate < 1%
- Memory leak prevention

### Adoption

- 50%+ of users try web UI within 3 months
- 25%+ of users prefer web over TUI
- Positive user feedback (surveys/issues)
- Mobile usage > 20% of total sessions

## Conclusion

This design provides a comprehensive roadmap for adding a web-based frontend to APChat. The phased approach allows for incremental development and testing, while the mobile-first responsive design ensures broad device compatibility. The session management strategy enables flexible workflows from simple standalone web sessions to collaborative multi-client sessions spanning TUI and web interfaces.

Key benefits:
- **Accessibility**: Use APChat from any device with a browser
- **Flexibility**: Switch between TUI and web seamlessly
- **Collaboration**: Multiple users can work in the same session
- **Mobile**: Full functionality on phones and tablets
- **Lightweight**: Minimal dependencies, fast loading
- **Future-proof**: Foundation for PWA, mobile apps, and cloud features
