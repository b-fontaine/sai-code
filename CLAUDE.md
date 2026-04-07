# sai-code Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-04-07

## Active Technologies
- Rust 2021 edition, MSRV 1.80.0 + genai 0.5 (multi-provider LLM abstraction), (002-llm-provider-adapter)
- Rust 2021 edition, MSRV 1.80.0 + okio (async), serde/serde_json (serialization), thiserror (errors), async-trait, globset (glob matching), grep-regex/grep-searcher (content search) (003-tool-function-execution)
- Local filesystem (read/write via `tokio::fs`) (003-tool-function-execution)
- Rust 2021 edition, MSRV 1.80.0 + okio (async runtime), sai-core (domain/ports), sai-llm (LLM adapter), sai-tools (tool registry), clap (CLI args), crossterm (raw terminal input, signal handling), color-eyre (application-level errors), tracing + tracing-subscriber (logging) (004-conversation-loop)
- N/A (no persistence; session is in-memory only) (004-conversation-loop)
- Rust 2021 edition, MSRV 1.80.0 + ratatui 0.29.x (pinned — v0.30 requires MSRV 1.86, incompatible with our MSRV), crossterm 0.28 (with `event-stream` feature), futures 0.3, sai-core (workspace), tokio (workspace, full) (005-ratatui-tui)
- N/A (no persistence; all state is in-memory for the duration of the session) (005-ratatui-tui)

- Rust 2021 edition, MSRV 1.80.0 + okio (async runtime), serde/serde_json (001-core-agent-loop)

## Project Structure

```text
src/
tests/
```

## Commands

cargo test [ONLY COMMANDS FOR ACTIVE TECHNOLOGIES][ONLY COMMANDS FOR ACTIVE TECHNOLOGIES] cargo clippy

## Code Style

Rust 2021 edition, MSRV 1.80.0: Follow standard conventions

## Recent Changes
- 005-ratatui-tui: Added Rust 2021 edition, MSRV 1.80.0 + ratatui 0.29.x (pinned — v0.30 requires MSRV 1.86, incompatible with our MSRV), crossterm 0.28 (with `event-stream` feature), futures 0.3, sai-core (workspace), tokio (workspace, full)
- 004-conversation-loop: Added Rust 2021 edition, MSRV 1.80.0 + okio (async runtime), sai-core (domain/ports), sai-llm (LLM adapter), sai-tools (tool registry), clap (CLI args), crossterm (raw terminal input, signal handling), color-eyre (application-level errors), tracing + tracing-subscriber (logging)
- 003-tool-function-execution: Added Rust 2021 edition, MSRV 1.80.0 + okio (async), serde/serde_json (serialization), thiserror (errors), async-trait, globset (glob matching), grep-regex/grep-searcher (content search)


<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
