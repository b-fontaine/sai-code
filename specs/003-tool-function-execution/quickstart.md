# Quickstart: Tool & Function Execution

**Feature**: 003-tool-function-execution
**Date**: 2026-04-05

## Prerequisites

- Rust toolchain >= 1.80.0
- cargo-nextest installed (`cargo install cargo-nextest`)
- Project cloned and on branch `003-tool-function-execution`

## Setup

1. The `sai-tools` crate should be added to the workspace `Cargo.toml`:
   ```toml
   [workspace]
   members = ["crates/sai-core", "crates/sai-llm", "crates/sai-tools"]
   ```

2. Create `crates/sai-tools/Cargo.toml` with dependencies on `sai-core` (workspace), `tokio`, `async-trait`, `serde`, `serde_json`, `thiserror`, `globset`, `ignore`, `grep-regex`, `grep-searcher`, `tree-sitter`, `tree-sitter-bash`.

3. Build the workspace:
   ```sh
   cargo build
   ```

## Running Tests

```sh
# All tests
cargo nextest run -p sai-tools

# Specific tool
cargo nextest run -p sai-tools file_read

# With output
cargo nextest run -p sai-tools -- --nocapture
```

## Quick Verification

After implementing a tool, verify it works end-to-end:

```sh
# Unit tests pass
cargo nextest run -p sai-tools

# Clippy clean
cargo clippy -p sai-tools -- -D warnings

# Format check
cargo fmt -p sai-tools -- --check
```

## Implementation Order

Follow the user story priorities from the spec:

1. **P1**: `FileReadTool` — simplest tool, read-only, establishes the pattern
2. **P1**: `ShellTool` — enables build/test workflows
3. **P2**: `FileWriteTool` + `FileEditTool` — mutating file operations
4. **P2**: `GrepTool` + `GlobTool` — codebase search
5. **P3**: `InMemoryToolRegistry` — wires tools together

Each tool should be test-driven: write a failing test, implement, refactor.

## Key Patterns

- Each tool is a struct implementing `ToolPort` from `sai-core`
- Tools receive `ToolConfig` at construction for shared settings
- Input is `serde_json::Value`, deserialized into the tool's input struct
- Output is `ToolOutput::Success(String)` or `ToolOutput::Error(String)`
- Errors use `ToolError` variants from `sai-core`
