# Migration Plan

This document provides a step-by-step migration strategy for converting the KimiChat CLI application from a single-crate structure to a multi-crate workspace.

## Migration Strategy Overview

The migration will be executed in 7 phases to minimize disruption and ensure each step can be validated independently. Each phase includes clear validation criteria to ensure success before proceeding to the next phase.

## Phase 1: Create Workspace Structure and Core Crate

### Objectives
- Establish the workspace structure
- Extract core functionality into `kimichat-core`
- Set up workspace configuration

### Steps

#### 1.1 Create Workspace Root Configuration
1. Move existing `Cargo.toml` to `kimichat-cli/Cargo.toml`
2. Create new root `Cargo.toml` with workspace configuration
3. Add all member crates to workspace
4. Configure shared dependencies and features

#### 1.2 Create kimichat-core Crate
1. Create `kimichat-core/` directory structure
2. Move core modules:
   - `src/core/` → `kimichat-core/src/core/`
   - `src/policy.rs` → `kimichat-core/src/policy.rs`
   - `src/config/` → `kimichat-core/src/config/`
   - `src/tools_execution/validation.rs` → `kimichat-core/src/validation/`
   - `src/models/` → `kimichat-core/src/models/`

#### 1.3 Update Core Dependencies
1. Create `kimichat-core/Cargo.toml` with minimal dependencies
2. Add necessary workspace dependencies
3. Configure core-specific features

#### 1.4 Update CLI Crate
1. Modify `kimichat-cli/Cargo.toml` to depend on `kimichat-core`
2. Update imports in `kimichat-cli/src/main.rs`
3. Fix compilation errors

### Validation Criteria
- [ ] Workspace compiles successfully (`cargo build --workspace`)
- [ ] Core crate compiles independently
- [ ] CLI application runs without errors
- [ ] All existing tests pass
- [ ] No circular dependencies

### Files to Create/Modify
```
Cargo.toml (new workspace root)
kimichat-core/Cargo.toml (new)
kimichat-core/src/lib.rs (new)
kimichat-cli/Cargo.toml (modified)
kimichat-cli/src/main.rs (modified)
```

## Phase 2: Extract Terminal Functionality

### Objectives
- Extract terminal operations into `kimichat-terminal`
- Ensure terminal functionality remains intact
- Establish clean API boundaries

### Steps

#### 2.1 Create kimichat-terminal Crate
1. Create `kimichat-terminal/` directory structure
2. Move terminal modules:
   - `src/terminal/` → `kimichat-terminal/src/`
   - Update all module paths and imports

#### 2.2 Configure Terminal Dependencies
1. Create `kimichat-terminal/Cargo.toml`
2. Add dependency on `kimichat-core`
3. Add terminal-specific dependencies

#### 2.3 Update CLI Dependencies
1. Add `kimichat-terminal` dependency to CLI crate
2. Update imports to use terminal crate
3. Fix compilation errors

#### 2.4 Extract Terminal Tools
1. Move terminal-specific tools from `src/tools/` to `kimichat-terminal/src/tools/`
2. Update tool registry to import from terminal crate
3. Ensure tool functionality is preserved

### Validation Criteria
- [ ] Terminal crate compiles independently
- [ ] All terminal functionality works (PTY sessions, screen capture, etc.)
- [ ] Terminal tools are properly registered and functional
- [ ] Workspace compiles successfully
- [ ] Terminal-related tests pass

### Files to Create/Modify
```
kimichat-terminal/Cargo.toml (new)
kimichat-terminal/src/lib.rs (new)
kimichat-cli/Cargo.toml (modified)
kimichat-cli/src/main.rs (modified)
```

## Phase 3: Extract Agent System

### Objectives
- Extract agent functionality into `kimichat-agents`
- Ensure multi-agent system works correctly
- Maintain LLM client functionality

### Steps

#### 3.1 Create kimichat-agents Crate
1. Create `kimichat-agents/` directory structure
2. Move agent modules:
   - `src/agents/` → `kimichat-agents/src/`
   - Update all module paths and imports

#### 3.2 Configure Agent Dependencies
1. Create `kimichat-agents/Cargo.toml`
2. Add dependencies on `kimichat-core` and `kimichat-tools`
3. Add agent-specific dependencies (reqwest, futures, etc.)

