# Web Frontend UI/UX Wireframes

This document describes the visual design and user experience flows for the APChat web frontend.

## Design System

### Color Palette

**Dark Theme (Default):**
```
Background:
- Primary: #111827 (gray-900)
- Secondary: #1F2937 (gray-800)
- Tertiary: #374151 (gray-700)

Text:
- Primary: #FFFFFF (white)
- Secondary: #D1D5DB (gray-300)
- Tertiary: #9CA3AF (gray-400)

Accent Colors:
- Blue (primary): #2563EB (blue-600)
- Green (success): #10B981 (green-500)
- Red (error): #EF4444 (red-500)
- Yellow (warning): #F59E0B (yellow-500)

Borders:
- Default: #374151 (gray-700)
- Light: #4B5563 (gray-600)
```

**Future: Light Theme Option**

### Typography

```
Font Family: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif
Monospace: "Fira Code", "JetBrains Mono", Consolas, monospace

Sizes:
- Heading 1: 2.5rem (40px) - bold
- Heading 2: 2rem (32px) - bold
- Heading 3: 1.5rem (24px) - semibold
- Body: 1rem (16px) - regular
- Small: 0.875rem (14px) - regular
- Tiny: 0.75rem (12px) - regular

Line Heights:
- Tight: 1.25
- Normal: 1.5
- Relaxed: 1.75
```

### Spacing

```
Scale: 4px base unit
xs: 4px (0.25rem)
sm: 8px (0.5rem)
md: 16px (1rem)
lg: 24px (1.5rem)
xl: 32px (2rem)
2xl: 48px (3rem)
```

### Border Radius

```
sm: 4px
md: 8px
lg: 12px
xl: 16px
full: 9999px (pill shape)
```

## Page Layouts

### 1. Session List Page (Home)

```
┌─────────────────────────────────────────────────────┐
│  Header                                             │
│  🤖 APChat                                        │
│  Multi-agent AI CLI with web interface             │
│                                                     │
│  [+ New Session]                                    │
│                                                     │
│  Active Sessions                                    │
│  ┌───────────────────────────────────────────────┐ │
│  │ 💻 TUI Session          [Attachable]          │ │
│  │ #550e8400                                     │ │
│  │ 📝 15 messages  👥 2 clients  🤖 GrnModel    │ │
│  │ Created: 2h ago  Last: 5m ago                │ │
│  └───────────────────────────────────────────────┘ │
│  ┌───────────────────────────────────────────────┐ │
│  │ 🌐 Web Session                                │ │
│  │ #661f9511                                     │ │
│  │ 📝 5 messages  👥 1 client  🤖 BluModel      │ │
│  │ Created: 1h ago  Last: 10m ago               │ │
│  └───────────────────────────────────────────────┘ │
│                                                     │
└─────────────────────────────────────────────────────┘

MOBILE VIEW:
┌──────────────────┐
│ 🤖 APChat      │
│                  │
│ [+ New Session]  │
│                  │
│ ┌──────────────┐ │
│ │ 💻 TUI       │ │
│ │ #550e        │ │
│ │ 15 msgs      │ │
│ │ 2h ago       │ │
│ └──────────────┘ │
│ ┌──────────────┐ │
│ │ 🌐 Web       │ │
│ │ #661f        │ │
│ │ 5 msgs       │ │
│ │ 1h ago       │ │
│ └──────────────┘ │
│                  │
│ [Home] [+] [⚙️]  │← Bottom nav
└──────────────────┘
```

#### Interactions:
- Click session card → Navigate to session
- Click "New Session" → Create session and redirect
- Pull to refresh → Reload session list (mobile)
- Long press on session → Context menu (future: close, rename)

### 2. Chat Interface

