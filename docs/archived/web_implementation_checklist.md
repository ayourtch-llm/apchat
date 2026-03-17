# Web Frontend Implementation Checklist

This checklist tracks the implementation progress for the APChat web frontend.

## Phase 1: Core Web Server (Week 1-2)

### Backend Setup
- [ ] Add Axum and WebSocket dependencies to `Cargo.toml`
  - [ ] `axum = { version = "0.7", features = ["ws", "macros"] }`
  - [ ] `tower = "0.4"`
  - [ ] `tower-http = { version = "0.5", features = ["fs", "cors"] }`
  - [ ] `tokio-tungstenite = "0.21"`

### Core Modules
- [ ] Create `src/web/mod.rs`
  - [ ] Module structure and exports
- [ ] Create `src/web/server.rs`
  - [ ] Axum server initialization
  - [ ] Static file serving
  - [ ] WebSocket upgrade handler
  - [ ] Graceful shutdown
- [ ] Create `src/web/session_manager.rs`
  - [ ] `SessionManager` struct
  - [ ] Session CRUD operations
  - [ ] Client connection tracking
  - [ ] Session cleanup/timeout
- [ ] Create `src/web/protocol.rs`
  - [ ] `ClientMessage` enum
  - [ ] `ServerMessage` enum
  - [ ] Serialization/deserialization
  - [ ] Message validation
- [ ] Create `src/web/routes.rs`
  - [ ] `GET /api/sessions` - List sessions
  - [ ] `POST /api/sessions` - Create session
  - [ ] `GET /api/sessions/:id` - Get session details
  - [ ] `DELETE /api/sessions/:id` - Close session
  - [ ] `GET /ws/:session_id` - WebSocket endpoint
  - [ ] `GET /` - Serve static index.html
  - [ ] `GET /session/:id` - Serve session view

### CLI Integration
- [ ] Update `src/cli.rs`
  - [ ] Add `--web` flag
  - [ ] Add `--web-port` option
  - [ ] Add `--web-bind` option
  - [ ] Add `--web-api-key` option
  - [ ] Add `--web-dir` option
- [ ] Update `src/main.rs`
  - [ ] Check for `--web` flag
  - [ ] Start web server alongside REPL or standalone
  - [ ] Pass configuration to web server

### Frontend Basics
- [ ] Create `web/` directory structure
- [ ] Create `web/index.html`
  - [ ] Session list page markup
  - [ ] Responsive meta tags
  - [ ] PWA meta tags
- [ ] Create `web/session.html`
  - [ ] Chat interface markup
  - [ ] Message container
  - [ ] Input area
  - [ ] Loading states
- [ ] Create `web/styles/main.css`
  - [ ] Tailwind CSS setup
  - [ ] Color system
  - [ ] Typography
  - [ ] Component styles
- [ ] Create `web/js/app.js`
  - [ ] Session list loading
  - [ ] Session creation
  - [ ] Session navigation
  - [ ] Error handling
- [ ] Create `web/js/websocket.js`
  - [ ] `ChatWebSocket` class
  - [ ] Connection management
  - [ ] Reconnection logic
  - [ ] Message sending/receiving
- [ ] Create `web/js/chat.js`
  - [ ] Chat UI initialization
  - [ ] Message display
  - [ ] User input handling
  - [ ] Streaming response handling

### Testing
- [ ] Manual test: Start server with `--web`
- [ ] Manual test: Access homepage at `http://localhost:8080`
- [ ] Manual test: Create new session
- [ ] Manual test: Send message and receive response
- [ ] Manual test: Streaming responses display correctly
- [ ] Manual test: WebSocket reconnection works
- [ ] Manual test: Multiple browser tabs (different sessions)

## Phase 2: TUI Session Attachment (Week 3)

### Backend Integration
- [ ] Update `src/app/repl.rs`
  - [ ] Add `--web-attachable` flag support
  - [ ] Register TUI session with `SessionManager`
  - [ ] Broadcast TUI messages to web clients
  - [ ] Handle web client messages in TUI
- [ ] Update `src/web/session_manager.rs`
  - [ ] Add `TuiSessionHandle` struct
  - [ ] Add `register_tui_session()` method
  - [ ] Add `attach_to_session()` method
  - [ ] Add `detach_from_session()` method
  - [ ] Implement message broadcasting
- [ ] Update `src/web/protocol.rs`
  - [ ] Add `JoinSession` message
  - [ ] Add `SessionJoined` response with history
  - [ ] Add `AttachMode` enum (ReadOnly, Collaborative, Takeover)

### Frontend Features
- [ ] Update `web/js/app.js`
  - [ ] Display session type (TUI vs Web)
  - [ ] Show "Attachable" badge for TUI sessions
  - [ ] Filter sessions by type
- [ ] Update `web/js/chat.js`
  - [ ] Handle `SessionJoined` message
  - [ ] Display conversation history
  - [ ] Handle multi-client updates
  - [ ] Show connected clients count