#### 3.3 Update Dependencies
1. Add `kimichat-agents` dependency to CLI and web crates
2. Update imports to use agents crate
3. Fix compilation errors

#### 3.4 Verify Agent Functionality
1. Test single-agent mode
2. Test multi-agent mode with `--agents` flag
3. Verify model switching works correctly
4. Test all LLM client implementations

### Validation Criteria
- [ ] Agents crate compiles independently
- [ ] Single-agent mode works correctly
- [ ] Multi-agent mode works correctly
- [ ] All LLM clients (Groq, Anthropic, llama.cpp) function
- [ ] Model switching logic works
- [ ] Agent-related tests pass

### Files to Create/Modify
```
kimichat-agents/Cargo.toml (new)
kimichat-agents/src/lib.rs (new)
kimichat-cli/Cargo.toml (modified)
kimichat-cli/src/main.rs (modified)
```

## Phase 4: Extract Web Components

### Objectives
- Extract web functionality into `kimichat-web`
- Ensure web interface remains functional
- Test WebSocket communication

### Steps

#### 4.1 Create kimichat-web Crate
1. Create `kimichat-web/` directory structure
2. Move web modules:
   - `src/web/` → `kimichat-web/src/`
   - Update all module paths and imports

#### 4.2 Configure Web Dependencies
1. Create `kimichat-web/Cargo.toml`
2. Add dependencies on `kimichat-core`, `kimichat-terminal`, `kimichat-agents`
3. Add web-specific dependencies (axum, tower, etc.)

#### 4.3 Update Dependencies
1. Add `kimichat-web` dependency to CLI crate
2. Update imports to use web crate
3. Fix compilation errors

#### 4.4 Test Web Functionality
1. Start web server with `--web` flag
2. Test WebSocket connections
3. Verify real-time communication
4. Test session management

### Validation Criteria
- [ ] Web crate compiles independently
- [ ] Web server starts without errors
- [ ] WebSocket connections work correctly
- [ ] Real-time communication functions
- [ ] Web interface displays properly
- [ ] Web-related tests pass

### Files to Create/Modify
```
kimichat-web/Cargo.toml (new)
kimichat-web/src/lib.rs (new)
kimichat-cli/Cargo.toml (modified)
kimichat-cli/src/main.rs (modified)
```

## Phase 5: Extract Tools

### Objectives
- Extract tool implementations into `kimichat-tools`
- Ensure all tools work correctly
- Maintain tool registration and discovery

### Steps

#### 5.1 Create kimichat-tools Crate
1. Create `kimichat-tools/` directory structure
2. Move tool modules:
   - `src/tools/` → `kimichat-tools/src/`
   - `src/skills/embedded.rs` → `kimichat-tools/src/skills/`
   - `src/todo.rs` → `kimichat-tools/src/todo.rs`
   - Update all module paths and imports

#### 5.2 Configure Tools Dependencies
1. Create `kimichat-tools/Cargo.toml`
2. Add dependencies on `kimichat-core` and `kimichat-terminal`
3. Add tool-specific dependencies (glob, ignore, regex, etc.)

#### 5.3 Update Dependencies
1. Add `kimichat-tools` dependency to CLI and agents crates
2. Update imports to use tools crate
3. Fix compilation errors

#### 5.4 Verify Tool Functionality
1. Test all file operation tools
2. Test search functionality
3. Test system tools
4. Test todo management
5. Test skill discovery and execution

### Validation Criteria
- [ ] Tools crate compiles independently
- [ ] All file operation tools work correctly
- [ ] Search functionality works
- [ ] System tools function properly
- [ ] Todo management works
- [ ] Skill discovery and execution work
- [ ] Tool-related tests pass

### Files to Create/Modify
```
kimichat-tools/Cargo.toml (new)
kimichat-tools/src/lib.rs (new)
kimichat-cli/Cargo.toml (modified)
kimichat-agents/Cargo.toml (modified)
kimichat-cli/src/main.rs (modified)
```

## Phase 6: Reorganize CLI Application

### Objectives
- Clean up CLI crate structure
- Ensure all functionality works through workspace
- Optimize CLI-specific code

### Steps

#### 6.1 Restructure CLI Crate
1. Organize remaining CLI-specific modules:
   - `src/cli.rs` (command parsing)
   - `src/app/` (application modes)
   - `src/chat/` (chat functionality)
   - `src/logging/` (logging setup)
   - `src/main.rs` (entry point)

