# Web Frontend Code Examples

This document provides example code for the web frontend implementation.

## Example HTML Structure

### index.html (Session List Page)

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no, viewport-fit=cover">
    <meta name="apple-mobile-web-app-capable" content="yes">
    <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">
    <meta name="theme-color" content="#000000">
    <title>APChat - Sessions</title>
    <link rel="manifest" href="/manifest.json">
    <link rel="stylesheet" href="/styles/main.css">
</head>
<body class="bg-gray-900 text-white">
    <div class="container mx-auto px-4 py-8">
        <header class="mb-8">
            <h1 class="text-4xl font-bold mb-2">🤖 APChat</h1>
            <p class="text-gray-400">Multi-agent AI CLI with web interface</p>
        </header>

        <div class="mb-6">
            <button id="newSessionBtn" class="bg-blue-600 hover:bg-blue-700 px-6 py-3 rounded-lg font-semibold">
                + New Session
            </button>
        </div>

        <div id="sessionList" class="space-y-4">
            <!-- Session cards will be inserted here -->
        </div>

        <div id="loading" class="hidden text-center py-8">
            <div class="inline-block animate-spin rounded-full h-12 w-12 border-b-2 border-white"></div>
            <p class="mt-4 text-gray-400">Loading sessions...</p>
        </div>

        <div id="error" class="hidden bg-red-900 border border-red-700 rounded-lg p-4 mt-4">
            <p class="font-semibold">Error</p>
            <p id="errorMessage" class="text-sm text-gray-300"></p>
        </div>
    </div>

    <script src="/js/app.js" type="module"></script>
</body>
</html>
```

### session.html (Chat Interface)

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no, viewport-fit=cover">
    <meta name="apple-mobile-web-app-capable" content="yes">
    <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">
    <meta name="theme-color" content="#000000">
    <title>APChat - Session</title>
    <link rel="manifest" href="/manifest.json">
    <link rel="stylesheet" href="/styles/main.css">
    <link rel="stylesheet" href="/styles/mobile.css">
</head>
<body class="bg-gray-900 text-white h-screen flex flex-col">
    <!-- Header -->
    <header class="bg-gray-800 border-b border-gray-700 px-4 py-3 flex items-center justify-between">
        <div class="flex items-center space-x-3">
            <a href="/" class="text-gray-400 hover:text-white">
                <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"/>
                </svg>
            </a>
            <div>
                <h1 class="text-lg font-semibold">APChat</h1>
                <p id="sessionInfo" class="text-xs text-gray-400">Session: ...</p>
            </div>
        </div>
        <div class="flex items-center space-x-2">
            <span id="modelBadge" class="px-2 py-1 bg-blue-600 rounded text-xs font-semibold">GrnModel</span>
            <button id="menuBtn" class="text-gray-400 hover:text-white">
                <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z"/>
                </svg>
            </button>
        </div>
    </header>

    <!-- Chat Messages -->
    <div id="chatContainer" class="flex-1 overflow-y-auto px-4 py-6 space-y-4">
        <!-- Messages will be inserted here -->
    </div>

    <!-- Agent Progress (Multi-Agent Mode) -->
    <div id="agentProgress" class="hidden bg-gray-800 border-t border-gray-700 px-4 py-3">
        <div class="flex items-center space-x-3">
            <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-500"></div>
            <div class="flex-1">
                <p id="agentName" class="text-sm font-semibold">planner</p>
                <p id="agentTask" class="text-xs text-gray-400">Analyzing request...</p>
            </div>
            <div id="agentProgressBar" class="w-24 h-2 bg-gray-700 rounded-full overflow-hidden">
                <div class="h-full bg-blue-500 transition-all duration-300" style="width: 0%"></div>
            </div>
        </div>
    </div>

    <!-- Input Area -->
    <div class="bg-gray-800 border-t border-gray-700 px-4 py-4">
        <div class="flex space-x-2">
            <textarea
                id="messageInput"
                placeholder="Type a message..."
                rows="1"
                class="flex-1 bg-gray-700 text-white rounded-lg px-4 py-3 resize-none focus:outline-none focus:ring-2 focus:ring-blue-500"
            ></textarea>
            <button
                id="sendBtn"
                class="bg-blue-600 hover:bg-blue-700 px-6 py-3 rounded-lg font-semibold disabled:opacity-50 disabled:cursor-not-allowed"
            >
                Send
            </button>
        </div>
        <div id="inputStatus" class="mt-2 text-xs text-gray-400 hidden"></div>
    </div>

    <!-- Tool Confirmation Modal -->
    <div id="toolConfirmModal" class="hidden fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
        <div class="bg-gray-800 rounded-lg max-w-2xl w-full max-h-[80vh] overflow-hidden">
            <div class="px-6 py-4 border-b border-gray-700">
                <h2 class="text-xl font-semibold">Confirm Tool Execution</h2>
            </div>
            <div class="px-6 py-4 overflow-y-auto max-h-96">
                <p class="text-sm text-gray-400 mb-2">Tool: <span id="toolName" class="text-white font-mono"></span></p>
                <pre id="toolArguments" class="bg-gray-900 rounded p-3 text-xs overflow-x-auto"></pre>
                <div id="toolDiff" class="mt-4 hidden">
                    <p class="text-sm text-gray-400 mb-2">Changes:</p>
                    <pre class="bg-gray-900 rounded p-3 text-xs overflow-x-auto"></pre>
                </div>
            </div>
            <div class="px-6 py-4 border-t border-gray-700 flex justify-end space-x-3">
                <button id="toolDenyBtn" class="px-4 py-2 bg-gray-700 hover:bg-gray-600 rounded">Deny</button>
                <button id="toolConfirmBtn" class="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded">Confirm</button>
            </div>
        </div>
    </div>

    <!-- Menu Dropdown -->
    <div id="menuDropdown" class="hidden fixed top-16 right-4 bg-gray-800 border border-gray-700 rounded-lg shadow-lg z-40 min-w-48">
        <a href="#" id="switchModelBtn" class="block px-4 py-2 hover:bg-gray-700">Switch Model</a>
        <a href="#" id="saveStateBtn" class="block px-4 py-2 hover:bg-gray-700">Save Session</a>
        <a href="#" id="loadStateBtn" class="block px-4 py-2 hover:bg-gray-700">Load Session</a>
        <div class="border-t border-gray-700"></div>
        <a href="#" id="closeSessionBtn" class="block px-4 py-2 hover:bg-gray-700 text-red-400">Close Session</a>
    </div>

    <script src="/js/websocket.js" type="module"></script>
    <script src="/js/chat.js" type="module"></script>
    <script src="/js/mobile.js" type="module"></script>
</body>
</html>
```

