# Tasks: Core Agent Loop

**Input**: Design documents from `/specs/001-core-agent-loop/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/

**Tests**: Included — the project constitution (Principle III: Test-First Development) mandates TDD for all new functionality.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Cargo workspace**: `crates/<crate-name>/src/` for source, `crates/<crate-name>/tests/` for integration tests

---

## Phase 1: Setup

**Purpose**: Create the crate directory structure and module scaffolding

- [x] T001 Create directory structure for `sai-core` crate: `crates/sai-core/src/domain/`, `crates/sai-core/src/ports/`, `crates/sai-core/src/services/`
- [x] T002 Create `crates/sai-core/Cargo.toml` with workspace dependencies: tokio (sync feature), serde, serde_json, async-trait, thiserror, uuid; dev-dependencies: mockall, tokio-test
- [x] T003 [P] Create `crates/sai-core/src/lib.rs` with module declarations for `domain`, `ports`, `services`, `error`
- [x] T004 [P] Create module index files: `crates/sai-core/src/domain/mod.rs`, `crates/sai-core/src/ports/mod.rs`, `crates/sai-core/src/services/mod.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Domain types, port traits, and error types that ALL user stories depend on

**CRITICAL**: No user story work can begin until this phase is complete

- [x] T005 [P] Define `AgentError` and `LlmError` enums with thiserror in `crates/sai-core/src/error.rs` per research.md R5
- [x] T006 [P] Define `Message` enum (User, Assistant, ToolResult variants), `ContentBlock` enum, and `StopReason` enum in `crates/sai-core/src/domain/message.rs` per data-model.md
- [x] T007 [P] Define `ToolCall` struct and `ToolResult` struct with `ToolResultStatus` enum in `crates/sai-core/src/domain/tool_call.rs` per data-model.md
- [x] T008 [P] Define `AgentConfig` struct with defaults (max_iterations: 50, max_parallel: 10, max_retries: 3) in `crates/sai-core/src/domain/config.rs` per data-model.md
- [x] T009 [P] Define `AgentEvent` enum (StreamStart, TextDelta, ToolCallStart, ToolCallComplete, TurnComplete, Error) in `crates/sai-core/src/domain/event.rs` per data-model.md
- [x] T010 [P] Define `LlmPort` async trait with `chat_stream()`, `model_name()`, `provider_name()` in `crates/sai-core/src/ports/llm.rs` per contracts/port-traits.md; include `ChatRequest`, `ChatResponse`, `ChatStream`, `ChatStreamEvent` types
- [x] T011 [P] Define `ToolPort` and `ToolRegistryPort` traits in `crates/sai-core/src/ports/tool.rs` per contracts/port-traits.md; include `is_concurrency_safe()`, `is_read_only()`, `tool_definitions()`
- [x] T012 [P] Define `UiPort` trait with `emit_event(AgentEvent)` in `crates/sai-core/src/ports/ui.rs` per contracts/port-traits.md
- [x] T013 [P] Define `PermissionPort` trait with `check()` returning `PermissionDecision` enum (Allow, Deny, Ask) in `crates/sai-core/src/ports/permissions.rs` per contracts/port-traits.md
- [x] T014 Wire all modules into parent `mod.rs` files and `lib.rs`; verify `cargo check -p sai-core` compiles cleanly

**Checkpoint**: Foundation ready — all domain types and port traits compile. User story implementation can now begin.

---

## Phase 3: User Story 1 — Single-Turn Text Response (Priority: P1) MVP

**Goal**: User sends a message, model responds with text only, response is streamed to user

**Independent Test**: Send a message via mock LlmPort that returns text-only stream; verify AgentEvent::TextDelta events emitted and final text returned

### Tests for User Story 1

> **Write these tests FIRST, ensure they FAIL before implementation**

- [x] T015 [P] [US1] Unit test: given mock LlmPort returns text-only stream, when `run_turn()` called, then returns final text in `crates/sai-core/src/services/agent_loop.rs` (test module)
- [x] T016 [P] [US1] Unit test: given mock LlmPort streams 3 text deltas, when `run_turn()` called, then 3 `AgentEvent::TextDelta` events emitted via mock UiPort in `crates/sai-core/src/services/agent_loop.rs` (test module)
- [x] T017 [P] [US1] Unit test: given mock LlmPort returns empty response (no text, no tools), when `run_turn()` called, then end-of-turn without error in `crates/sai-core/src/services/agent_loop.rs` (test module)

