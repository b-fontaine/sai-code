# Implementation Plan: Core Agent Loop

**Branch**: `001-core-agent-loop` | **Date**: 2026-04-05 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-core-agent-loop/spec.md`

## Summary

Implement the foundational agent loop in `sai-core` as a domain service
that orchestrates the LLM conversation cycle: assemble messages, call
the model via the `LlmPort` trait, inspect the response for tool-use
requests, dispatch tool calls through the `ToolRegistryPort`, append
results, and loop until end-of-turn. Streaming is supported via an
async event channel. Parallel tool execution uses `tokio::JoinSet` with
concurrency-safety partitioning. The loop enforces a configurable
iteration limit and handles all error paths (unknown tool, tool failure,
connection loss, token limit exceeded) by feeding errors back to the
model or surfacing them to the user via the `UiPort`.

## Technical Context

**Language/Version**: Rust 2021 edition, MSRV 1.80.0
**Primary Dependencies**: tokio (async runtime), serde/serde_json
(serialization), async-trait (port traits), thiserror (domain errors),
uuid (call IDs), genai 0.5 (LLM provider, in sai-llm adapter only)
**Storage**: N/A (in-memory conversation history; session persistence
is out of scope)
**Testing**: cargo-nextest, mockall (mock port traits)
**Target Platform**: macOS, Linux (CLI terminal)
**Project Type**: Cargo workspace — domain logic in `sai-core` library
crate, wiring in `sai-cli` binary crate
**Performance Goals**: <2s to first streamed token (excluding provider
latency), <500ms end-of-turn detection after final token
**Constraints**: sai-core MUST have zero infrastructure dependencies;
all LLM and I/O concerns go through port traits
**Scale/Scope**: Single-user CLI tool; sessions up to 20+ turns with
up to 50 tool-call iterations per turn

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
|-----------|--------|----------|
| I. Hexagonal Architecture | PASS | Agent loop is a domain service in `sai-core`. LLM, tools, UI, and permissions are accessed only through port traits. No infrastructure imports in sai-core. |
| II. Multi-Provider LLM Abstraction | PASS | The agent loop calls `LlmPort::chat_stream()` — it never references genai, provider names, or wire formats. Provider normalization happens in `sai-llm`. |
| III. Test-First Development | PASS | All port traits are mockable via `mockall`. Unit tests will use mock LLM and tool implementations. Integration tests will verify the full loop with real adapters. |
| IV. Type-Safe Domain Modeling | PASS | Domain entities (Message, ToolCall, ToolResult, etc.) use typed enums and structs. Errors use `thiserror` with explicit variants. No `unwrap()` in domain code. |
| V. Security by Default | PASS | Tool execution flows through `PermissionPort::check()` before dispatch. The agent loop does not bypass permissions. Unknown tools produce error results, not panics. |

**Gate result**: All 5 principles pass. Proceeding to Phase 0.

## Project Structure

### Documentation (this feature)

```text
specs/001-core-agent-loop/
├── plan.md              # This file
├── research.md          # Phase 0: genai API, async patterns, tool dispatch
├── data-model.md        # Phase 1: domain entities and relationships
├── quickstart.md        # Phase 1: how to run the agent loop
├── contracts/           # Phase 1: port trait contracts
└── tasks.md             # Phase 2 (/speckit.tasks — NOT created here)
```

### Source Code (repository root)

```text
crates/
├── sai-core/
│   └── src/
│       ├── lib.rs
│       ├── domain/
│       │   ├── mod.rs
│       │   ├── message.rs       # Message, Role, Content enums
│       │   ├── tool_call.rs     # ToolCall, ToolResult structs
│       │   ├── session.rs       # AgentSession, ConversationTurn
│       │   └── config.rs        # AgentConfig (iteration limit, etc.)
│       ├── ports/
│       │   ├── mod.rs
│       │   ├── llm.rs           # LlmPort, ChatRequest, ChatResponse, ChatStream
│       │   ├── tool.rs          # ToolPort, ToolRegistryPort
│       │   ├── ui.rs            # UiPort (streaming output, permission prompts)
│       │   └── permissions.rs   # PermissionPort
│       ├── services/
│       │   ├── mod.rs
│       │   ├── agent_loop.rs    # AgentLoop service (the core loop)
│       │   └── tool_executor.rs # ToolExecutor (parallel/serial dispatch)
│       └── error.rs             # AgentError, ToolError, LlmError
├── sai-llm/
│   └── src/
│       ├── lib.rs
│       └── genai_adapter.rs     # GenaiLlmAdapter implements LlmPort
└── sai-cli/
    └── src/
        └── main.rs              # Wires AgentLoop with concrete adapters

tests/
├── agent_loop_unit.rs           # Unit tests with mocked ports
└── agent_loop_integration.rs    # Integration tests with real adapters
```

**Structure Decision**: Cargo workspace with hexagonal architecture.
The agent loop lives in `sai-core/src/services/agent_loop.rs` as a
pure domain service. Tool execution logic is separated into
`tool_executor.rs` for parallel/serial partitioning. All external
concerns accessed via port traits defined in `sai-core/src/ports/`.

## Complexity Tracking

> No constitution violations. Table intentionally empty.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| (none)    |            |                                     |