### Testing
- [ ] Start TUI with `--web-attachable`
- [ ] Verify TUI session appears in web session list
- [ ] Join TUI session from web
- [ ] Send message from TUI, verify web client receives it
- [ ] Send message from web, verify TUI receives it
- [ ] Test with multiple web clients attached
- [ ] Test session persistence after web disconnect

## Phase 3: Tool Confirmation & Advanced Features (Week 4)

### Tool Confirmation UI
- [ ] Create `web/components/tool-confirm.js`
  - [ ] Tool confirmation modal component
  - [ ] Display tool name and arguments
  - [ ] Display file diff (for edit operations)
  - [ ] Confirm/deny buttons
- [ ] Update `web/js/chat.js`
  - [ ] Handle `ToolCallRequest` message
  - [ ] Show confirmation modal
  - [ ] Send `ConfirmTool` response
  - [ ] Handle `ToolCallResult`

### File Diff Rendering
- [ ] Create `web/components/diff-viewer.js`
  - [ ] Parse unified diff format
  - [ ] Syntax highlighting for diffs
  - [ ] Line-by-line comparison
  - [ ] Expand/collapse sections

### Multi-Agent Progress
- [ ] Create `web/components/agent-progress.js`
  - [ ] Progress bar component
  - [ ] Agent name and task display
  - [ ] Multiple concurrent agents
- [ ] Update `web/js/chat.js`
  - [ ] Handle `TaskProgress` message
  - [ ] Handle `AgentAssigned` message
  - [ ] Update progress indicators
  - [ ] Clear completed tasks

### Session State Management
- [ ] Add save session button to menu
- [ ] Add load session button to menu
- [ ] Handle `SaveState` request
- [ ] Handle `LoadState` request
- [ ] Display state operation results

### Model Switching
- [ ] Add model switching UI to menu
- [ ] Show current model badge
- [ ] Handle `SwitchModel` request
- [ ] Handle `ModelSwitched` response

### Testing
- [ ] Test tool confirmation flow (edit_file)
- [ ] Test tool auto-execution (no confirmation)
- [ ] Test diff rendering (various file types)
- [ ] Test multi-agent progress display
- [ ] Test save/load session state
- [ ] Test model switching

## Phase 4: Mobile Optimization (Week 5)

### Responsive CSS
- [ ] Create `web/styles/mobile.css`
  - [ ] Mobile breakpoints (0-640px)
  - [ ] Tablet breakpoints (641-1024px)
  - [ ] Touch-friendly tap targets (min 44px)
  - [ ] Safe area handling (iOS notch)
- [ ] Update layouts for mobile
  - [ ] Single column session list
  - [ ] Full-screen chat view
  - [ ] Bottom navigation
  - [ ] Collapsible header

### Mobile Interactions
- [ ] Create `web/js/mobile.js`
  - [ ] Device detection
  - [ ] Touch gesture handlers
  - [ ] Swipe navigation
  - [ ] Pull-to-refresh
  - [ ] Keyboard handling
  - [ ] Auto-scroll to input on focus

### iOS Optimizations
- [ ] Add iOS-specific meta tags
  - [ ] `apple-mobile-web-app-capable`
  - [ ] `apple-mobile-web-app-status-bar-style`
  - [ ] Viewport with safe-area-inset
- [ ] Handle iOS quirks
  - [ ] Prevent zoom on input focus
  - [ ] Safe area padding (notch/home indicator)
  - [ ] Disable pull-to-refresh on body
  - [ ] Virtual keyboard overlap handling

### Android Optimizations
- [ ] Add Android-specific meta tags
  - [ ] `theme-color`
- [ ] Handle Android quirks
  - [ ] Back button navigation
  - [ ] Address bar auto-hide
  - [ ] Touch feedback

### PWA Support
- [ ] Create `web/manifest.json`
  - [ ] App name and description
  - [ ] Icons (72, 96, 128, 144, 152, 192, 384, 512)
  - [ ] Display mode: standalone
  - [ ] Theme colors
  - [ ] Screenshots
- [ ] Create app icons
  - [ ] Generate icon set (various sizes)
  - [ ] Save to `web/icons/`
- [ ] Add install prompt
  - [ ] Detect install capability
  - [ ] Show custom install button
  - [ ] Handle beforeinstallprompt event

### Mobile Testing
- [ ] Test on iPhone (Safari)
  - [ ] iPhone 12 and newer
  - [ ] iOS 15+
  - [ ] Portrait and landscape
  - [ ] Add to home screen
- [ ] Test on Android (Chrome)
  - [ ] Pixel devices
  - [ ] Samsung devices
  - [ ] Various screen sizes
  - [ ] Install as PWA
- [ ] Test touch interactions
  - [ ] Tap, swipe, long-press
  - [ ] Virtual keyboard behavior
  - [ ] Scroll performance
- [ ] Test responsive breakpoints
  - [ ] Mobile (< 640px)
  - [ ] Tablet (641-1024px)
  - [ ] Desktop (> 1024px)