### Implementation for User Story 1

- [x] T018 [US1] Create `AgentLoop` struct accepting port trait objects (LlmPort, ToolRegistryPort, UiPort, PermissionPort) and AgentConfig in `crates/sai-core/src/services/agent_loop.rs`
- [x] T019 [US1] Implement `collect_stream()` helper: consume ChatStream, accumulate text deltas, emit AgentEvents, return collected text and tool calls in `crates/sai-core/src/services/agent_loop.rs`
- [x] T020 [US1] Implement `run_turn(user_message)` method: build ChatRequest with system prompt + messages, call `LlmPort::chat_stream()`, collect stream, return text if no tool calls in `crates/sai-core/src/services/agent_loop.rs`
- [x] T021 [US1] Handle empty response edge case: treat as end-of-turn, emit `AgentEvent::TurnComplete`
- [x] T022 [US1] Verify all US1 tests pass with `cargo nextest run -p sai-core`

**Checkpoint**: User Story 1 fully functional — text-only conversations work end-to-end with mocked ports

---

## Phase 4: User Story 2 — Single Tool Call and Resolution (Priority: P1)

**Goal**: Model requests a tool, agent executes it, sends result back, model gives final answer

**Independent Test**: Mock LlmPort returns tool-use on first call, text-only on second; mock ToolRegistryPort returns a dummy tool; verify tool executed and result sent back

### Tests for User Story 2

> **Write these tests FIRST, ensure they FAIL before implementation**

- [x] T023 [P] [US2] Unit test: given model returns tool-use, when `run_turn()` called, then tool executed and result sent back to model in `crates/sai-core/src/services/agent_loop.rs` (test module)
- [x] T024 [P] [US2] Unit test: given model requests unknown tool, when tool lookup fails, then error result sent to model (not panic) in `crates/sai-core/src/services/agent_loop.rs` (test module)
- [x] T025 [P] [US2] Unit test: given tool execution fails, when error returned, then error sent as ToolResult::Error to model in `crates/sai-core/src/services/agent_loop.rs` (test module)
- [x] T026 [P] [US2] Unit test: given PermissionPort denies tool, when check returns Deny, then denial reason sent as tool error result in `crates/sai-core/src/services/tool_executor.rs` (test module)

### Implementation for User Story 2

- [x] T027 [US2] Create `ToolExecutor` struct with sequential `execute()` method in `crates/sai-core/src/services/tool_executor.rs`: takes Vec<ToolCall>, runs each via ToolRegistryPort, returns Vec<ToolResult>
- [x] T028 [US2] Add permission check in `ToolExecutor::execute()`: call PermissionPort::check() before each tool, convert Deny to ToolResult::Error
- [x] T029 [US2] Add tool dispatch to `AgentLoop::run_turn()`: when collect_stream returns tool_calls, call ToolExecutor, append results to history, re-call LlmPort
- [x] T030 [US2] Handle unknown tool: when ToolRegistryPort::get() returns None, create ToolResult::Error with descriptive message
- [x] T031 [US2] Handle tool execution failure: wrap error in ToolResult::Error, continue loop
- [x] T032 [US2] Emit `AgentEvent::ToolCallStart` and `AgentEvent::ToolCallComplete` for each tool execution
- [x] T033 [US2] Verify all US2 tests pass with `cargo nextest run -p sai-core`

**Checkpoint**: User Stories 1 AND 2 work — text-only and single-tool conversations functional

---

## Phase 5: User Story 3 — Multi-Turn Tool Chain (Priority: P2)

**Goal**: Model issues multiple sequential tool calls across loop iterations until final answer

**Independent Test**: Mock LlmPort returns tool-use 3 times, then text; verify all 3 tools execute and final response integrates context

### Tests for User Story 3

> **Write these tests FIRST, ensure they FAIL before implementation**