## Example JavaScript Code

### js/app.js (Session List)

```javascript
// Session list page logic

const API_BASE = window.location.origin;

// Load sessions on page load
document.addEventListener('DOMContentLoaded', async () => {
    await loadSessions();

    // New session button
    document.getElementById('newSessionBtn').addEventListener('click', createNewSession);
});

async function loadSessions() {
    showLoading(true);
    hideError();

    try {
        const response = await fetch(`${API_BASE}/api/sessions`);
        if (!response.ok) throw new Error('Failed to load sessions');

        const data = await response.json();
        displaySessions(data.sessions);
    } catch (error) {
        showError(error.message);
    } finally {
        showLoading(false);
    }
}

function displaySessions(sessions) {
    const container = document.getElementById('sessionList');
    container.innerHTML = '';

    if (sessions.length === 0) {
        container.innerHTML = `
            <div class="text-center py-12 text-gray-400">
                <p class="text-lg mb-2">No active sessions</p>
                <p class="text-sm">Click "New Session" to get started</p>
            </div>
        `;
        return;
    }

    sessions.forEach(session => {
        const card = createSessionCard(session);
        container.appendChild(card);
    });
}

function createSessionCard(session) {
    const card = document.createElement('div');
    card.className = 'bg-gray-800 rounded-lg p-4 hover:bg-gray-750 transition cursor-pointer';
    card.onclick = () => joinSession(session.id);

    const typeIcon = session.type === 'Tui' ? '💻' : '🌐';
    const typeColor = session.type === 'Tui' ? 'text-green-400' : 'text-blue-400';

    card.innerHTML = `
        <div class="flex items-start justify-between">
            <div class="flex-1">
                <div class="flex items-center space-x-2 mb-2">
                    <span class="text-2xl">${typeIcon}</span>
                    <span class="${typeColor} font-semibold">${session.type} Session</span>
                    ${session.attachable ? '<span class="text-xs bg-green-900 text-green-300 px-2 py-1 rounded">Attachable</span>' : ''}
                </div>
                <p class="text-sm text-gray-400 mb-1">
                    <span class="text-white font-mono text-xs">${session.id.substring(0, 8)}</span>
                </p>
                <div class="flex items-center space-x-4 text-xs text-gray-400">
                    <span>📝 ${session.message_count} messages</span>
                    <span>👥 ${session.active_clients} client${session.active_clients !== 1 ? 's' : ''}</span>
                    <span>🤖 ${session.current_model}</span>
                </div>
            </div>
            <div class="text-right text-xs text-gray-500">
                <p>${formatTimestamp(session.created_at)}</p>
                <p class="text-gray-600">Last: ${formatTimestamp(session.last_activity)}</p>
            </div>
        </div>
    `;

    return card;
}

async function createNewSession() {
    showLoading(true);
    hideError();

    try {
        const response = await fetch(`${API_BASE}/api/sessions`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                config: {
                    model: 'GrnModel',
                    agents_enabled: false,
                    stream_responses: true
                }
            })
        });

        if (!response.ok) throw new Error('Failed to create session');

        const data = await response.json();
        window.location.href = `/session/${data.session_id}`;
    } catch (error) {
        showError(error.message);
    } finally {
        showLoading(false);
    }
}

function joinSession(sessionId) {
    window.location.href = `/session/${sessionId}`;
}

function formatTimestamp(timestamp) {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now - date;
    const diffMins = Math.floor(diffMs / 60000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffMins < 1440) return `${Math.floor(diffMins / 60)}h ago`;
    return date.toLocaleDateString();
}

function showLoading(show) {
    document.getElementById('loading').classList.toggle('hidden', !show);
    document.getElementById('sessionList').classList.toggle('hidden', show);
}

function showError(message) {
    document.getElementById('error').classList.remove('hidden');
    document.getElementById('errorMessage').textContent = message;
}

function hideError() {
    document.getElementById('error').classList.add('hidden');
}
```

