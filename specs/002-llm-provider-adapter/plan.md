# Implementation Plan: Multi-Provider LLM Adapter

**Branch**: `002-llm-provider-adapter` | **Date**: 2026-04-05 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/002-llm-provider-adapter/spec.md`

## Summary

Implement the `sai-llm` adapter crate that provides a concrete
implementation of `sai-core::ports::llm::LlmPort`. The adapter uses
the `genai` crate v0.5 as its multi-provider abstraction, translating
between the sai-core normalized interface (ChatRequest, ChatStream,
ChatStreamEvent) and genai's provider-specific handling. The adapter
auto-detects providers from model identifier strings, resolves API
credentials from environment variables, normalizes streaming responses
and tool-call formats, and maps provider-specific errors to sai-core's
`LlmError` types.

## Technical Context

**Language/Version**: Rust 2021 edition, MSRV 1.80.0
**Primary Dependencies**: genai 0.5 (multi-provider LLM abstraction),
tokio (async runtime), serde/serde_json (serialization), async-trait,
thiserror, reqwest (transitive via genai)
**Storage**: N/A
**Testing**: cargo-nextest, mockall; integration tests require live
API keys (gated behind feature flag or env var check)
**Target Platform**: macOS, Linux (CLI terminal)
**Project Type**: Cargo workspace — adapter crate `sai-llm` depending
on `sai-core` (port traits) and `genai` (provider abstraction)
**Performance Goals**: <500ms adapter overhead on first streamed token;
zero-copy text delta forwarding where possible
**Constraints**: sai-llm MUST NOT leak genai types into its public API;
all public types come from sai-core. genai is a private implementation
detail.
**Scale/Scope**: 4 provider families (Anthropic, OpenAI, Gemini,
Ollama); extensible to 14+ via genai's built-in routing

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
|-----------|--------|----------|
| I. Hexagonal Architecture | PASS | sai-llm is an adapter crate implementing `LlmPort` from sai-core. genai is an internal dependency — no genai types in the public API. |
| II. Multi-Provider LLM Abstraction | PASS | This feature IS the multi-provider implementation. 4+ providers via genai, normalized through the LlmPort trait. |
| III. Test-First Development | PASS | Unit tests use mock genai responses. Integration tests are gated behind env var checks for API keys. |
| IV. Type-Safe Domain Modeling | PASS | Conversion between genai types and sai-core types is explicit with typed error variants. No unwrap() in production code. |
| V. Security by Default | PASS | API keys read from env vars only, never logged or serialized. Missing key errors don't leak partial credentials. |

**Gate result**: All 5 principles pass.

## Project Structure

### Documentation (this feature)

```text
specs/002-llm-provider-adapter/
├── plan.md              # This file
├── research.md          # Phase 0: genai API mapping, provider differences
├── data-model.md        # Phase 1: conversion types
├── quickstart.md        # Phase 1: how to test with different providers
├── contracts/           # Phase 1: LlmPort implementation contract
└── tasks.md             # Phase 2 output
```

### Source Code (repository root)

```text
crates/
├── sai-core/                    # Already exists (feature 001)
│   └── src/
│       └── ports/llm.rs         # LlmPort trait (consumed, not modified)
├── sai-llm/
│   └── src/
│       ├── lib.rs               # Public API: GenaiLlmAdapter
│       ├── adapter.rs           # LlmPort implementation
│       ├── convert/
│       │   ├── mod.rs
│       │   ├── request.rs       # ChatRequest → genai ChatRequest
│       │   ├── response.rs      # genai ChatStreamEvent → sai ChatStreamEvent
│       │   ├── tools.rs         # Tool definition normalization
│       │   └── errors.rs        # genai errors → LlmError
│       └── provider.rs          # Model-to-provider routing, API key resolution
└── sai-cli/                     # Will wire GenaiLlmAdapter (future feature)

tests/
└── llm_integration.rs           # Live provider tests (gated by env vars)
```

**Structure Decision**: Cargo workspace adapter crate. Conversion logic
is split into `convert/` submodules — one per concern (request, response,
tools, errors) — so provider-specific changes are isolated. The
`provider.rs` module handles model routing and credential resolution.

## Complexity Tracking

> No constitution violations. Table intentionally empty.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| (none)    |            |                                     |
