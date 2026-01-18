# MSPC Implementation Analysis - 2026-01-18

## Executive Summary

The MSPC (Multi-Source Input) architecture plan is well-designed but requires careful ticket creation for a simpler model to implement. Based on my analysis of:
- Existing code in `apchat-main/src/mspc/`, `apchat-main/src/input_router/`
- Implementation plan in `docs/plans/2026-01-18-mspc-multi-source-input.md`
- Existing issues 101-109

## Current State Analysis

### ✅ Already Implemented

1. **MSPC Channel**: `apchat-main/src/mspc/channel.rs`
   - Basic channel implementation with send/recv
   - Message history tracking
   - Interruption handling
   - Message parsing ("!", "/", "confirm:")

2. **TerminalInputRouter**: `apchat-main/src/input_router/terminal.rs`
   - Complete implementation
   - parse_input() with "!"/"/" detection
   - send_to_channel() method
   - handle_confirmation_prompt() method

3. **WebexInputRouter**: `apchat-main/src/input_router/webex.rs` (stub)
   - Trait implementation exists
   - Basic structure in place

4. **Input Router Module**: `apchat-main/src/input_router/mod.rs`
   - InputRouter trait defined
   - Terminal and Webex implementations

5. **Tests**: Multiple test files exist
   - `test_mspc_repl.rs`
   - `test_mspc_repl_integration.rs`
   - `test_mspc_comprehensive.rs`

### ❌ Missing Components

1. **Sender Field in MSPCMessage enum** - CRITICAL for multi-source tracking
2. **InputSourceManager** - Missing manager struct
3. **OutputDestination trait** - Missing output abstraction
4. **MSPC Chat Loop** - Main processing loop not integrated
5. **CancellationToken integration** - For interrupt handling
6. **TerminalOutputDestination** - For Phase 1 output broadcasting

## Existing Issues Analysis (101-109)

### Issues That Need Refactoring:

1. **Issue 104 & 105**: Both address MSPCMessage enum but are redundant
   - Solution: Keep Issue 105 (better title), merge sender field work

2. **Issue 101 & 106**: Both address REPL integration but are overlapping
   - Solution: Keep Issue 106 (more specific), make 101 about initialization

3. **Issue 102**: Too broad (WebSocket + Webex)
   - Solution: Split into Webex (Phase 2) and WebSocket (Phase 3)

4. **Issue 103**: Too complex for single ticket
   - Solution: Split into history validation and interruption handling

### Issues That Are Good:

5. **Issue 107**: InputSourceManager - Well defined
6. **Issue 108**: OutputDestination trait - Clear and focused
7. **Issue 109**: Terminal reader task - Specific enough

## Recommended New Issues

### Phase 1: Core MSPC (Critical)

**110**: Update MSPCMessage with sender fields
- Update enum variants to include sender: String
- Add helper methods (is_interrupt(), sender())
- Update TerminalInputRouter to use new enum

**111**: Implement TerminalOutputDestination
- Create trait implementation for terminal output
- Support colored output for different message types
- Implement broadcast_to_all() helper

**112**: Implement InputSourceManager
- Create manager struct with task tracking
- Implement spawn_terminal_reader() method
- Implement cleanup() for graceful shutdown

**113**: Implement MSPC Chat Loop
- Create main processing loop (run_mspc_chat)
- Handle interruption with queue clearing
- Broadcast all messages to output destinations

**114**: Integrate CancellationToken
- Add token to MSPCChatLoop
- Cancel LLM operations on interrupt
- Ensure proper cleanup

**115**: Update REPL with MSPC
- Modify start_repl() to use MSPC channel
- Spawn InputSourceManager
- Connect output destinations
- Test backward compatibility

### Phase 2: Webex Integration

**116**: Implement WebexInputSource
- Spawn webex polling task
- Tag messages with user IDs
- Handle webex-specific message parsing

**117**: Implement WebexOutputDestination
- Send messages to webex API
- Format for webex markdown
- Handle API errors gracefully

### Phase 3: WebSocket Integration

**118**: Implement WebSocketInputSource
- Accept websocket connections
- Tag messages with connection IDs
- Handle disconnections