### js/websocket.js (WebSocket Client)

```javascript
// WebSocket client for real-time communication

export class ChatWebSocket {
    constructor(sessionId) {
        this.sessionId = sessionId;
        this.ws = null;
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 5;
        this.handlers = {
            message: [],
            connected: [],
            disconnected: [],
            error: []
        };
    }

    connect() {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/ws/${this.sessionId}`;

        this.ws = new WebSocket(wsUrl);

        this.ws.onopen = () => {
            console.log('WebSocket connected');
            this.reconnectAttempts = 0;
            this.emit('connected');
        };

        this.ws.onmessage = (event) => {
            try {
                const message = JSON.parse(event.data);
                this.emit('message', message);
            } catch (error) {
                console.error('Failed to parse message:', error);
            }
        };

        this.ws.onerror = (error) => {
            console.error('WebSocket error:', error);
            this.emit('error', error);
        };

        this.ws.onclose = (event) => {
            console.log('WebSocket closed:', event.code, event.reason);
            this.emit('disconnected');

            if (!event.wasClean && this.reconnectAttempts < this.maxReconnectAttempts) {
                this.reconnectAttempts++;
                const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
                console.log(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);
                setTimeout(() => this.connect(), delay);
            }
        };
    }

    send(type, data) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify({ type, data }));
        } else {
            console.error('WebSocket not connected');
            throw new Error('WebSocket not connected');
        }
    }

    on(event, handler) {
        if (this.handlers[event]) {
            this.handlers[event].push(handler);
        }
    }

    emit(event, data) {
        if (this.handlers[event]) {
            this.handlers[event].forEach(handler => handler(data));
        }
    }

    close() {
        if (this.ws) {
            this.ws.close();
        }
    }
}
```

### js/chat.js (Chat Interface Logic)

```javascript
// Chat interface logic

import { ChatWebSocket } from './websocket.js';

let ws;
let sessionId;
let currentToolCall = null;
let messageBuffer = '';
let isStreaming = false;

