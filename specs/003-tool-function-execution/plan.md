# Implementation Plan: Tool & Function Execution

**Branch**: `003-tool-function-execution` | **Date**: 2026-04-05 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/003-tool-function-execution/spec.md`

## Summary

Implement the concrete built-in tools (file read, file write, file edit, grep, glob, shell execution) that the agent uses to interact with the developer's project. Each tool implements the `ToolPort` trait defined in `sai-core`, lives in the `sai-tools` adapter crate, and integrates with the existing `ToolExecutor` and `PermissionPort` infrastructure.

## Technical Context

**Language/Version**: Rust 2021 edition, MSRV 1.80.0
**Primary Dependencies**: tokio (async), serde/serde_json (serialization), thiserror (errors), async-trait, globset (glob matching), grep-regex/grep-searcher (content search)
**Storage**: Local filesystem (read/write via `tokio::fs`)
**Testing**: cargo-nextest, mockall for port mocks, tempfile for filesystem tests
**Target Platform**: macOS, Linux (POSIX shell); Windows support deferred
**Project Type**: Library crate (adapter) consumed by `sai-cli`
**Performance Goals**: File read <1s for 1MB files, search <2s for 100k-file projects, shell timeout enforcement within 1s of deadline
**Constraints**: All tool execution gated by PermissionPort; fail-closed on permission errors; output truncation at configurable limit
**Scale/Scope**: 6 built-in tools, extensible registry for custom tools

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Hexagonal Architecture | PASS | Tools live in `sai-tools` adapter crate, implement `ToolPort` from `sai-core`. Zero domain logic in adapter. |
| II. Multi-Provider LLM Abstraction | N/A | This feature does not touch LLM providers. |
| III. Test-First Development | PASS | Each tool will follow Red-Green-Refactor. Unit tests with mocked filesystem, integration tests with real tempdir. |
| IV. Type-Safe Domain Modeling | PASS | Tool errors use typed `ToolError` variants from `sai-core`. Input validation via JSON Schema. No `unsafe`. |
| V. Security by Default | PASS | All tools route through PermissionPort. Shell commands will use tree-sitter-bash for AST validation. File tools respect bypass-immune paths. Fail-closed default. |

No violations. No complexity tracking needed.

## Project Structure

### Documentation (this feature)

```text
specs/003-tool-function-execution/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── tool-contracts.md
└── tasks.md             # Phase 2 output (via /speckit.tasks)
```

### Source Code (repository root)

```text
crates/sai-tools/
├── Cargo.toml
├── src/
│   ├── lib.rs               # Public API: re-exports, InMemoryToolRegistry
│   ├── registry.rs          # InMemoryToolRegistry implements ToolRegistryPort
│   ├── file_read.rs         # FileReadTool implements ToolPort
│   ├── file_write.rs        # FileWriteTool implements ToolPort
│   ├── file_edit.rs         # FileEditTool implements ToolPort
│   ├── grep.rs              # GrepTool implements ToolPort
│   ├── glob.rs              # GlobTool implements ToolPort
│   └── shell.rs             # ShellTool implements ToolPort
└── tests/
    ├── file_tools_test.rs   # Integration tests for file read/write/edit
    ├── search_tools_test.rs # Integration tests for grep/glob
    └── shell_tool_test.rs   # Integration tests for shell execution
```

**Structure Decision**: New `sai-tools` adapter crate as defined in the constitution (Principle I). One module per tool for clear separation. `InMemoryToolRegistry` provides the `ToolRegistryPort` implementation.
