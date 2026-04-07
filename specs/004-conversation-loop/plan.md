# Implementation Plan: Interactive Conversation Loop

**Branch**: `004-conversation-loop` | **Date**: 2026-04-06 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/004-conversation-loop/spec.md`

## Summary

Add the `sai-cli` binary crate that wires all existing library crates together and implements an interactive conversation loop (REPL). The CLI reads user input from stdin, delegates each message to `AgentLoop::run_turn()`, streams the response token-by-token to stdout via a concrete `UiPort` adapter, handles signal-based exit (Ctrl-C/Ctrl-D) and command-based exit (`/exit`, `/quit`), and loops until termination.

## Technical Context

**Language/Version**: Rust 2021 edition, MSRV 1.80.0
**Primary Dependencies**: tokio (async runtime), sai-core (domain/ports), sai-llm (LLM adapter), sai-tools (tool registry), clap (CLI args), crossterm (raw terminal input, signal handling), color-eyre (application-level errors), tracing + tracing-subscriber (logging)
**Storage**: N/A (no persistence; session is in-memory only)
**Testing**: cargo-nextest, mockall for port mocks, assert_cmd + predicates for CLI integration tests
**Target Platform**: macOS, Linux (POSIX terminals); Windows support deferred
**Project Type**: Binary crate (`sai-cli`) consuming library crates
**Performance Goals**: <500ms overhead from model first-token to user-visible output; prompt redisplay <50ms after turn completion
**Constraints**: Must not block the async runtime on stdin reads; signal handling must work during both input and streaming phases
**Scale/Scope**: Single binary, ~6-8 source modules, 3 concrete port adapters (UI, permissions, input)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Hexagonal Architecture | PASS | `sai-cli` is the application crate that wires adapters to ports via DI. All concrete adapters (TerminalUi, TerminalPermissions) live in `sai-cli`, not in domain. `sai-core` remains infrastructure-free. |
| II. Multi-Provider LLM Abstraction | PASS | CLI uses `GenaiLlmAdapter` from `sai-llm` — model selected by config string. No provider-specific code in CLI. |
| III. Test-First Development | PASS | Domain interactions tested via mocked ports. CLI integration tested with assert_cmd. Signal handling tested manually (documented). |
| IV. Type-Safe Domain Modeling | PASS | Application errors use `color-eyre`. No string-only errors. No `unsafe`. Doc comments on public items. |
| V. Security by Default | PASS | `TerminalPermissions` adapter prompts user interactively for non-read-only tools. Fail-closed default: unknown tool permissions → deny. API keys from env vars only. |

No violations. No complexity tracking needed.

## Project Structure

### Documentation (this feature)

```text
specs/004-conversation-loop/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── cli-contract.md
└── tasks.md             # Phase 2 output (via /speckit.tasks)
```

### Source Code (repository root)

```text
crates/sai-cli/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point: parse args, wire DI, launch loop
│   ├── cli.rs               # Clap argument definitions
│   ├── repl.rs              # Conversation loop (read → run_turn → display → repeat)
│   ├── terminal_ui.rs       # UiPort implementation: renders AgentEvents to terminal
│   ├── terminal_permissions.rs  # PermissionPort implementation: interactive y/n prompts
│   ├── input.rs             # Async stdin reader with signal handling
│   └── banner.rs            # Startup banner display
└── tests/
    └── cli_integration_test.rs  # End-to-end CLI tests with assert_cmd
```

**Structure Decision**: New `sai-cli` binary crate as prescribed by the constitution (Principle I: application crate wires adapters to ports). One module per concern. All port adapters are local to this crate — they are application-layer glue, not reusable library code.

## Phase 2: Implementation Steps

### Step 1: Scaffold `sai-cli` crate (P1 - US1 foundation)

- Add `crates/sai-cli` to workspace `Cargo.toml` members
- Create `Cargo.toml` with dependencies: sai-core, sai-llm, sai-tools, tokio, clap, color-eyre, tracing, tracing-subscriber
- Create minimal `main.rs` that compiles and exits

### Step 2: CLI argument parsing (P1 - US1, US7)

- Define clap struct in `cli.rs`: optional positional `<message>`, `--model` flag (default from env or `claude-sonnet-4`)
- Parse args in `main.rs`

### Step 3: Terminal UI adapter (P1 - US1, US4)

- Implement `UiPort` for `TerminalUi` in `terminal_ui.rs`
- Handle `AgentEvent::TextDelta` → write to stdout immediately (flush)
- Handle `AgentEvent::StreamStart` → print thinking indicator
- Handle `AgentEvent::ToolCallStart` → print tool name
- Handle `AgentEvent::ToolCallComplete` → print success/failure
- Handle `AgentEvent::TurnComplete` → newline
- Handle `AgentEvent::Error` → print to stderr
- Handle `AgentEvent::HistorySizeWarning` → print notice

### Step 4: Terminal permission adapter (P1 - US1)

- Implement `PermissionPort` for `TerminalPermissions` in `terminal_permissions.rs`
- `PermissionDecision::Ask` → prompt "Allow [tool_name]? (y/n): " on stderr, read response
- Read-only tools → `Allow`
- Default for non-interactive stdin → `Deny`

### Step 5: Input reader (P1 - US1, US3, US6)

- Implement async stdin reading in `input.rs`
- Detect interactive vs piped stdin (is_terminal check)
- Trim input; skip empty/whitespace-only lines
- Detect `/exit` and `/quit` commands
- Handle EOF (Ctrl-D) → return None to signal exit

### Step 6: Conversation loop (P1 - US1, US2, US3)

- Implement the REPL in `repl.rs`:
  1. Display startup banner (model name, working directory)
  2. If initial message from CLI args → run first turn
  3. Loop: read input → run_turn → display result → repeat
  4. On None from input reader (EOF/exit command) → break, farewell message
- Wire DI in `main.rs`: create AgentConfig, GenaiLlmAdapter, InMemoryToolRegistry::with_defaults(), TerminalUi, TerminalPermissions
- Create AgentLoop with all ports
- Call repl loop

### Step 7: Signal handling (P1 - US3)

- Install Ctrl-C handler via tokio::signal
- At input prompt: Ctrl-C → exit
- During active turn: first Ctrl-C → cancel turn (drop stream), return to prompt; second Ctrl-C → exit
- Use a shared atomic flag or channel to communicate cancellation

### Step 8: Error recovery (P2 - US5)

- Wrap `run_turn()` in error handling in the REPL loop
- On `AgentError::Llm(LlmError::Connection(_))` → print "Connection error: {msg}. You can try again."
- On `AgentError::Llm(LlmError::RateLimited(_))` → print "Rate limited. Please wait and try again."
- On `AgentError::IterationLimitExceeded` → print "Turn stopped: too many tool iterations."
- On any error → do NOT exit; continue the loop with existing session

### Step 9: Integration tests

- Test binary launches and exits with code 0 on EOF
- Test `--help` flag
- Test initial message via argument
- Test empty input produces no output