- [x] T034 [P] [US3] Unit test: given model returns tool-use 3 times then text, when `run_turn()` called, then all 3 tools execute sequentially and final text returned in `crates/sai-core/src/services/agent_loop.rs` (test module)
- [x] T035 [P] [US3] Unit test: given max_iterations_per_turn is 5 and model loops 6 times, when iteration limit exceeded, then AgentError::IterationLimitExceeded returned in `crates/sai-core/src/services/agent_loop.rs` (test module)

### Implementation for User Story 3

- [x] T036 [US3] Add iteration counter to `run_turn()`: increment on each tool-dispatch loop, check against `AgentConfig::max_iterations_per_turn`
- [x] T037 [US3] When iteration limit exceeded: emit `AgentEvent::Error(IterationLimitExceeded)`, break loop, return error to caller
- [x] T038 [US3] Ensure each iteration appends both the assistant message (with tool-use blocks) and the tool results to the history before re-calling the model
- [x] T039 [US3] Verify all US3 tests pass with `cargo nextest run -p sai-core`

**Checkpoint**: Multi-turn tool chains work — agent correctly loops through sequential tool calls

---

## Phase 6: User Story 4 — Parallel Tool Execution (Priority: P2)

**Goal**: Multiple tool calls in one response execute concurrently when safe

**Independent Test**: Mock LlmPort returns 3 tool calls; 2 concurrency-safe, 1 not; verify safe ones run in parallel, unsafe one runs sequentially

### Tests for User Story 4

> **Write these tests FIRST, ensure they FAIL before implementation**

- [x] T040 [P] [US4] Unit test: given 2 concurrency-safe tool calls, when executed, then both run in parallel (verify via timing or execution order) in `crates/sai-core/src/services/tool_executor.rs` (test module)
- [x] T041 [P] [US4] Unit test: given 1 non-safe and 1 safe tool call, when executed, then non-safe runs sequentially and safe runs in parallel batch in `crates/sai-core/src/services/tool_executor.rs` (test module)
- [x] T042 [P] [US4] Unit test: given max_parallel_tool_calls is 2 and 5 safe calls, when executed, then no more than 2 run simultaneously in `crates/sai-core/src/services/tool_executor.rs` (test module)

### Implementation for User Story 4

- [x] T043 [US4] Add `partition_tool_calls()` method to `ToolExecutor`: split calls into concurrency-safe and non-safe groups using `ToolPort::is_concurrency_safe()` in `crates/sai-core/src/services/tool_executor.rs`
- [x] T044 [US4] Implement parallel execution with `tokio::task::JoinSet` for concurrency-safe batch: spawn each tool call, collect results via `join_next()`, cap concurrency at `max_parallel_tool_calls` in `crates/sai-core/src/services/tool_executor.rs`
- [x] T045 [US4] Run non-safe batch sequentially after parallel batch completes; merge all results in original request order
- [x] T046 [US4] Update `AgentLoop::run_turn()` to use parallel-aware ToolExecutor (should be transparent — same `execute()` API)
- [x] T047 [US4] Verify all US4 tests pass with `cargo nextest run -p sai-core`

**Checkpoint**: Parallel tool execution works — concurrent-safe tools run in parallel, others sequentially

---

## Phase 7: User Story 5 — Conversation History Continuity (Priority: P3)

**Goal**: Multi-turn conversations preserve full history across user exchanges

**Independent Test**: Run 2 consecutive turns with mock LlmPort; verify second turn's ChatRequest includes all messages from the first turn

### Tests for User Story 5

> **Write these tests FIRST, ensure they FAIL before implementation**

- [x] T048 [P] [US5] Unit test: given 2 sequential turns, when second turn starts, then ChatRequest includes all messages from first turn in `crates/sai-core/src/services/agent_loop.rs` (test module)
- [x] T049 [P] [US5] Unit test: given history exceeds configured size limit, when checked, then signal emitted (does not crash) in `crates/sai-core/src/services/agent_loop.rs` (test module)

### Implementation for User Story 5

- [x] T050 [US5] Create `AgentSession` struct in `crates/sai-core/src/domain/session.rs` per data-model.md: id, config, messages vec, created_at
- [x] T051 [US5] Refactor `AgentLoop` to hold `AgentSession` state: `run_turn()` reads and appends to `session.messages` instead of taking messages as parameter
- [x] T052 [US5] Add history size check: after each turn, if message count exceeds a configurable threshold, emit a warning event (actual compression is out of scope)
- [x] T053 [US5] Verify all US5 tests pass with `cargo nextest run -p sai-core`

