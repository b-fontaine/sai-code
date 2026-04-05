# Quickstart: Core Agent Loop

**Feature**: 001-core-agent-loop
**Date**: 2026-04-05

## Prerequisites

- Rust toolchain (1.80.0+)
- cargo-nextest installed (`cargo install cargo-nextest`)
- An LLM provider API key (e.g., `ANTHROPIC_API_KEY` or `OPENAI_API_KEY`)

## Build

```bash
# Build the entire workspace
cargo build

# Build only the core crate (no infrastructure deps needed)
cargo build -p sai-core

# Build the CLI binary
cargo build -p sai-cli
```

## Run Tests

```bash
# Run all tests (unit + integration)
cargo nextest run

# Run only sai-core unit tests (no API key needed)
cargo nextest run -p sai-core

# Run with a specific test filter
cargo nextest run -p sai-core agent_loop
```

## Run the Agent

```bash
# Set your provider API key
export ANTHROPIC_API_KEY="sk-ant-..."

# Run the CLI (once implemented)
cargo run -p sai-cli

# Or with a specific model
cargo run -p sai-cli -- --model "claude-sonnet-4"
```

## Verify the Agent Loop

Once the feature is implemented, verify these scenarios:

### 1. Text-only response (P1)

```
> What is 2+2?
```

Expected: The agent streams "4" (or similar) and returns to the prompt.

### 2. Single tool call (P1)

```
> Read the file Cargo.toml
```

Expected: The agent calls the file-read tool, sends the result to the
model, and displays a response referencing the file content.

### 3. Multi-tool chain (P2)

```
> Find all Rust files in crates/sai-core/src/ and tell me how many there are
```

Expected: The agent calls glob/search, then possibly reads files,
and produces a summary.

### 4. Error handling

```
> Use the nonexistent-tool to do something
```

Expected: The agent reports that the tool is not found and the model
recovers gracefully.

## Configuration

The agent loop reads configuration from `AgentConfig`:

| Setting | Default | Description |
|---------|---------|-------------|
| `system_prompt` | (empty) | System prompt prepended to every request |
| `model_name` | `"claude-sonnet-4"` | Model identifier string |
| `max_iterations_per_turn` | 50 | Max tool-call loops before stopping |
| `max_parallel_tool_calls` | 10 | Max concurrent tool executions |
| `max_retries_on_error` | 3 | Retries for transient LLM errors |

## Architecture Overview

```
User Input
    │
    ▼
AgentLoop (sai-core/services/agent_loop.rs)
    │
    ├──► LlmPort.chat_stream()  ──► sai-llm (genai adapter)
    │
    ├──► ToolRegistryPort.get() ──► sai-tools (built-in tools)
    │
    ├──► PermissionPort.check() ──► sai-permissions
    │
    └──► UiPort.emit_event()    ──► sai-tui (terminal display)
```

The agent loop lives in the domain layer (`sai-core`) and accesses
all external concerns through port traits. It can be tested with
mock implementations of all ports.