#### 6.2 Update CLI Dependencies
1. Remove direct dependencies now provided by workspace crates
2. Update `Cargo.toml` with only CLI-specific dependencies
3. Configure CLI-specific features

#### 6.3 Clean Up Imports
1. Update all imports to use workspace crates
2. Remove any remaining references to old module structure
3. Ensure consistent import style

#### 6.4 Optimize CLI Code
1. Remove any duplicate functionality now in other crates
2. Streamline CLI-specific logic
3. Improve error handling and user experience

### Validation Criteria
- [ ] CLI crate has minimal, focused dependencies
- [ ] All CLI commands work correctly
- [ ] REPL functions properly
- [ ] Help system works
- [ ] Configuration loading works
- [ ] CLI-related tests pass

### Files to Create/Modify
```
kimichat-cli/Cargo.toml (cleaned up)
kimichat-cli/src/main.rs (optimized)
kimichat-cli/src/app/mod.rs (cleaned up)
```

## Phase 7: Testing and Validation

### Objectives
- Comprehensive testing of the migrated workspace
- Performance benchmarking
- Documentation updates

### Steps

#### 7.1 Comprehensive Testing
1. Run full test suite (`cargo test --workspace`)
2. Test all application modes:
   - CLI REPL mode
   - Single-agent mode
   - Multi-agent mode
   - Web interface mode
3. Test all functionality areas:
   - File operations
   - Terminal sessions
   - Web communication
   - Tool execution
   - Model switching

#### 7.2 Performance Benchmarking
1. Measure build times before and after migration
2. Test compilation performance with various change scenarios
3. Verify runtime performance is maintained
4. Document performance improvements

#### 7.3 Integration Testing
1. Test with real LLM APIs
2. Test with actual file operations
3. Test web interface with real browsers
4. Test terminal sessions with actual PTY operations

#### 7.4 Documentation Updates
1. Update README.md with new build instructions
2. Update contributing guidelines
3. Document workspace structure for developers
4. Create migration guide for other projects

### Validation Criteria
- [ ] All tests pass across workspace
- [ ] All application modes work correctly
- [ ] Performance meets or exceeds expectations
- [ ] Documentation is comprehensive and accurate
- [ ] No functionality regressions detected
- [ ] Build times are improved

### Files to Create/Modify
```
README.md (updated)
CONTRIBUTING.md (updated)
docs/ (updated as needed)
```

## Risk Mitigation

### Potential Issues and Solutions

#### 1. Circular Dependencies
**Risk:** Creating circular dependencies between crates
**Solution:** Careful dependency planning and regular dependency graph analysis

#### 2. Compilation Errors
**Risk:** Large number of compilation errors during migration
**Solution:** Incremental migration with validation at each step

#### 3. Runtime Issues
**Risk:** Runtime behavior changes due to refactoring
**Solution:** Comprehensive testing at each phase

#### 4. Performance Regression
**Risk:** Performance degradation due to abstraction layers
**Solution:** Performance benchmarking and optimization

### Rollback Strategy

Each phase maintains the ability to rollback:
1. Keep original code until phase is validated
2. Use git branches for each phase
3. Maintain documentation of changes
4. Test thoroughly before proceeding

## Success Metrics

### Build Performance
- [ ] Initial build time improved by >20%
- [ ] Incremental build time improved by >40%
- [ ] Parallel compilation working effectively

### Code Organization
- [ ] Clear separation of concerns
- [ ] Reduced coupling between modules
- [ ] Improved code discoverability

### Developer Experience
- [ ] Better IDE performance and indexing
- [ ] Easier unit testing of components
- [ ] Clearer contribution boundaries

### Maintainability
- [ ] Reduced complexity in individual crates
- [ ] Better dependency management
- [ ] Easier future refactoring

## Timeline

- **Phase 1:** 1-2 days
- **Phase 2:** 1-2 days
- **Phase 3:** 2-3 days
- **Phase 4:** 1-2 days
- **Phase 5:** 2-3 days
- **Phase 6:** 1 day
- **Phase 7:** 2-3 days

**Total Estimated Time:** 10-16 days

This migration plan provides a structured approach to transforming the KimiChat codebase into a well-organized workspace while minimizing risks and ensuring functionality is preserved throughout the process.