document.addEventListener('DOMContentLoaded', () => {
    // Extract session ID from URL
    const pathParts = window.location.pathname.split('/');
    sessionId = pathParts[pathParts.length - 1];

    // Initialize WebSocket
    ws = new ChatWebSocket(sessionId);
    ws.on('message', handleServerMessage);
    ws.on('connected', onConnected);
    ws.on('disconnected', onDisconnected);
    ws.on('error', onError);
    ws.connect();

    // Setup UI handlers
    setupUIHandlers();
});

function setupUIHandlers() {
    const input = document.getElementById('messageInput');
    const sendBtn = document.getElementById('sendBtn');

    // Send message on button click
    sendBtn.addEventListener('click', sendMessage);

    // Send message on Enter (Shift+Enter for new line)
    input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            sendMessage();
        }
    });

    // Auto-resize textarea
    input.addEventListener('input', () => {
        input.style.height = 'auto';
        input.style.height = input.scrollHeight + 'px';
    });

    // Menu button
    document.getElementById('menuBtn').addEventListener('click', toggleMenu);

    // Tool confirmation buttons
    document.getElementById('toolConfirmBtn').addEventListener('click', () => confirmTool(true));
    document.getElementById('toolDenyBtn').addEventListener('click', () => confirmTool(false));
}

function onConnected() {
    console.log('Connected to session:', sessionId);
    document.getElementById('sessionInfo').textContent = `Session: ${sessionId.substring(0, 8)}`;
    setInputEnabled(true);
}

function onDisconnected() {
    console.log('Disconnected from session');
    setInputEnabled(false);
    showInputStatus('Disconnected. Reconnecting...', 'text-yellow-400');
}

function onError(error) {
    console.error('WebSocket error:', error);
    showInputStatus('Connection error', 'text-red-400');
}

function handleServerMessage(message) {
    console.log('Received:', message.type, message.data);

    switch (message.type) {
        case 'SessionJoined':
            handleSessionJoined(message.data);
            break;
        case 'AssistantMessageChunk':
            handleMessageChunk(message.data);
            break;
        case 'AssistantMessage':
            handleAssistantMessage(message.data);
            break;
        case 'AssistantMessageComplete':
            handleMessageComplete();
            break;
        case 'ToolCallRequest':
            handleToolCallRequest(message.data);
            break;
        case 'ToolCallResult':
            handleToolCallResult(message.data);
            break;
        case 'TaskProgress':
            handleTaskProgress(message.data);
            break;
        case 'AgentAssigned':
            handleAgentAssigned(message.data);
            break;
        case 'ModelSwitched':
            handleModelSwitched(message.data);
            break;
        case 'TokenUsage':
            handleTokenUsage(message.data);
            break;
        case 'Error':
            handleError(message.data);
            break;
    }
}

function handleSessionJoined(data) {
    // Display conversation history
    data.history.forEach(msg => {
        if (msg.role !== 'system') {
            appendMessage(msg.role, msg.content);
        }
    });

    // Update model badge
    document.getElementById('modelBadge').textContent = data.current_model;

    scrollToBottom();
}

function handleMessageChunk(data) {
    if (!isStreaming) {
        isStreaming = true;
        messageBuffer = '';
        createStreamingMessage();
    }

    messageBuffer += data.chunk;
    updateStreamingMessage(messageBuffer);
}

function handleAssistantMessage(data) {
    appendMessage('assistant', data.content);
    scrollToBottom();
}

function handleMessageComplete() {
    if (isStreaming) {
        finalizeStreamingMessage();
        isStreaming = false;
        messageBuffer = '';
    }
    setInputEnabled(true);
    hideInputStatus();
}

function handleToolCallRequest(data) {
    if (data.requires_confirmation) {
        currentToolCall = data;
        showToolConfirmation(data);
    } else {
        // Auto-execute, just show notification
        appendToolNotification(data.name, 'executing');
    }
}

function handleToolCallResult(data) {
    appendToolResult(data.result, data.success);
}

function handleTaskProgress(data) {
    showAgentProgress(data);
}

function handleAgentAssigned(data) {
    showAgentProgress({
        agent_name: data.agent_name,
        status: 'TaskExecution',
        progress: 0,
        description: data.task_description
    });
}