**Checkpoint**: Full conversation continuity — multi-turn sessions preserve context across exchanges

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Error handling, robustness, and code quality across all user stories

- [x] T054 [P] Add model error handling to `run_turn()`: catch LlmError variants (Connection, RateLimited, TokenLimitExceeded), emit AgentEvent::Error, return appropriate error in `crates/sai-core/src/services/agent_loop.rs`
- [x] T055 [P] Add retry with exponential backoff for transient LLM errors (RateLimited, Connection): max retries from AgentConfig::max_retries_on_error in `crates/sai-core/src/services/agent_loop.rs`
- [x] T056 [P] Handle MaxTokens stop reason: when model response is truncated, emit warning event and optionally re-request with increased budget in `crates/sai-core/src/services/agent_loop.rs`
- [x] T057 Run `cargo clippy -p sai-core` and fix all warnings
- [x] T058 Run `cargo doc -p sai-core --no-deps` and ensure all public items have doc comments
- [x] T059 Run `cargo nextest run -p sai-core` and verify all tests pass (full regression)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion — BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
  - US1 and US2 are both P1 but US2 depends on US1 (tool dispatch extends the basic loop)
  - US3 depends on US2 (multi-turn extends tool dispatch)
  - US4 depends on US2 (parallel extends tool executor)
  - US3 and US4 can run in parallel after US2
  - US5 depends on US1 (session state extends the basic loop)
  - US5 can run in parallel with US3 and US4
- **Polish (Phase 8)**: Depends on all user stories being complete

### User Story Dependencies

```
Phase 1: Setup
    │
Phase 2: Foundational
    │
Phase 3: US1 (text-only)
    │
Phase 4: US2 (single tool)
    ├──────────┬──────────┐
Phase 5: US3  Phase 6: US4  Phase 7: US5
(multi-tool)  (parallel)   (history)
    │          │            │
    └──────────┴────────────┘
              │
Phase 8: Polish
```

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Domain types before services
- Core logic before edge case handling
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks T005-T013 marked [P] can run in parallel
- Within US1: Tests T015-T017 can run in parallel
- Within US2: Tests T023-T026 can run in parallel
- Within US4: Tests T040-T042 can run in parallel
- US3, US4, and US5 can run in parallel after US2 completes

---

## Parallel Example: Foundational Phase

```bash
# Launch all domain types and port traits in parallel:
Task: "T005 [P] Define AgentError and LlmError in error.rs"
Task: "T006 [P] Define Message enum in domain/message.rs"
Task: "T007 [P] Define ToolCall struct in domain/tool_call.rs"
Task: "T008 [P] Define AgentConfig in domain/config.rs"
Task: "T009 [P] Define AgentEvent in domain/event.rs"
Task: "T010 [P] Define LlmPort in ports/llm.rs"
Task: "T011 [P] Define ToolPort in ports/tool.rs"
Task: "T012 [P] Define UiPort in ports/ui.rs"
Task: "T013 [P] Define PermissionPort in ports/permissions.rs"
```

## Parallel Example: After US2

```bash
# Launch US3, US4, and US5 in parallel:
Task: "T034 [P] [US3] Unit test: multi-tool chain"
Task: "T040 [P] [US4] Unit test: parallel safe tools"
Task: "T048 [P] [US5] Unit test: history across turns"
```

---

## Implementation Strategy

### MVP First (User Story 1 + 2)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL — blocks all stories)
3. Complete Phase 3: User Story 1 (text-only responses)
4. **STOP and VALIDATE**: Verify text-only conversations work
5. Complete Phase 4: User Story 2 (single tool call)
6. **STOP and VALIDATE**: Verify tool execution works end-to-end
7. Deploy/demo if ready — this is a functional coding agent MVP

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add US1 → Test independently → Text conversations work (MVP-0)
3. Add US2 → Test independently → Tool execution works (MVP-1)
4. Add US3 + US4 in parallel → Multi-tool and parallel work
5. Add US5 → Session history works
6. Polish → Production-ready quality

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable (except US2 depends on US1's basic loop)
- Verify tests fail before implementing
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