```
DESKTOP VIEW:
┌─────────────────────────────────────────────────────────────┐
│ [←] APChat                      [GrnModel ▼] [⋮]          │
│     Session: #550e8400                                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ System Message                                        │  │
│  │ Current model: GrnModel                              │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│                        ┌──────────────────────────────┐    │
│                        │ Hello! How can I help you?   │    │
│                        │ (User message)                │    │
│                        └──────────────────────────────┘    │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Hi! I'd like to analyze the codebase structure.      │  │
│  │ (Assistant message with markdown)                    │  │
│  │                                                       │  │
│  │ Let me help you with that...                        │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ 🔧 Tool: list_files                                   │  │
│  │ Arguments: { "path": "src" }                         │  │
│  │ Result: main.rs, config/, tools/, ...               │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│ 🤖 planner: Analyzing request... ████░░░░ 40%              │← Agent progress
├─────────────────────────────────────────────────────────────┤
│ ┌─────────────────────────────────────────────┐ [Send]     │
│ │ Type a message...                            │            │
│ └─────────────────────────────────────────────┘            │
└─────────────────────────────────────────────────────────────┘

MOBILE VIEW:
┌──────────────────────┐
│ [←] APChat   [⋮]   │
│ Session: #550e       │
├──────────────────────┤
│                      │
│ System: GrnModel     │
│                      │
│        ┌──────────┐  │
│        │ Hello!   │  │
│        └──────────┘  │
│ ┌──────────────────┐ │
│ │ Hi! I'd like to  │ │
│ │ analyze...       │ │
│ └──────────────────┘ │
│                      │
│ ┌──────────────────┐ │
│ │ 🔧 list_files    │ │
│ │ ✅ Success       │ │
│ └──────────────────┘ │
│                      │
│                      │
│                      │
├──────────────────────┤
│ ┌────────────┐ [📤] │
│ │ Message... │      │
│ └────────────┘      │
└──────────────────────┘
```

#### Message Types:

**User Message (right-aligned, blue):**
```
                    ┌────────────────────────┐
                    │ What files are in src? │
                    │ 10:30 AM               │
                    └────────────────────────┘
```

**Assistant Message (left-aligned, gray):**
```
┌────────────────────────────────────┐
│ I'll check the src directory       │
│ for you...                         │
│                                    │
│ The src directory contains:        │
│ • main.rs - Entry point            │
│ • config/ - Configuration          │
│                        10:30 AM    │
└────────────────────────────────────┘
```

**System Message (centered, subtle):**
```
        ────────────────────────
         Model switched to BluModel
        ────────────────────────
```

**Tool Call (indented, bordered):**
```
┌────────────────────────────────────┐
│ 🔧 Tool: peek_file_top_10_lines                 │
│                                    │
│ Arguments:                         │
│ {                                  │
│   "file_path": "README.md"        │
│ }                                  │
│                                    │
│ ✅ Result: (235 lines)             │
└────────────────────────────────────┘
```

**Error Message (red border):**
```
┌────────────────────────────────────┐
│ ❌ Error                           │
│ File not found: nonexistent.txt    │
└────────────────────────────────────┘
```

### 3. Tool Confirmation Modal

```
DESKTOP VIEW:
┌─────────────────────────────────────────────────────┐
│                     Background (dimmed)             │
│   ┌─────────────────────────────────────────────┐  │
│   │ Confirm Tool Execution                   [×] │  │
│   ├─────────────────────────────────────────────┤  │
│   │                                             │  │
│   │ Tool: edit_file                            │  │
│   │                                             │  │
│   │ Arguments:                                  │  │
│   │ ┌─────────────────────────────────────────┐ │  │
│   │ │ {                                       │ │  │
│   │ │   "file_path": "src/main.rs",          │ │  │
│   │ │   "old_string": "pub fn main() {",     │ │  │
│   │ │   "new_string": "pub async fn main() {"│ │  │
│   │ │ }                                       │ │  │
│   │ └─────────────────────────────────────────┘ │  │
│   │                                             │  │
│   │ Changes:                                    │  │
│   │ ┌─────────────────────────────────────────┐ │  │
│   │ │ --- src/main.rs                         │ │  │
│   │ │ +++ src/main.rs                         │ │  │
│   │ │ @@ -1,1 +1,1 @@                        │ │  │
│   │ │ -pub fn main() {                       │ │  │
│   │ │ +pub async fn main() {                 │ │  │
│   │ └─────────────────────────────────────────┘ │  │
│   │                                             │  │
│   ├─────────────────────────────────────────────┤  │
│   │                      [Deny] [Confirm ✓]    │  │
│   └─────────────────────────────────────────────┘  │
│                                                     │
└─────────────────────────────────────────────────────┘

MOBILE VIEW (Bottom Sheet):
┌──────────────────────┐
│                      │
│  (Pull down to close)│
├──────────────────────┤
│ Confirm Tool         │
├──────────────────────┤
│ Tool: edit_file      │
│                      │
│ Arguments:           │
│ ┌──────────────────┐ │
│ │ {                │ │
│ │   "file_path":   │ │
│ │   "src/main.rs"  │ │
│ │   ...            │ │
│ │ }                │ │
│ └──────────────────┘ │
│                      │
│ Changes:             │
│ ┌──────────────────┐ │
│ │ - pub fn main    │ │
│ │ + pub async fn   │ │
│ └──────────────────┘ │
│                      │
├──────────────────────┤
│ [Deny] [Confirm ✓]  │
└──────────────────────┘
```

