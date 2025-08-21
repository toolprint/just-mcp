# ultrafast-mcp Integration Tasks

## Overview

This document provides detailed task breakdown for migrating just-mcp from its custom MCP implementation to the ultrafast-mcp framework. The migration is tracked in Goaly under goal ID `23` and consists of 18 discrete tasks organized into 4 phases.

**Goal:** just-mcp: ultrafast-mcp integration  
**Goal ID:** 23  
**Total Tasks:** 18  
**Estimated Timeline:** 8 days  

## Implementation Strategy

The migration follows a phased approach that prioritizes:
1. **Reusing existing code** - Especially ToolRegistry, Prompts, Resources
2. **Maintaining functionality** - No features lost during migration
3. **Incremental progress** - Each phase leaves system functional
4. **Risk mitigation** - Feature flags and parallel implementation

The key insight is that this is primarily a **transport layer replacement**, not a complete rewrite. The business logic (parsers, security, execution) remains largely unchanged.

## Task Dependencies Overview

```
Phase 1: Foundation (171 → 172 → 173 → 174)
                                      ↓
Phase 2: Dynamic Tools (175 → 176 → 177 → 178)
                                              ↓
Phase 3: Core Migration (179 → [180, 181, 182] → 183)
                                                    ↓
Phase 4: Cleanup (184 → 185 → [186, 187] → 188)
```

## Phase 1: Foundation Setup (Tasks 171-174)

### Task 171: Research and Dependency Setup
- **Task ID:** 171
- **Owner:** ai-engineer
- **Status:** owned
- **Importance:** 1 (highest)
- **Dependencies:** None (leaf node)
- **Blocks:** Task 172 (Create Framework Server Module)

**Description:** Identify correct ultrafast-mcp crate version and dependencies. Research actual crate names/versions on crates.io, possibly use git dependency if needed.

**Implementation Details:**
- Research actual ultrafast-mcp crate availability (document mentions v202506018.1.0 which may not exist)
- Investigate git dependency options if needed
- Update Cargo.toml with correct dependencies
- Ensure compatibility with existing dependencies

**Validation Criteria:** Cargo build succeeds with new dependencies

**Risk Areas:**
- Framework version availability
- Dependency conflicts
- API compatibility

---

### Task 172: Create Framework Server Module
- **Task ID:** 172
- **Owner:** ai-engineer
- **Status:** owned (blocked)
- **Dependencies:** Task 171 (requires ultrafast-mcp dependency)
- **Blocks:** Task 173 (Basic Framework Initialization)

**Description:** Create new `src/server_v2/` module structure parallel to existing implementation. This allows development without breaking current functionality.

**Implementation Details:**
```
src/
├── server_v2/
│   ├── mod.rs           # Framework server setup
│   ├── dynamic_handler.rs  # Dynamic tool management
│   ├── resources.rs     # Resource providers
│   └── prompts.rs       # Prompt providers
```

**Validation Criteria:** New module compiles without affecting existing code

---

### Task 173: Basic Framework Initialization
- **Task ID:** 173
- **Owner:** ai-engineer
- **Status:** owned (blocked)
- **Dependencies:** Task 172 (requires server module structure)
- **Blocks:** Task 174 (Feature Flag Implementation)

**Description:** Implement minimal framework server that can start and handle basic requests. Focus on getting ultrafast-mcp server running with basic initialization.

**Implementation Details:**
- Initialize ultrafast-mcp server
- Handle basic MCP protocol messages (initialize, ping)
- Establish stdio transport
- Basic error handling

**Validation Criteria:** Framework server starts and responds to initialize request

---

### Task 174: Feature Flag Implementation
- **Task ID:** 174
- **Owner:** ai-engineer
- **Status:** owned (blocked)
- **Dependencies:** Task 173 (requires working framework initialization)
- **Blocks:** Task 175 (DynamicToolHandler Implementation)

**Description:** Add feature flag to switch between old and new implementations. This enables safe testing and rollback capability.

**Implementation Details:**
- Add cargo feature flag: `ultrafast-framework`
- Conditional compilation for server selection
- Runtime switching capability
- Maintain existing CLI interface

**Validation Criteria:** Can toggle between implementations via cargo feature

## Phase 2: Dynamic Tool System (Tasks 175-178)

### Task 175: DynamicToolHandler Implementation
- **Task ID:** 175
- **Owner:** ai-engineer
- **Status:** owned (blocked)
- **Dependencies:** Task 174 (requires feature flag system)
- **Blocks:** Task 176 (Dynamic Registration Adapter)

**Description:** Create wrapper that bridges existing ToolRegistry with framework. Reuse existing dynamic registration logic and adapt to framework patterns.

**Implementation Details:**
```rust
pub struct DynamicToolHandler {
    tools: Arc<RwLock<HashMap<String, ToolDefinition>>>,
    executor: Arc<JustExecutor>,
    framework_handle: UltrafastMcpHandle,
}
```