**119**: Implement WebSocketOutputDestination
- Send JSON to web clients
- Support message formatting
- Handle reconnection logic

### Phase 4: Enhancements

**120**: Implement TUI OutputDestination
- Split-screen terminal UI
- Scrolling output region
- Fixed input line
- Drop-in replacement for terminal output

## Implementation Strategy for Simpler Model

### Ticket Design Principles:

1. **Small Scope**: Each ticket should be 1-3 hours of work
2. **Isolated Changes**: Minimize file modifications per ticket
3. **Clear Dependencies**: Define which tickets must come first
4. **Test-First**: Provide test cases in each ticket
5. **Backward Compatible**: Preserve existing behavior when possible

### Recommended Order:

**Phase 1 (Must Complete Before Phase 2)**:
1. Issue 110 (MSPCMessage with sender) - Foundation
2. Issue 112 (InputSourceManager) - Structure
3. Issue 111 (TerminalOutputDestination) - Output abstraction
4. Issue 113 (MSPC Chat Loop) - Core processing
5. Issue 114 (CancellationToken) - Interrupt handling
6. Issue 115 (REPL Integration) - Main loop update

**Phase 2 (Webex Support)**:
7. Issue 116 (WebexInputSource)
8. Issue 117 (WebexOutputDestination)

**Phase 3 (WebSocket)**:
9. Issue 118 (WebSocketInputSource)
10. Issue 119 (WebSocketOutputDestination)

**Phase 4 (Optional)**:
11. Issue 120 (TUI OutputDestination)

## Files That Need Modification

### Phase 1:
- `apchat-main/src/mspc/channel.rs` - Message enum, chat loop
- `apchat-main/src/mspc/mod.rs` - Module exports
- `apchat-main/src/input_router/terminal.rs` - Use sender field
- `apchat-main/src/input_router/mod.rs` - Add InputSourceManager
- `apchat-main/src/input_router/manager.rs` (NEW)
- `apchat-main/src/mspc/output.rs` (NEW) - OutputDestination trait
- `apchat-main/src/chat/mspc_chat.rs` (NEW) - MSPC chat loop
- `apchat-main/src/app/repl.rs` - Update start_repl()

### Phase 2:
- `apchat-main/src/input_router/webex.rs` - Webex reader
- `apchat-main/src/mspc/output.rs` - Webex output implementation

### Phase 3:
- `apchat-main/src/web/ws.rs` (NEW) - WebSocket handlers
- `apchat-main/src/mspc/output.rs` - WebSocket output

## Risk Assessment

### High Risk:
1. **Issue 110** (MSPCMessage changes): Breaking change affecting all code
   - Mitigation: Coordinate with all affected tickets

2. **Issue 113** (MSPC Chat Loop): Core processing logic
   - Mitigation: Start with simple implementation, iterate

### Medium Risk:
1. **Issue 114** (CancellationToken): May affect LLM stability
   - Mitigation: Test thoroughly with various LLM operations

### Low Risk:
1. **Issue 111** (TerminalOutputDestination): New abstraction
2. **Issue 112** (InputSourceManager): Manager pattern

## Testing Strategy

### Unit Tests:
- Each component should have unit tests
- Test message parsing with sender fields
- Test interruption handling
- Test output destination formatting

### Integration Tests:
- Test terminal + MSPC channel
- Test interruption clearing queue
- Test message broadcast to multiple outputs
- Test webex + terminal collaboration

### End-to-End Tests:
- Manual testing of REPL with "!" interrupt
- Manual testing of multiple simultaneous inputs
- Manual testing of output formatting

## Recommendation

Create issues 110-115 for Phase 1, focusing on:
1. Updating MSPCMessage enum with sender fields (Issue 110)
2. Implementing InputSourceManager (Issue 112)
3. Implementing TerminalOutputDestination (Issue 111)
4. Implementing MSPC Chat Loop (Issue 113)
5. Integrating CancellationToken (Issue 114)
6. Updating REPL integration (Issue 115)

These 6 tickets provide a complete Phase 1 implementation that:
- Maintains backward compatibility
- Enables multi-source input architecture
- Provides foundation for Phase 2/3
- Can be tested independently