### 4. Multi-Agent Progress Indicator

```
DESKTOP VIEW (Below messages, above input):
┌─────────────────────────────────────────────────────┐
│ 🤖 planner: Analyzing request and decomposing into │
│            subtasks                                 │
│ ████████████████░░░░░░░░░░░░ 60%                   │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│ 🔧 file_manager: Reading configuration files        │
│ ████████████████████████████ 100%                  │
└─────────────────────────────────────────────────────┘

MOBILE VIEW:
┌──────────────────────┐
│ 🤖 planner           │
│ Analyzing request... │
│ ██████░░░░░░ 50%    │
└──────────────────────┘
```

### 5. Session Menu (Dropdown)

```
DESKTOP:
                        ┌────────────────────┐
                        │ Switch Model       │
                        │ Save Session       │
                        │ Load Session       │
                        ├────────────────────┤
                        │ Close Session      │← Red text
                        └────────────────────┘

MOBILE (Bottom Sheet):
┌──────────────────────┐
│ Session Options      │
├──────────────────────┤
│ Switch Model         │
│ Save Session         │
│ Load Session         │
├──────────────────────┤
│ Close Session        │← Red
└──────────────────────┘
```

## User Flows

### Flow 1: Creating a New Session

```
1. User visits homepage
   [Session List Page]

2. User clicks "+ New Session"
   [Loading indicator]

3. Server creates session
   [Redirect to /session/{id}]

4. WebSocket connects
   [Chat Interface loads]

5. Ready to chat
   [Input enabled, system message shown]
```

### Flow 2: Joining a TUI Session

```
1. User visits homepage
   [Session List Page with TUI session visible]

2. User clicks TUI session card
   [Navigate to /session/{tui_id}]

3. WebSocket connects and joins
   [Chat Interface loads with history]

4. History displayed
   [Previous messages shown, scrolled to bottom]

5. Ready to participate
   [Input enabled, can send messages]
```

### Flow 3: Chat with Tool Confirmation

```
1. User sends message
   [Message appears in chat, input disabled]

2. Assistant starts responding
   [Streaming chunks appear]

3. Tool call requested (requires confirmation)
   [Modal/bottom sheet appears with diff]

4. User reviews and confirms
   [Modal closes, tool executes]

5. Tool result shown
   [Result message appears in chat]

6. Assistant continues
   [More streaming chunks]

7. Response complete
   [Input re-enabled]
```

### Flow 4: Multi-Agent Task

```
1. User sends complex request
   [Message appears, input disabled]

2. Planner agent starts
   [Progress bar: "planner: Analyzing request..."]

3. Planner assigns subtasks
   [Multiple progress bars appear]

4. Agents execute in parallel
   [Progress bars update independently]

5. Results aggregated
   [Final response appears]

6. Task complete
   [Progress indicators disappear, input enabled]
```

### Flow 5: Mobile Session Switching

```
1. User is in chat session
   [Chat Interface]

2. User swipes right
   [Navigate back to session list]

3. User selects different session
   [New chat interface loads]

4. WebSocket reconnects
   [History loads, ready to chat]
```

## Responsive Breakpoints

### Mobile (0-640px)
- Single column layout
- Bottom navigation
- Full-screen chat
- Tool confirmations as bottom sheets
- Simplified header
- Larger tap targets (min 44px)

### Tablet (641-1024px)
- Optional sidebar for session list
- Landscape optimizations
- Modal dialogs (not bottom sheets)
- Medium header size

### Desktop (1025px+)
- Full sidebar with session list
- Modal dialogs
- Keyboard shortcuts
- Full-featured header
- Multi-panel view (future)

## Accessibility Features

### Keyboard Navigation
- Tab through all interactive elements
- Enter to send message
- Escape to close modals
- Arrow keys for message history (future)
- Shortcuts: Ctrl+K for new session, Ctrl+/ for commands

### Screen Reader Support
- Semantic HTML (header, nav, main, article)
- ARIA labels for icons
- ARIA live regions for streaming messages
- Role="status" for progress indicators
- Alt text for all images/icons

### High Contrast Mode
- Respect prefers-contrast media query
- Ensure 4.5:1 contrast ratio (WCAG AA)
- Bold borders for focus states

### Reduced Motion
- Respect prefers-reduced-motion
- Disable animations when requested
- Instant transitions instead of smooth

## Loading States