**Key Integration Points:**
- Wrap existing ToolRegistry
- Maintain tool naming conventions
- Preserve security wrappers

**Validation Criteria:** Can register static tools through framework

---

### Task 176: Dynamic Registration Adapter
- **Task ID:** 176
- **Owner:** ai-engineer
- **Status:** owned (blocked)
- **Dependencies:** Task 175 (requires DynamicToolHandler)
- **Blocks:** Task 177 (Watcher Integration)

**Description:** Implement dynamic tool updates via framework handle. Create adapter layer that can notify framework of tool changes from file watcher events.

**Implementation Details:**
- Tool diff calculation
- Framework notification system
- Change batching and debouncing
- Error recovery

**Validation Criteria:** Tool changes propagate to framework

---

### Task 177: Watcher Integration
- **Task ID:** 177
- **Owner:** ai-engineer
- **Status:** owned (blocked)
- **Dependencies:** Task 176 (requires dynamic registration adapter)
- **Blocks:** Task 178 (Execution Integration)

**Description:** Connect existing JustfileWatcher to DynamicToolHandler. Preserve existing debouncing and file change detection while wiring to new handler.

**Implementation Details:**
- Maintain existing JustfileWatcher
- Route change events to DynamicToolHandler
- Preserve 500ms debouncing
- Keep file hash-based change detection

**Validation Criteria:** File changes trigger tool updates in framework

---

### Task 178: Execution Integration
- **Task ID:** 178
- **Owner:** ai-engineer
- **Status:** owned (blocked)
- **Dependencies:** Task 177 (requires watcher integration)
- **Blocks:** Task 179 (Transport Layer Replacement)

**Description:** Wire TaskExecutor through framework tool calls. Maintain existing security model and resource limits. Ensure existing execution patterns work through framework.

**Implementation Details:**
- Preserve SecurityWrapper integration
- Maintain ResourceManager limits
- Keep existing error handling
- Tool result formatting

**Validation Criteria:** Tools execute with existing security/limits

## Phase 3: Core Migration (Tasks 179-183)

### Task 179: Transport Layer Replacement
- **Task ID:** 179
- **Owner:** ai-engineer
- **Status:** owned (blocked)
- **Dependencies:** Task 178 (requires execution integration)
- **Blocks:** Tasks 180, 182, 183 (Resources, Admin, Error Handling)

**Description:** Replace custom JSON-RPC handling with framework transport. Remove src/server/handler.rs and src/protocol/ modules. Route all protocol methods through framework.

**Implementation Details:**
- Remove custom JSON-RPC code (~1100 lines)
- Replace with framework transport
- Maintain API compatibility
- Protocol method routing

**Code Removal:**
- `src/server/handler.rs` (~600 lines)
- `src/protocol/` (~500 lines)

**Validation Criteria:** All protocol methods work through framework

---

### Task 180: Resources Migration
- **Task ID:** 180
- **Owner:** ai-engineer
- **Status:** owned (blocked)
- **Dependencies:** Task 179 (requires transport layer)
- **Blocks:** Task 183 (Error Handling Alignment)

**Description:** Migrate existing resources support to framework patterns. Preserve existing resource providers and adapt them to framework expectations.

**Implementation Details:**
- Adapt existing ResourceProvider implementations
- Maintain resource URIs and formats
- Framework-compatible resource registration
- Preserve metadata resources

**Validation Criteria:** Resources list/read work through framework

---

### Task 181: Prompts Migration
- **Task ID:** 181
- **Owner:** ai-engineer
- **Status:** owned (blocked)
- **Dependencies:** Task 180 (requires resources migration)
- **Blocks:** Task 183 (Error Handling Alignment)

**Description:** Migrate existing prompts (including /just:do-it) to framework patterns. Ensure prompt registry works with framework and slash command prompt is available.

**Implementation Details:**
- Preserve existing PromptRegistry
- Maintain /just:do-it slash command
- Framework-compatible prompt registration
- Natural language task execution

**Validation Criteria:** Prompts list/execute work, /just:do-it available

---

### Task 182: Admin Tools Migration
- **Task ID:** 182
- **Owner:** ai-engineer
- **Status:** owned (blocked)
- **Dependencies:** Task 179 (requires transport layer)
- **Blocks:** Task 183 (Error Handling Alignment)

**Description:** Migrate admin tools to framework patterns. Ensure existing admin functionality (parser diagnostics, sync commands, etc.) work through framework.

**Implementation Details:**
- Preserve admin tool functionality
- Framework-compatible tool registration
- Maintain diagnostic capabilities
- Parser doctor tools

**Validation Criteria:** Admin tools functional through framework

---

### Task 183: Error Handling Alignment
- **Task ID:** 183
- **Owner:** ai-engineer
- **Status:** owned (blocked)
- **Dependencies:** Tasks 179, 180, 181, 182 (requires all core migrations)
- **Blocks:** Task 184 (Remove Old Implementation)