function handleModelSwitched(data) {
    document.getElementById('modelBadge').textContent = data.new_model;
    appendSystemMessage(`Model switched to ${data.new_model}`);
}

function handleTokenUsage(data) {
    // Could show in UI footer or status bar
    console.log('Token usage:', data);
}

function handleError(data) {
    appendErrorMessage(data.message);
    if (!data.recoverable) {
        setInputEnabled(false);
    }
}

function sendMessage() {
    const input = document.getElementById('messageInput');
    const content = input.value.trim();

    if (!content) return;

    // Add user message to UI
    appendMessage('user', content);

    // Send to server
    ws.send('SendMessage', { content });

    // Clear input
    input.value = '';
    input.style.height = 'auto';

    // Disable input while processing
    setInputEnabled(false);
    showInputStatus('Sending...', 'text-gray-400');

    scrollToBottom();
}

function appendMessage(role, content) {
    const container = document.getElementById('chatContainer');
    const messageDiv = document.createElement('div');
    messageDiv.className = `message message-${role}`;

    if (role === 'user') {
        messageDiv.innerHTML = `
            <div class="flex justify-end">
                <div class="bg-blue-600 rounded-lg px-4 py-2 max-w-[80%]">
                    <p class="text-sm whitespace-pre-wrap">${escapeHtml(content)}</p>
                </div>
            </div>
        `;
    } else {
        messageDiv.innerHTML = `
            <div class="flex justify-start">
                <div class="bg-gray-700 rounded-lg px-4 py-2 max-w-[80%]">
                    <div class="prose prose-invert prose-sm max-w-none">
                        ${renderMarkdown(content)}
                    </div>
                </div>
            </div>
        `;
    }

    container.appendChild(messageDiv);
}

function createStreamingMessage() {
    const container = document.getElementById('chatContainer');
    const messageDiv = document.createElement('div');
    messageDiv.id = 'streamingMessage';
    messageDiv.className = 'message message-assistant';
    messageDiv.innerHTML = `
        <div class="flex justify-start">
            <div class="bg-gray-700 rounded-lg px-4 py-2 max-w-[80%]">
                <div class="prose prose-invert prose-sm max-w-none" id="streamingContent"></div>
                <span class="inline-block w-2 h-4 bg-white animate-pulse ml-1"></span>
            </div>
        </div>
    `;
    container.appendChild(messageDiv);
}

function updateStreamingMessage(content) {
    const contentDiv = document.getElementById('streamingContent');
    if (contentDiv) {
        contentDiv.innerHTML = renderMarkdown(content);
        scrollToBottom();
    }
}

function finalizeStreamingMessage() {
    const messageDiv = document.getElementById('streamingMessage');
    if (messageDiv) {
        messageDiv.id = '';
        const cursor = messageDiv.querySelector('.animate-pulse');
        if (cursor) cursor.remove();
    }
}

function showToolConfirmation(data) {
    currentToolCall = data;

    document.getElementById('toolName').textContent = data.name;
    document.getElementById('toolArguments').textContent = JSON.stringify(data.arguments, null, 2);

    if (data.diff) {
        document.getElementById('toolDiff').classList.remove('hidden');
        document.getElementById('toolDiff').querySelector('pre').textContent = data.diff;
    } else {
        document.getElementById('toolDiff').classList.add('hidden');
    }

    document.getElementById('toolConfirmModal').classList.remove('hidden');
}

function confirmTool(confirmed) {
    if (currentToolCall) {
        ws.send('ConfirmTool', {
            tool_call_id: currentToolCall.tool_call_id,
            confirmed
        });
    }

    document.getElementById('toolConfirmModal').classList.add('hidden');
    currentToolCall = null;
}

function showAgentProgress(data) {
    const progressDiv = document.getElementById('agentProgress');
    progressDiv.classList.remove('hidden');

    document.getElementById('agentName').textContent = data.agent_name;
    document.getElementById('agentTask').textContent = data.description;

    const progressBar = document.getElementById('agentProgressBar').querySelector('div');
    progressBar.style.width = `${data.progress * 100}%`;

    if (data.status === 'Completed') {
        setTimeout(() => progressDiv.classList.add('hidden'), 2000);
    }
}