### Initial Page Load
```
┌──────────────────────┐
│                      │
│   ┌────────────┐     │
│   │    ⏳      │     │
│   │  Loading...│     │
│   └────────────┘     │
│                      │
└──────────────────────┘
```

### Session List Loading
```
┌──────────────────────┐
│ Sessions             │
│ ┌──────────────────┐ │
│ │ ▪▪▪▪▪▪▪▪▪▪▪▪▪▪  │ │← Skeleton loader
│ │ ▪▪▪▪▪▪▪▪         │ │
│ └──────────────────┘ │
│ ┌──────────────────┐ │
│ │ ▪▪▪▪▪▪▪▪▪▪▪▪▪▪  │ │
│ │ ▪▪▪▪▪▪▪▪         │ │
│ └──────────────────┘ │
└──────────────────────┘
```

### Message Sending
```
┌────────────────────────┐
│ Your message here...   │
│ Sending... ⏳          │
└────────────────────────┘
```

### Streaming Response
```
┌────────────────────────┐
│ I'll help you with... █│← Blinking cursor
└────────────────────────┘
```

## Error States

### Network Error
```
┌────────────────────────────────┐
│ ⚠️ Connection Lost             │
│                                │
│ Unable to connect to server.   │
│                                │
│ [Retry] [Go Back]              │
└────────────────────────────────┘
```

### Session Not Found
```
┌────────────────────────────────┐
│ ❌ Session Not Found           │
│                                │
│ This session may have expired  │
│ or been deleted.               │
│                                │
│ [Back to Sessions]             │
└────────────────────────────────┘
```

### Tool Execution Error
```
┌────────────────────────────────┐
│ ❌ Tool Execution Failed       │
│                                │
│ Error: File not found          │
│                                │
│ The assistant will be notified │
│ and can try a different        │
│ approach.                      │
└────────────────────────────────┘
```

## Animation & Transitions

### Message Appearance
- Fade in + slide up (200ms ease-out)
- Stagger delay for multiple messages (50ms)

### Modal/Dialog
- Fade in background (150ms)
- Scale up content (200ms spring)

### Progress Bar
- Smooth width transition (300ms ease-in-out)
- Pulse animation for indeterminate state

### Button Hover
- Background color transition (150ms)
- Scale transform (100ms)

### Mobile Bottom Sheet
- Slide up from bottom (250ms ease-out)
- Backdrop fade in (200ms)

## Icon Set

Use [Heroicons](https://heroicons.com/) or similar for consistency:

- Send: Paper airplane
- Back: Arrow left
- Menu: Three dots vertical
- Close: X
- Confirm: Check
- Deny: X in circle
- User: User circle
- Assistant: Sparkles or robot
- Tool: Wrench or cog
- Success: Check in circle (green)
- Error: Exclamation in circle (red)
- Warning: Exclamation triangle (yellow)
- Info: Info circle (blue)
- Loading: Spinner or dots

## Code Syntax Highlighting

Use [Prism.js](https://prismjs.com/) or [Highlight.js](https://highlightjs.org/) for code blocks:

**Supported Languages:**
- Rust
- JavaScript/TypeScript
- Python
- HTML/CSS
- JSON
- Markdown
- Bash/Shell
- And more...

**Theme:** VS Code Dark+ or similar dark theme to match UI

## Markdown Rendering

Use [marked.js](https://marked.js.org/) or [markdown-it](https://github.com/markdown-it/markdown-it):

**Supported Elements:**
- Headings (h1-h6)
- Bold, italic, strikethrough
- Lists (ordered, unordered)
- Links
- Inline code, code blocks
- Blockquotes
- Tables
- Task lists
- Images (with lazy loading)

## Future UI Enhancements

### Phase 2+
- [ ] Dark/light theme toggle
- [ ] Custom color themes
- [ ] Session search/filter
- [ ] Message search within session
- [ ] Export chat as Markdown/PDF
- [ ] File upload (drag & drop)
- [ ] Voice input (mobile)
- [ ] Image paste from clipboard
- [ ] Split view (multiple sessions)
- [ ] Session folders/organization
- [ ] Keyboard shortcuts help overlay
- [ ] Settings panel
- [ ] User preferences persistence

### Advanced Features
- [ ] Collaborative cursors (multiplayer)
- [ ] @ mentions for agents
- [ ] Emoji reactions to messages
- [ ] Message threading
- [ ] Bookmarks/favorites
- [ ] Session templates
- [ ] Custom agent creation UI
- [ ] Visual agent workflow builder
- [ ] Performance monitoring dashboard

This completes the UI/UX wireframe specifications!