**Description:** Ensure error handling matches framework expectations. Update error types and formats to work with framework patterns. Maintain meaningful error messages.

**Implementation Details:**
- Framework-compatible error types
- Error message preservation
- Protocol error formatting
- Validation error handling

**Validation Criteria:** Errors properly formatted and returned

## Phase 4: Cleanup & Testing (Tasks 184-188)

### Task 184: Remove Old Implementation
- **Task ID:** 184
- **Owner:** ai-engineer
- **Status:** owned (blocked)
- **Dependencies:** Task 183 (requires all migrations complete)
- **Blocks:** Task 185 (Test Migration)

**Description:** Delete old server code, protocol definitions, custom transport. Remove approximately 2000 lines of custom MCP code. Clean up unused dependencies.

**Code Removal:**
- Remaining custom MCP protocol code
- Unused dependencies
- Legacy imports and modules
- Dead code elimination

**Validation Criteria:** Codebase compiles and tests pass

---

### Task 185: Test Migration
- **Task ID:** 185
- **Owner:** ai-engineer
- **Status:** owned (blocked)
- **Dependencies:** Task 184 (requires old code removed)
- **Blocks:** Tasks 186, 187 (Integration Tests, Performance Benchmarks)

**Description:** Update all tests to work with new framework. Migrate existing test suite to test framework-based implementation instead of custom MCP code.

**Implementation Details:**
- Update unit tests for new architecture
- Fix integration tests
- Test helper adaptations
- Mock framework interactions

**Validation Criteria:** All existing tests pass

---

### Task 186: Integration Tests
- **Task ID:** 186
- **Owner:** ai-engineer
- **Status:** owned (blocked)
- **Dependencies:** Task 185 (requires test migration)

**Description:** Add new integration tests for framework-specific features. Test end-to-end functionality, Resources/Prompts integration, and dynamic tool registration.

**Implementation Details:**
- End-to-end workflow tests
- Framework-specific feature tests
- Protocol compliance validation
- Dynamic registration testing

**Validation Criteria:** New test suite covers all functionality

---

### Task 187: Performance Benchmarks
- **Task ID:** 187
- **Owner:** ai-engineer
- **Status:** owned (blocked)
- **Dependencies:** Task 185 (requires test migration)
- **Blocks:** Task 188 (Documentation Update)

**Description:** Benchmark new implementation vs old. Test startup time, tool execution latency, memory usage, and concurrent request handling.

**Benchmark Areas:**
- Startup time comparison
- Tool execution latency
- Memory usage under load
- Concurrent request handling
- Protocol message throughput

**Validation Criteria:** No significant performance regression

---

### Task 188: Documentation Update
- **Task ID:** 188
- **Owner:** ai-engineer
- **Status:** owned (blocked)
- **Dependencies:** Task 187 (requires benchmarks complete)

**Description:** Update all documentation for v0.2.0. Update README, architecture docs, API examples to reflect new framework-based implementation.

**Documentation Updates:**
- README.md with new capabilities
- Architecture documentation
- API examples and usage
- Migration guide
- Version 0.2.0 release notes

**Validation Criteria:** Docs reflect new architecture

## Critical Path Analysis

The critical path for this migration is:

```
171 → 172 → 173 → 174 → 175 → 176 → 177 → 178 → 179 → 183 → 184 → 185 → 187 → 188
```

**Total Duration:** 8 days
- **Phase 1:** 2 days (Tasks 171-174)
- **Phase 2:** 2 days (Tasks 175-178)  
- **Phase 3:** 2 days (Tasks 179-183)
- **Phase 4:** 2 days (Tasks 184-188)

**Parallel Opportunities:**
- Tasks 180, 181, 182 can run in parallel after Task 179
- Tasks 186, 187 can run in parallel after Task 185

## Risk Mitigation

**High-Risk Tasks:**
- **Task 171:** Framework availability risk
- **Task 175:** Dynamic registration complexity
- **Task 179:** Transport layer replacement scope

**Mitigation Strategies:**
- Feature flags for rollback capability
- Parallel implementation approach
- Comprehensive testing at each phase
- Performance monitoring throughout

## Success Metrics

**Phase Completion Criteria:**
1. Framework server successfully initializes and handles basic requests
2. Dynamic tool registration/deregistration working correctly
3. Complete feature parity with existing implementation
4. Resources and Prompts fully functional with /just:do-it enabled
5. Clean codebase with comprehensive documentation

**Overall Benefits:**
- 40-50% reduction in MCP-related code (~2000 lines removed)
- Improved MCP spec compliance
- Better maintainability and extensibility
- Foundation for future enhancements

---

*This document serves as the implementation roadmap for the ultrafast-mcp migration. All task execution should reference this document and update task status in Goaly (Goal ID: 23).*