function setInputEnabled(enabled) {
    document.getElementById('messageInput').disabled = !enabled;
    document.getElementById('sendBtn').disabled = !enabled;
}

function showInputStatus(message, className) {
    const status = document.getElementById('inputStatus');
    status.textContent = message;
    status.className = `mt-2 text-xs ${className}`;
    status.classList.remove('hidden');
}

function hideInputStatus() {
    document.getElementById('inputStatus').classList.add('hidden');
}

function scrollToBottom() {
    const container = document.getElementById('chatContainer');
    container.scrollTop = container.scrollHeight;
}

// Utility functions
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function renderMarkdown(text) {
    // Simple markdown rendering (use marked.js or similar library in production)
    return escapeHtml(text)
        .replace(/`([^`]+)`/g, '<code>$1</code>')
        .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
        .replace(/\n/g, '<br>');
}

function toggleMenu() {
    document.getElementById('menuDropdown').classList.toggle('hidden');
}

// Close menu when clicking outside
document.addEventListener('click', (e) => {
    const menu = document.getElementById('menuDropdown');
    const menuBtn = document.getElementById('menuBtn');
    if (!menu.contains(e.target) && !menuBtn.contains(e.target)) {
        menu.classList.add('hidden');
    }
});
```

### js/mobile.js (Mobile-Specific Handling)

```javascript
// Mobile-specific handling

const isMobile = /iPhone|iPad|iPod|Android/i.test(navigator.userAgent);
const isIOS = /iPhone|iPad|iPod/i.test(navigator.userAgent);

if (isMobile) {
    console.log('Mobile device detected');

    // Prevent zoom on input focus (iOS)
    if (isIOS) {
        const viewportMeta = document.querySelector('meta[name="viewport"]');
        if (viewportMeta) {
            viewportMeta.content = 'width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no, viewport-fit=cover';
        }
    }

    // Handle safe area insets (notch/home indicator)
    document.documentElement.style.setProperty('--safe-area-inset-top', 'env(safe-area-inset-top)');
    document.documentElement.style.setProperty('--safe-area-inset-bottom', 'env(safe-area-inset-bottom)');

    // Adjust input area for keyboard
    const input = document.getElementById('messageInput');
    if (input) {
        input.addEventListener('focus', () => {
            // Scroll input into view when keyboard appears
            setTimeout(() => {
                input.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
            }, 300);
        });
    }

    // Add touch swipe gestures for session switching (future enhancement)
    let touchStartX = 0;
    let touchEndX = 0;

    document.addEventListener('touchstart', (e) => {
        touchStartX = e.changedTouches[0].screenX;
    });

    document.addEventListener('touchend', (e) => {
        touchEndX = e.changedTouches[0].screenX;
        handleSwipe();
    });

    function handleSwipe() {
        const threshold = 100;
        const diff = touchEndX - touchStartX;

        if (diff > threshold) {
            // Swipe right - go back to session list
            window.location.href = '/';
        }
    }

    // Prevent pull-to-refresh on chat container
    const chatContainer = document.getElementById('chatContainer');
    if (chatContainer) {
        chatContainer.addEventListener('touchmove', (e) => {
            if (chatContainer.scrollTop === 0) {
                e.preventDefault();
            }
        }, { passive: false });
    }
}

// PWA install prompt
let deferredPrompt;

window.addEventListener('beforeinstallprompt', (e) => {
    e.preventDefault();
    deferredPrompt = e;

    // Show custom install button (optional)
    showInstallPrompt();
});

function showInstallPrompt() {
    // Could show a banner or button to install the app
    console.log('PWA can be installed');
}

// Handle online/offline status
window.addEventListener('online', () => {
    console.log('Back online');
    // Could show notification to user
});

window.addEventListener('offline', () => {
    console.log('Offline');
    // Show offline indicator
    document.getElementById('inputStatus').textContent = 'Offline - check your connection';
    document.getElementById('inputStatus').className = 'mt-2 text-xs text-red-400';
    document.getElementById('inputStatus').classList.remove('hidden');
});
```

## PWA Manifest

### web/manifest.json

