<!--
Sync Impact Report
===================
Version change: 0.0.0 (template) → 1.0.0
Modified principles: N/A (initial ratification)
Added sections:
  - Principle I: Hexagonal Architecture
  - Principle II: Multi-Provider LLM Abstraction
  - Principle III: Test-First Development
  - Principle IV: Type-Safe Domain Modeling
  - Principle V: Security by Default
  - Technology Stack Constraints
  - Development Workflow
  - Governance
Templates requiring updates:
  - .specify/templates/plan-template.md — ✅ aligned (Constitution Check section present)
  - .specify/templates/spec-template.md — ✅ aligned (requirements + success criteria present)
  - .specify/templates/tasks-template.md — ✅ aligned (phased tasks with test-first notes present)
Follow-up TODOs: none
-->

# sai-code Constitution

## Core Principles

### I. Hexagonal Architecture

All business logic MUST reside in the `sai-core` crate, which has
**zero infrastructure dependencies**. External concerns (LLM providers,
MCP servers, TUI rendering, file I/O, configuration loading) are
implemented as adapter crates that depend on `sai-core` and implement
its port traits.

- `sai-core` defines port traits (`LlmPort`, `ToolPort`, `McpClientPort`,
  `UiPort`, `ContextPort`, `PermissionPort`, `CodeAnalysisPort`, `ConfigPort`).
- Adapter crates (`sai-llm`, `sai-mcp`, `sai-tui`, `sai-tools`,
  `sai-analysis`, `sai-config`, `sai-permissions`, `sai-context`)
  implement those traits.
- The application crate (`sai-cli`) wires adapters to ports via
  dependency injection. No other crate performs DI wiring.
- Domain entities and error types are defined in `sai-core` and shared
  across the workspace. Adapter crates MUST NOT define their own
  domain-level abstractions that duplicate core types.

**Rationale**: Decoupling domain logic from infrastructure makes every
adapter independently replaceable and every use-case independently
testable without external services.

### II. Multi-Provider LLM Abstraction

sai-code MUST support multiple LLM providers through a single unified
interface. Provider-specific details MUST NOT leak into domain logic.

- The `LlmPort` trait is the sole contract between the agent loop and
  any LLM provider. Switching providers at runtime means changing the
  model identifier string — no code changes required.
- `genai` is the primary abstraction crate, covering 14+ providers
  (OpenAI, Anthropic, Gemini, xAI, Ollama, Groq, DeepSeek, Cohere,
  and others).
- Tool/function calling differences across providers (schema keys,
  argument formats, result roles, stop signals) MUST be normalized
  within the `sai-llm` adapter, never exposed to consumers.
- Streaming MUST be supported for all providers that offer it, using
  a unified `ChatStream` type.

**Rationale**: Users choose their preferred provider; the agent's
capabilities MUST NOT depend on which provider is active.

### III. Test-First Development

Test-Driven Development is **mandatory** for all new functionality.

- The Red-Green-Refactor cycle MUST be followed: write a failing test,
  make it pass with minimal code, then refactor.
- `cargo-nextest` is the test runner. All tests MUST pass before code
  is merged.
- Domain logic in `sai-core` MUST be tested with unit tests using
  mocked port implementations (`mockall`).
- Adapter crates MUST have integration tests that verify correct
  behavior against their external dependencies (real or simulated).
- Contract tests MUST verify that each adapter satisfies the port
  trait's behavioral expectations.

**Rationale**: Tests written before implementation catch design issues
early and serve as living documentation of intended behavior.

### IV. Type-Safe Domain Modeling

Rust's type system MUST be leveraged to make invalid states
unrepresentable wherever practical.

- Domain errors use `thiserror` with explicit, typed variants — no
  string-only errors in domain code.
- Application-level errors use `color-eyre` for rich context and
  backtraces.
- All public API items MUST have doc comments (`/// ...`). Missing
  docs trigger a compiler warning (`missing_docs = "warn"`).
- `unsafe` code is **denied** workspace-wide
  (`unsafe_code = "deny"`).
