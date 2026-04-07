# Quickstart: Interactive Conversation Loop

**Feature**: 004-conversation-loop
**Date**: 2026-04-06

## Prerequisites

- Rust toolchain >= 1.80.0
- cargo-nextest installed (`cargo install cargo-nextest`)
- Project cloned and on branch `004-conversation-loop`
- An LLM provider API key (e.g., `ANTHROPIC_API_KEY`)

## Setup

1. The `sai-cli` crate should be added to the workspace `Cargo.toml`:
   ```toml
   [workspace]
   members = ["crates/sai-core", "crates/sai-llm", "crates/sai-tools", "crates/sai-cli"]
   ```

2. Create `crates/sai-cli/Cargo.toml` with dependencies:
   - sai-core (workspace)
   - sai-llm (workspace)
   - sai-tools (workspace)
   - tokio (workspace, full features)
   - clap (v4, derive feature)
   - color-eyre
   - tracing + tracing-subscriber (env-filter feature)
   - tokio-util (for CancellationToken)

3. Build the workspace:
   ```sh
   cargo build
   ```

## Running

```sh
# Interactive mode
ANTHROPIC_API_KEY=sk-... cargo run -p sai-cli

# With initial message
ANTHROPIC_API_KEY=sk-... cargo run -p sai-cli -- "What files are in src/"

# With a specific model
ANTHROPIC_API_KEY=sk-... cargo run -p sai-cli -- --model claude-sonnet-4 "Hello"
```

## Running Tests

```sh
# All tests
cargo nextest run -p sai-cli

# With output
cargo nextest run -p sai-cli -- --nocapture

# All workspace tests (verify no regressions)
cargo nextest run
```

## Quick Verification

After implementing, verify the full stack works:

```sh
# Build succeeds
cargo build -p sai-cli

# Clippy clean
cargo clippy -p sai-cli -- -D warnings

# Format check
cargo fmt -p sai-cli -- --check

# Unit tests pass
cargo nextest run -p sai-cli

# Manual interactive test
ANTHROPIC_API_KEY=sk-... cargo run -p sai-cli
# Type: "What is 2+2?"
# Verify streamed response appears
# Type: /exit
# Verify clean exit
```

## Implementation Order

Follow the user story priorities from the spec:

1. **P1**: Scaffold crate + `main.rs` + clap args — establishes the binary
2. **P1**: `TerminalUi` adapter — renders events to terminal
3. **P1**: `TerminalPermissions` adapter — interactive y/n prompts
4. **P1**: Input reader — stdin reading with exit detection
5. **P1**: REPL loop — wires everything together for first conversation
6. **P1**: Signal handling — Ctrl-C support
7. **P2**: Error recovery — graceful handling of LLM failures
8. **P3**: Initial message via CLI argument
9. **P3**: Integration tests

Each component should be test-driven: write a failing test, implement, refactor.

## Key Patterns

- `main.rs` is purely DI wiring: create adapters, inject into AgentLoop, launch REPL
- `TerminalUi` implements `UiPort` from `sai-core` — stdout for text, stderr for metadata
- `TerminalPermissions` implements `PermissionPort` from `sai-core` — interactive y/n
- REPL loop uses `tokio::select!` to race input reading against signal handling
- Errors from `run_turn()` are displayed and the loop continues (never exits on transient errors)