```json
{
  "name": "APChat - Multi-Agent AI CLI",
  "short_name": "APChat",
  "description": "Multi-agent AI CLI with web interface supporting both TUI and web sessions",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#111827",
  "theme_color": "#111827",
  "orientation": "portrait-primary",
  "icons": [
    {
      "src": "/icons/icon-72.png",
      "sizes": "72x72",
      "type": "image/png",
      "purpose": "any maskable"
    },
    {
      "src": "/icons/icon-96.png",
      "sizes": "96x96",
      "type": "image/png",
      "purpose": "any maskable"
    },
    {
      "src": "/icons/icon-128.png",
      "sizes": "128x128",
      "type": "image/png",
      "purpose": "any maskable"
    },
    {
      "src": "/icons/icon-144.png",
      "sizes": "144x144",
      "type": "image/png",
      "purpose": "any maskable"
    },
    {
      "src": "/icons/icon-152.png",
      "sizes": "152x152",
      "type": "image/png",
      "purpose": "any maskable"
    },
    {
      "src": "/icons/icon-192.png",
      "sizes": "192x192",
      "type": "image/png",
      "purpose": "any maskable"
    },
    {
      "src": "/icons/icon-384.png",
      "sizes": "384x384",
      "type": "image/png",
      "purpose": "any maskable"
    },
    {
      "src": "/icons/icon-512.png",
      "sizes": "512x512",
      "type": "image/png",
      "purpose": "any maskable"
    }
  ],
  "categories": ["productivity", "developer tools"],
  "screenshots": [
    {
      "src": "/screenshots/desktop.png",
      "sizes": "1280x720",
      "type": "image/png",
      "platform": "wide"
    },
    {
      "src": "/screenshots/mobile.png",
      "sizes": "750x1334",
      "type": "image/png",
      "platform": "narrow"
    }
  ]
}
```

## Tailwind CSS Configuration

### tailwind.config.js

```javascript
module.exports = {
  content: [
    './web/**/*.html',
    './web/**/*.js',
  ],
  theme: {
    extend: {
      colors: {
        gray: {
          750: '#374151',
          850: '#1F2937',
          950: '#0F172A',
        }
      },
      spacing: {
        'safe-top': 'var(--safe-area-inset-top)',
        'safe-bottom': 'var(--safe-area-inset-bottom)',
      }
    },
  },
  plugins: [
    require('@tailwindcss/typography'),
  ],
}
```

## Mobile-Specific CSS

### web/styles/mobile.css

```css
/* Mobile-specific styles */

/* Safe area handling for iOS notch */
@supports (padding: env(safe-area-inset-top)) {
  body {
    padding-top: env(safe-area-inset-top);
    padding-bottom: env(safe-area-inset-bottom);
  }
}

/* Touch-friendly tap targets */
@media (max-width: 640px) {
  button {
    min-height: 44px;
    min-width: 44px;
  }

  a {
    min-height: 44px;
    display: inline-flex;
    align-items: center;
  }

  /* Larger text for readability */
  body {
    font-size: 16px;
  }

  /* Full-width session cards */
  .session-card {
    width: 100%;
  }

  /* Bottom navigation for mobile */
  .mobile-nav {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    background: var(--bg-gray-800);
    border-top: 1px solid var(--border-gray-700);
    padding-bottom: env(safe-area-inset-bottom);
  }

  /* Adjust chat container for mobile keyboard */
  #chatContainer {
    padding-bottom: calc(env(safe-area-inset-bottom) + 80px);
  }

  /* Tool confirmation as bottom sheet on mobile */
  #toolConfirmModal .bg-gray-800 {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    border-radius: 16px 16px 0 0;
    max-height: 80vh;
  }
}

/* Landscape mode adjustments */
@media (max-width: 896px) and (orientation: landscape) {
  #chatContainer {
    max-height: calc(100vh - 200px);
  }
}

/* Prevent text selection on tap (better mobile UX) */
.no-select {
  -webkit-user-select: none;
  -moz-user-select: none;
  -ms-user-select: none;
  user-select: none;
}

/* Smooth scrolling */
html {
  scroll-behavior: smooth;
}

/* Hide scrollbar on mobile for cleaner look */
@media (max-width: 640px) {
  *::-webkit-scrollbar {
    display: none;
  }

  * {
    -ms-overflow-style: none;
    scrollbar-width: none;
  }
}
```

These examples provide a solid foundation for implementing the web frontend with mobile support!