- Clippy pedantic lints are enabled as warnings. `unwrap()` and
  `expect()` trigger warnings; use `Result`-based error propagation
  instead.

**Rationale**: Compile-time guarantees eliminate entire categories of
runtime bugs and reduce the surface area for security vulnerabilities.

### V. Security by Default

All tool execution MUST go through the permission system. The default
posture is **fail-closed**.

- Tools default to `is_read_only: false`, `is_destructive: false`,
  `is_concurrency_safe: false` — conservative defaults that require
  explicit opt-in for elevated capabilities.
- The permission pipeline MUST evaluate deny rules before any bypass
  mode. Dangerous paths (`.git/`, credential files, shell configs)
  are **bypass-immune** — no permission mode can override their
  protection.
- Shell commands MUST be validated via tree-sitter AST analysis
  (`tree-sitter-bash`) before execution to detect dangerous patterns
  (destructive commands, pipe-to-eval, sensitive path redirection).
- API keys MUST NOT appear in configuration files. Config files
  reference environment variable names
  (e.g., `api_key_env = "ANTHROPIC_API_KEY"`).

**Rationale**: A coding agent with shell and file access is a
high-privilege process. Security MUST be structural, not advisory.

## Technology Stack Constraints

- **Language**: Rust, 2021 edition, MSRV 1.80.0.
- **Build**: Cargo workspace with `resolver = "2"`. The workspace
  contains 10 crates (see Principle I for the list).
- **Async runtime**: Tokio with `features = ["full"]`.
- **Configuration**: TOML files with layered overrides
  (defaults → environment-specific → local → env vars). `serde` for
  deserialization, `config` crate for merging.
- **TUI**: `ratatui` v0.30 with the Component trait pattern,
  `crossterm` backend. Immediate-mode rendering at controlled frame
  rate (~30 FPS), decoupled from event processing via
  `tokio::mpsc` action channels.
- **MCP**: `rmcp` v1.3+ for Model Context Protocol client support
  (stdio and SSE transports).
- **Code analysis**: `tree-sitter` v0.26+ with language grammars for
  Rust, JavaScript, TypeScript, and Python (feature-gated).
- **CLI**: `clap` v4 with derive macros.
- **Logging**: `tracing` + `tracing-subscriber` with env-filter.

Adding a new workspace crate MUST be justified by a clear domain
boundary. Existing crates MUST NOT be split without demonstrating
that the split reduces coupling.

## Development Workflow

- **Branching**: Feature branches off `main`, named
  `NNN-feature-name` where NNN is the spec number.
- **Spec-Driven Development**: Every non-trivial feature goes through
  the SpecKit cycle: constitution check → specify → clarify → plan →
  tasks → implement.
- **Code review**: All PRs MUST pass CI (format, lint, test) before
  merge. Constitution compliance is verified as part of review.
- **Formatting**: `cargo fmt` with default settings. No custom
  `rustfmt.toml` unless a deviation is justified and documented.
- **Linting**: `cargo clippy` with workspace-level pedantic lints.
  Warnings MUST be resolved before merge, not suppressed.
- **Commit discipline**: Each commit represents a single logical
  change. Commit messages describe the *why*, not the *what*.

## Governance

This constitution is the highest-authority document for sai-code
development decisions. When any practice, convention, or
implementation conflicts with a constitutional principle, the
constitution prevails.

- **Amendments** require: (1) a written proposal describing the change
  and rationale, (2) review by at least one project maintainer,
  (3) a migration plan if the change affects existing code, and
  (4) propagation to all dependent templates and specs.
- **Versioning** follows semantic versioning:
  - MAJOR: principle removal, redefinition, or backward-incompatible
    governance change.
  - MINOR: new principle or section added, materially expanded
    guidance.
  - PATCH: wording clarifications, typo fixes, non-semantic
    refinements.
- **Compliance review**: Every PR and spec MUST be checked against
  the constitution. The plan template's "Constitution Check" section
  is the formal gate.

**Version**: 1.0.0 | **Ratified**: 2026-04-05 | **Last Amended**: 2026-04-05