## Phase 5: Polish & Production (Week 6)

### Security
- [ ] Implement API key authentication
  - [ ] Environment variable support
  - [ ] WebSocket handshake validation
  - [ ] HTTP header authentication
- [ ] Add CORS configuration
  - [ ] Configurable allowed origins
  - [ ] Development vs production settings
- [ ] Add rate limiting
  - [ ] Per-session limits
  - [ ] Per-IP limits
  - [ ] Configurable thresholds
- [ ] Session access control
  - [ ] Session ownership tracking
  - [ ] Permission levels (read-only, full)
  - [ ] Session expiry

### Performance
- [ ] Optimize bundle size
  - [ ] Minify JavaScript
  - [ ] Minify CSS
  - [ ] Compress static assets
- [ ] Add caching headers
  - [ ] Static assets (1 year)
  - [ ] API responses (no-cache)
- [ ] Implement lazy loading
  - [ ] Code splitting
  - [ ] Syntax highlighter on-demand
  - [ ] Diff viewer on-demand
- [ ] Database/persistence (optional)
  - [ ] Session storage backend
  - [ ] Message history persistence
  - [ ] User preferences

### Error Handling
- [ ] Comprehensive error messages
  - [ ] Network errors
  - [ ] Session errors
  - [ ] Tool execution errors
- [ ] Error recovery
  - [ ] Auto-reconnect on disconnect
  - [ ] Session resume
  - [ ] Partial message recovery
- [ ] User feedback
  - [ ] Toast notifications
  - [ ] Error modals
  - [ ] Loading states

### Documentation
- [ ] User Guide
  - [ ] Getting started
  - [ ] Creating sessions
  - [ ] Joining TUI sessions
  - [ ] Mobile usage
  - [ ] Tool confirmations
- [ ] API Documentation
  - [ ] HTTP endpoints
  - [ ] WebSocket protocol
  - [ ] Message types
  - [ ] Error codes
- [ ] Developer Guide
  - [ ] Architecture overview
  - [ ] Adding features
  - [ ] Customizing UI
  - [ ] Deployment guide
- [ ] README updates
  - [ ] Web frontend section
  - [ ] Configuration options
  - [ ] Environment variables

### Testing
- [ ] Unit tests
  - [ ] Session manager
  - [ ] Protocol serialization
  - [ ] Message routing
- [ ] Integration tests
  - [ ] WebSocket communication
  - [ ] Session attachment
  - [ ] Tool confirmation flow
- [ ] End-to-end tests
  - [ ] Create session and chat
  - [ ] Join TUI session
  - [ ] Multi-client scenarios
- [ ] Performance tests
  - [ ] Concurrent connections
  - [ ] Large message history
  - [ ] Streaming performance

### Deployment
- [ ] Docker support
  - [ ] Dockerfile
  - [ ] docker-compose.yml
  - [ ] Multi-stage build
- [ ] Systemd service
  - [ ] Service file
  - [ ] Auto-restart
  - [ ] Logging
- [ ] Reverse proxy examples
  - [ ] Nginx configuration
  - [ ] Caddy configuration
  - [ ] HTTPS/SSL setup
- [ ] Environment templates
  - [ ] `.env.example`
  - [ ] Production settings
  - [ ] Development settings

## Future Enhancements (Post-Launch)

### Short-Term
- [ ] Dark/light theme toggle
- [ ] Custom color themes
- [ ] Session search and filtering
- [ ] Message search within session
- [ ] Export chat as Markdown/PDF
- [ ] File upload (drag & drop)
- [ ] Voice input (mobile)
- [ ] Image paste from clipboard
- [ ] Keyboard shortcuts help overlay
- [ ] User preferences persistence

### Medium-Term
- [ ] Desktop app (Tauri)
- [ ] Native mobile apps
- [ ] Cloud session sync
- [ ] User accounts
- [ ] Shared session templates
- [ ] Analytics dashboard
- [ ] Collaborative features
  - [ ] Multi-user editing
  - [ ] User cursors
  - [ ] @ mentions

### Long-Term
- [ ] Custom agent creation UI
- [ ] Visual workflow builder
- [ ] Agent marketplace
- [ ] Workflow automation
- [ ] Third-party integrations
- [ ] Enterprise features
  - [ ] SSO integration
  - [ ] Audit logging
  - [ ] Team management
  - [ ] Role-based access control

## Progress Tracking

**Phase 1:** ⬜ Not Started
**Phase 2:** ⬜ Not Started
**Phase 3:** ⬜ Not Started
**Phase 4:** ⬜ Not Started
**Phase 5:** ⬜ Not Started

**Overall Progress:** 0% (0/200+ tasks)

---

**Legend:**
- ⬜ Not Started
- 🟡 In Progress
- ✅ Complete
- ❌ Blocked
- ⏸️ On Hold

**Notes:**
- Update this checklist as work progresses
- Mark items complete only when tested and verified
- Add additional tasks as discovered during implementation
- Link to related issues/PRs for tracking
