# Tasks: Multi-Provider LLM Adapter

**Input**: Design documents from `/specs/002-llm-provider-adapter/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/

**Tests**: Included — the project constitution (Principle III: Test-First Development) mandates TDD for all new functionality.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Cargo workspace**: `crates/sai-llm/src/` for source

---

## Phase 1: Setup

**Purpose**: Create the sai-llm crate structure and dependencies

- [ ] T001 Create directory structure: `crates/sai-llm/src/convert/`
- [ ] T002 Create `crates/sai-llm/Cargo.toml` with dependencies: sai-core (workspace), genai 0.5, tokio, async-trait, serde, serde_json, thiserror, uuid; dev-dependencies: mockall, tokio-test, tokio-stream
- [ ] T003 Add `sai-llm` to workspace members in root `Cargo.toml`
- [ ] T004 [P] Create `crates/sai-llm/src/lib.rs` with module declarations for `adapter`, `convert`, `provider`
- [ ] T005 [P] Create `crates/sai-llm/src/convert/mod.rs` with submodule declarations for `request`, `response`, `tools`, `errors`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Conversion utilities and provider routing that ALL user stories depend on

**CRITICAL**: No user story work can begin until this phase is complete

- [ ] T006 [P] Implement `crates/sai-llm/src/provider.rs`: model prefix → provider name mapping function; API key environment variable name lookup per provider; pre-flight key check function that returns `LlmError::Connection` if key is missing
- [ ] T007 [P] Implement `crates/sai-llm/src/convert/request.rs`: convert `sai_core::Message` variants to `genai::ChatMessage` (User → user, Assistant → assistant with text extraction, ToolResult → tool with ToolCallResponse); convert system prompt; convert tool definitions from `Vec<serde_json::Value>` to `Vec<genai::Tool>`
- [ ] T008 [P] Implement `crates/sai-llm/src/convert/errors.rs`: map genai error types to `sai_core::error::LlmError` variants (connection, rate limit with retry_after, token limit, provider error); extract retry-after from HTTP headers
- [ ] T009 [P] Create stub `crates/sai-llm/src/convert/response.rs` and `crates/sai-llm/src/convert/tools.rs` (filled in US1/US2)
- [ ] T010 [P] Create stub `crates/sai-llm/src/adapter.rs` with `GenaiLlmAdapter` struct holding `genai::Client` and `model_name: String`
- [ ] T011 Verify `cargo check -p sai-llm` compiles cleanly

**Checkpoint**: Foundation ready — conversion utilities and provider routing compile.

---

## Phase 3: User Story 1 — Text Conversation with Any Provider (Priority: P1) MVP

**Goal**: Send a text prompt, receive a normalized streaming response from any provider

**Independent Test**: Mock genai to return text stream events; verify sai-core ChatStreamEvent sequence (StreamStart, TextDelta*, StreamEnd)

### Tests for User Story 1

> **Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T012 [P] [US1] Unit test: given a text-only genai stream, when `chat_stream()` called, then returns StreamStart + TextDelta + StreamEnd with StopReason::EndTurn in `crates/sai-llm/src/adapter.rs` (test module)
- [ ] T013 [P] [US1] Unit test: given genai stream with MaxTokens stop, when response complete, then StreamEnd has StopReason::MaxTokens in `crates/sai-llm/src/adapter.rs` (test module)
- [ ] T014 [P] [US1] Unit test: given Message::User and Message::Assistant in request, when converted, then genai ChatMessages have correct roles in `crates/sai-llm/src/convert/request.rs` (test module)

### Implementation for User Story 1

- [ ] T015 [US1] Implement `crates/sai-llm/src/convert/response.rs`: convert genai `ChatStreamEvent::Chunk` to `ChatStreamEvent::TextDelta`; convert `ChatStreamEvent::End` to `ChatStreamEvent::StreamEnd` with stop_reason mapping (EndTurn, MaxTokens, ToolUse)
- [ ] T016 [US1] Implement `GenaiLlmAdapter::chat_stream()` in `crates/sai-llm/src/adapter.rs`: pre-flight API key check, convert ChatRequest to genai ChatRequest via convert/request.rs, call `genai::Client::exec_chat_stream()`, wrap genai stream in a tokio mpsc channel that converts events via convert/response.rs, return as `ChatStream`
- [ ] T017 [US1] Implement `LlmPort` trait for `GenaiLlmAdapter` in `crates/sai-llm/src/adapter.rs`: `chat_stream()` delegates to the method above; `model_name()` returns stored name; `provider_name()` returns derived provider
- [ ] T018 [US1] Verify all US1 tests pass with `cargo nextest run -p sai-llm`

**Checkpoint**: Text conversations work — streaming responses from any genai-supported provider

---

## Phase 4: User Story 2 — Tool Calling Across Providers (Priority: P1)

**Goal**: Tool definitions sent to providers, tool-call responses normalized with unique IDs and parsed args

**Independent Test**: Mock genai to return tool-call events; verify normalized ToolCall has id, name, parsed input

### Tests for User Story 2

> **Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T019 [P] [US2] Unit test: given genai End event with tool calls, when converted, then each produces ChatStreamEvent::ToolCallComplete with id, name, parsed args in `crates/sai-llm/src/convert/tools.rs` (test module)
- [ ] T020 [P] [US2] Unit test: given tool call args as JSON string (OpenAI style), when converted, then args are parsed into serde_json::Value object in `crates/sai-llm/src/convert/tools.rs` (test module)
- [ ] T021 [P] [US2] Unit test: given tool call without call ID (Ollama style), when converted, then a synthetic UUID is generated in `crates/sai-llm/src/convert/tools.rs` (test module)
- [ ] T022 [P] [US2] Unit test: given tool definitions as Vec<serde_json::Value>, when converted, then genai Tool structs have correct name, description, schema in `crates/sai-llm/src/convert/request.rs` (test module)
- [ ] T023 [P] [US2] Unit test: given Message::ToolResult, when converted to genai message, then ToolCallResponse has matching call_id and content in `crates/sai-llm/src/convert/request.rs` (test module)

### Implementation for User Story 2

- [ ] T024 [US2] Implement `crates/sai-llm/src/convert/tools.rs`: extract tool calls from genai End event; normalize each: ensure id present (generate UUID if missing), ensure args are parsed Value (parse JSON string if needed); return Vec<ToolCall>
- [ ] T025 [US2] Update `crates/sai-llm/src/convert/response.rs`: when genai End event contains tool calls, emit one ChatStreamEvent::ToolCallComplete per call before StreamEnd
- [ ] T026 [US2] Update `crates/sai-llm/src/convert/request.rs`: convert tool definitions from Vec<serde_json::Value> to Vec<genai::Tool> using name/description/schema fields; convert Message::ToolResult to genai::ChatMessage::tool()
- [ ] T027 [US2] Verify all US2 tests pass with `cargo nextest run -p sai-llm`

**Checkpoint**: Tool calling works — definitions sent, responses normalized with IDs and parsed args

---

## Phase 5: User Story 3 — Runtime Provider Switching (Priority: P2)

**Goal**: Changing model name mid-session routes next request to new provider

**Independent Test**: Create adapter with model A, switch to model B, verify provider_name() changes

### Tests for User Story 3

> **Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T028 [P] [US3] Unit test: given adapter with "claude-sonnet-4", when set_model("gpt-4o") called, then model_name() returns "gpt-4o" and provider_name() returns "openai" in `crates/sai-llm/src/adapter.rs` (test module)
- [ ] T029 [P] [US3] Unit test: given adapter switched to new provider, when chat_stream() called, then pre-flight check uses new provider's API key variable in `crates/sai-llm/src/adapter.rs` (test module)

### Implementation for User Story 3

- [ ] T030 [US3] Add `set_model(name: &str)` method to `GenaiLlmAdapter` in `crates/sai-llm/src/adapter.rs`: update stored model_name and derived provider_name; no client restart needed (genai routes by model string)
- [ ] T031 [US3] Verify all US3 tests pass with `cargo nextest run -p sai-llm`

**Checkpoint**: Runtime switching works — model changes take effect on next request

---

## Phase 6: User Story 4 — API Key Resolution (Priority: P2)

**Goal**: Clear error messages when API keys are missing

**Independent Test**: Unset API key env var, call chat_stream(), verify error names the missing variable

### Tests for User Story 4

> **Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T032 [P] [US4] Unit test: given ANTHROPIC_API_KEY not set, when chat_stream() called with claude model, then LlmError::Connection contains "ANTHROPIC_API_KEY" in `crates/sai-llm/src/provider.rs` (test module)
- [ ] T033 [P] [US4] Unit test: given Ollama model, when chat_stream() called without any API key, then no error (local provider) in `crates/sai-llm/src/provider.rs` (test module)
- [ ] T034 [P] [US4] Unit test: given unknown model prefix "xyz-model", when provider lookup called, then error lists supported prefixes in `crates/sai-llm/src/provider.rs` (test module)

### Implementation for User Story 4

- [ ] T035 [US4] Update pre-flight check in `crates/sai-llm/src/provider.rs`: if model prefix is unrecognized, return LlmError::Provider with message listing supported prefixes (claude-*, gpt-*, gemini-*, ollama::*)
- [ ] T036 [US4] Ensure Ollama models skip API key check entirely in `crates/sai-llm/src/provider.rs`
- [ ] T037 [US4] Verify all US4 tests pass with `cargo nextest run -p sai-llm`

**Checkpoint**: API key errors are clear and actionable; local providers work without keys

---

## Phase 7: User Story 5 — Error Normalization (Priority: P3)

**Goal**: Provider-specific errors mapped to standard LlmError variants

**Independent Test**: Feed provider-specific error responses through error converter; verify correct LlmError variant

### Tests for User Story 5

> **Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T038 [P] [US5] Unit test: given HTTP 429 response with Retry-After header, when converted, then LlmError::RateLimited with correct retry_after_secs in `crates/sai-llm/src/convert/errors.rs` (test module)
- [ ] T039 [P] [US5] Unit test: given connection timeout error, when converted, then LlmError::Connection and is_retryable() returns true in `crates/sai-llm/src/convert/errors.rs` (test module)
- [ ] T040 [P] [US5] Unit test: given context-length-exceeded error, when converted, then LlmError::TokenLimitExceeded in `crates/sai-llm/src/convert/errors.rs` (test module)

### Implementation for User Story 5

- [ ] T041 [US5] Complete error mapping in `crates/sai-llm/src/convert/errors.rs`: handle all genai error variants including HTTP status codes; extract Retry-After header value (default 5s if absent); classify auth errors as non-retryable Connection
- [ ] T042 [US5] Wire error conversion into the streaming pipeline in `crates/sai-llm/src/adapter.rs`: when genai stream yields an error, convert via errors.rs and yield as Err(LlmError) in the ChatStream
- [ ] T043 [US5] Verify all US5 tests pass with `cargo nextest run -p sai-llm`

**Checkpoint**: All provider errors correctly classified — retry logic in agent loop works consistently

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Code quality, edge cases, documentation

- [ ] T044 [P] Handle streaming connection drop: when genai stream ends unexpectedly (no End event), emit LlmError::Connection in `crates/sai-llm/src/adapter.rs`
- [ ] T045 [P] Handle unexpected response format: when genai returns unparseable data, emit LlmError::Provider with raw response context in `crates/sai-llm/src/convert/response.rs`
- [ ] T046 [P] Handle invalid JSON in tool call arguments: deliver raw string wrapped in serde_json::Value::String instead of failing in `crates/sai-llm/src/convert/tools.rs`
- [ ] T047 Run `cargo clippy -p sai-llm` and fix all warnings
- [ ] T048 Run `cargo doc -p sai-llm --no-deps` and ensure all public items have doc comments
- [ ] T049 Run `cargo nextest run -p sai-llm` and verify all tests pass (full regression)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Foundational — streaming text
- **US2 (Phase 4)**: Depends on US1 — tool calling extends streaming
- **US3 (Phase 5)**: Depends on US1 — provider switching
- **US4 (Phase 6)**: Depends on Foundational — API key checks
  - US3 and US4 can run in parallel after US1
- **US5 (Phase 7)**: Depends on Foundational — error mapping
  - US5 can run in parallel with US3 and US4
- **Polish (Phase 8)**: Depends on all user stories

### User Story Dependencies

```
Phase 1: Setup
    │
Phase 2: Foundational
    │
Phase 3: US1 (streaming text) ◄── MVP
    │
Phase 4: US2 (tool calling)
    ├──────────┬──────────┐
Phase 5: US3  Phase 6: US4  Phase 7: US5
(switching)   (API keys)   (errors)
    │          │            │
    └──────────┴────────────┘
              │
Phase 8: Polish
```

### Parallel Opportunities

- All Setup tasks T004-T005 marked [P] can run in parallel
- All Foundational tasks T006-T010 marked [P] can run in parallel
- Within each story: test tasks marked [P] can run in parallel
- US3, US4, and US5 can all run in parallel after US2

---

## Implementation Strategy

### MVP First (User Story 1)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: US1 (streaming text)
4. **STOP and VALIDATE**: Verify text conversations work with at least one provider
5. This is sufficient for the agent loop to function with text-only interactions

### Full Feature

1. MVP above
2. Add US2 (tool calling) → agent loop fully functional with tools
3. Add US3 + US4 + US5 in parallel → production-ready adapter
4. Polish → clean, documented, all edge cases handled

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story
- genai types MUST NOT appear in sai-llm's public API
- Integration tests with real providers should be in a separate test file gated by env var checks
- Commit after each phase or logical group
