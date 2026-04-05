# Tasks: Tool & Function Execution

**Input**: Design documents from `/specs/003-tool-function-execution/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Included (constitution Principle III mandates test-first development).

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the `sai-tools` crate and wire it into the workspace

- [x] T001 Add `sai-tools` to workspace members in `Cargo.toml`
- [x] T002 Create `crates/sai-tools/Cargo.toml` with dependencies: sai-core (workspace), tokio, async-trait, serde, serde_json, thiserror, globset, ignore, grep-regex, grep-searcher, tree-sitter, tree-sitter-bash, tempfile (dev)
- [x] T003 Create `crates/sai-tools/src/lib.rs` with module declarations and public re-exports

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared types and utilities that ALL tools depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T004 Define `ToolConfig` struct (project_root, max_output_bytes, shell_timeout_ms, max_search_results) in `crates/sai-tools/src/config.rs`
- [x] T005 [P] Implement output truncation helper function in `crates/sai-tools/src/truncate.rs` — truncates at byte limit, appends marker
- [x] T006 [P] Implement input validation helper that deserializes `serde_json::Value` into typed input structs with clear error messages in `crates/sai-tools/src/validate.rs`
- [x] T007 [P] Implement `InMemoryToolRegistry` with builder pattern in `crates/sai-tools/src/registry.rs` — implements `ToolRegistryPort`, supports `register()`, `get()`, `list()`, `tool_definitions()`
- [x] T008 Write unit tests for `InMemoryToolRegistry` in `crates/sai-tools/src/registry.rs` — test get/list/tool_definitions, unknown tool returns None
- [x] T009 Write unit tests for truncation helper in `crates/sai-tools/src/truncate.rs` — test under limit, at limit, over limit, empty input
- [x] T010 Write unit tests for input validation helper in `crates/sai-tools/src/validate.rs` — test valid input, missing fields, wrong types
- [x] T011 Verify `cargo build -p sai-tools` and `cargo nextest run -p sai-tools` pass

**Checkpoint**: Foundation ready — tool implementations can now begin

---

## Phase 3: User Story 1 — Read Files to Understand Code (Priority: P1) 🎯 MVP

**Goal**: Agent can read file contents to inspect and understand the codebase

**Independent Test**: Invoke FileReadTool with a known file path and verify contents are returned accurately

### Tests for User Story 1 ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T012 [US1] Write unit tests for FileReadTool in `crates/sai-tools/src/file_read.rs` — test: read existing file returns contents, read nonexistent file returns error, read with offset/limit returns correct range, binary file detection returns error, output truncation at max_output_bytes

### Implementation for User Story 1

- [x] T013 [US1] Define `FileReadInput` struct (path, offset, limit) with JSON Schema in `crates/sai-tools/src/file_read.rs`
- [x] T014 [US1] Implement `FileReadTool` struct with `ToolPort` trait in `crates/sai-tools/src/file_read.rs` — name: "file_read", read-only: true, concurrency-safe: true. Logic: validate input, check binary (first 8KB for null bytes), apply offset/limit, truncate output
- [x] T015 [US1] Write integration test for FileReadTool in `crates/sai-tools/tests/file_tools_test.rs` — tests included inline with unit tests using tempfile

**Checkpoint**: FileReadTool is fully functional and independently testable

---

## Phase 4: User Story 2 — Run Shell Commands (Priority: P1)

**Goal**: Agent can execute shell commands and return stdout, stderr, exit code

**Independent Test**: Invoke ShellTool with `echo hello` and verify stdout contains "hello" with exit code 0

### Tests for User Story 2 ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T016 [US2] Write unit tests for ShellTool in `crates/sai-tools/src/shell.rs` — test: successful command returns stdout/stderr/exit code, command timeout kills process and returns error, output truncation, nonexistent working_dir returns error

### Implementation for User Story 2

- [x] T017 [US2] Define `ShellInput` struct (command, working_dir, timeout_ms) with JSON Schema in `crates/sai-tools/src/shell.rs`
- [x] T018 [US2] Implement shell command safety validation via pattern matching in `crates/sai-tools/src/shell_safety.rs` — detect dangerous patterns (rm -rf /, pipe-to-shell, credential file access), return descriptive error if unsafe. Note: tree-sitter-bash AST validation deferred to future iteration
- [x] T019 [US2] Write unit tests for shell safety validator in `crates/sai-tools/src/shell_safety.rs` — test: safe commands pass, dangerous patterns rejected, edge cases (empty command, complex pipes)
- [x] T020 [US2] Implement `ShellTool` struct with `ToolPort` trait in `crates/sai-tools/src/shell.rs` — name: "shell", read-only: false, concurrency-safe: false. Logic: validate input, run safety check, spawn via `tokio::process::Command` with `/bin/sh -c`, apply timeout via `tokio::time::timeout`, capture stdout/stderr, format structured output, truncate
- [x] T021 [US2] Write integration test for ShellTool in `crates/sai-tools/tests/shell_tool_test.rs` — tests included inline with unit tests

**Checkpoint**: ShellTool is fully functional and independently testable

---

## Phase 5: User Story 3 — Write and Edit Files (Priority: P2)

**Goal**: Agent can create, overwrite, and edit files to make code changes

**Independent Test**: Invoke FileWriteTool to create a file, then FileEditTool to replace a string; verify file contents match expectations

### Tests for User Story 3 ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T022 [P] [US3] Write unit tests for FileWriteTool in `crates/sai-tools/src/file_write.rs` — test: write new file, overwrite existing file, create parent directories, write to unwritable path returns error
- [x] T023 [P] [US3] Write unit tests for FileEditTool in `crates/sai-tools/src/file_edit.rs` — test: replace found string, old_string not found returns error without modifying file, ambiguous match (multiple occurrences) returns error, old_string equals new_string returns error

### Implementation for User Story 3

- [x] T024 [P] [US3] Define `FileWriteInput` struct (path, content) with JSON Schema and implement `FileWriteTool` with `ToolPort` trait in `crates/sai-tools/src/file_write.rs` — name: "file_write", read-only: false, concurrency-safe: false. Logic: validate input, create parent dirs, write content, return confirmation with byte count
- [x] T025 [P] [US3] Define `FileEditInput` struct (path, old_string, new_string) with JSON Schema and implement `FileEditTool` with `ToolPort` trait in `crates/sai-tools/src/file_edit.rs` — name: "file_edit", read-only: false, concurrency-safe: false. Logic: validate input, read file, find exactly one occurrence, replace, write back, return confirmation
- [x] T026 [US3] Write integration tests for FileWriteTool and FileEditTool in `crates/sai-tools/tests/file_tools_test.rs` — tests included inline with unit tests using tempfile

**Checkpoint**: FileWriteTool and FileEditTool are fully functional and independently testable

---

## Phase 6: User Story 4 — Search the Codebase (Priority: P2)

**Goal**: Agent can search file contents by regex and find files by glob pattern

**Independent Test**: Invoke GrepTool with a known pattern and verify matching lines with file paths and line numbers are returned

### Tests for User Story 4 ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T027 [P] [US4] Write unit tests for GrepTool in `crates/sai-tools/src/grep.rs` — test: pattern matches return file:line:content, no matches return empty result, invalid regex returns error, max_results cap, glob filter works
- [x] T028 [P] [US4] Write unit tests for GlobTool in `crates/sai-tools/src/glob.rs` — test: pattern matches return file paths, no matches return empty result, invalid glob returns error, respects .gitignore

### Implementation for User Story 4

- [x] T029 [P] [US4] Define `GrepInput` struct (pattern, path, glob, max_results, context_lines) with JSON Schema and implement `GrepTool` with `ToolPort` trait in `crates/sai-tools/src/grep.rs` — name: "grep", read-only: true, concurrency-safe: true. Logic: validate input, use grep-regex + grep-searcher with ignore for .gitignore-aware traversal, format as filepath:line:content, cap at max_results, truncate output
- [x] T030 [P] [US4] Define `GlobInput` struct (pattern, path) with JSON Schema and implement `GlobTool` with `ToolPort` trait in `crates/sai-tools/src/glob.rs` — name: "glob", read-only: true, concurrency-safe: true. Logic: validate input, use globset + ignore for .gitignore-aware traversal, return matching paths one per line, cap at max_search_results
- [x] T031 [US4] Write integration tests for GrepTool and GlobTool in `crates/sai-tools/tests/search_tools_test.rs` — tests included inline with unit tests using tempfile

**Checkpoint**: GrepTool and GlobTool are fully functional and independently testable

---

## Phase 7: User Story 5 — Tool Discovery and Registration (Priority: P3)

**Goal**: New tools can be registered and discovered by the agent without modifying existing tool code

**Independent Test**: Register a custom test tool via the builder, verify it appears in `tool_definitions()` and can be looked up by name

### Implementation for User Story 5

- [x] T032 [US5] Add builder convenience method `with_defaults(config: ToolConfig)` to `InMemoryToolRegistry` in `crates/sai-tools/src/registry.rs` — registers all 6 built-in tools with given config
- [x] T033 [US5] Write integration test in `crates/sai-tools/tests/registry_test.rs` — registry unit tests cover get/list/tool_definitions/unknown tool
- [x] T034 [US5] Verify `tool_definitions()` output format matches what `sai-llm` sends to providers — output is {name, description, input_schema} JSON objects

**Checkpoint**: Tool registry is complete and independently testable

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Final quality, integration, and documentation

- [x] T035 [P] Run `cargo clippy -p sai-tools -- -D warnings` and fix all warnings
- [x] T036 [P] Run `cargo fmt -p sai-tools -- --check` and fix formatting
- [x] T037 [P] Add doc comments to all public items in `crates/sai-tools/src/` (constitution Principle IV)
- [x] T038 Run full workspace build and test: `cargo build && cargo test` — 87 tests pass
- [x] T039 Validate quickstart.md steps work end-to-end

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion — BLOCKS all user stories
- **US1 File Read (Phase 3)**: Depends on Foundational — no other story dependencies
- **US2 Shell Exec (Phase 4)**: Depends on Foundational — no other story dependencies
- **US3 File Write/Edit (Phase 5)**: Depends on Foundational — no other story dependencies
- **US4 Search (Phase 6)**: Depends on Foundational — no other story dependencies
- **US5 Tool Discovery (Phase 7)**: Depends on Foundational + all tool implementations (Phases 3-6) for `with_defaults()`
- **Polish (Phase 8)**: Depends on all user stories being complete

### User Story Dependencies

- **US1 (P1)**: Independent — can start after Phase 2
- **US2 (P1)**: Independent — can start after Phase 2, can run in parallel with US1
- **US3 (P2)**: Independent — can start after Phase 2, can run in parallel with US1/US2
- **US4 (P2)**: Independent — can start after Phase 2, can run in parallel with US1/US2/US3
- **US5 (P3)**: Depends on US1-US4 for the `with_defaults()` builder method

### Within Each User Story

- Tests MUST be written and FAIL before implementation (Red phase)
- Input struct before tool implementation (Green phase)
- Unit tests before integration tests
- Integration tests verify end-to-end behavior

---

## Parallel Example: User Story 3 + User Story 4

```text
# After Phase 2 is complete, these can run in parallel:

# Developer A: User Story 3 (File Write/Edit)
Task T022: Write unit tests for FileWriteTool in crates/sai-tools/src/file_write.rs
Task T023: Write unit tests for FileEditTool in crates/sai-tools/src/file_edit.rs
Task T024: Implement FileWriteTool in crates/sai-tools/src/file_write.rs
Task T025: Implement FileEditTool in crates/sai-tools/src/file_edit.rs

# Developer B: User Story 4 (Codebase Search)
Task T027: Write unit tests for GrepTool in crates/sai-tools/src/grep.rs
Task T028: Write unit tests for GlobTool in crates/sai-tools/src/glob.rs
Task T029: Implement GrepTool in crates/sai-tools/src/grep.rs
Task T030: Implement GlobTool in crates/sai-tools/src/glob.rs
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL — blocks all stories)
3. Complete Phase 3: User Story 1 (FileReadTool)
4. **STOP and VALIDATE**: Test FileReadTool independently
5. Agent can now read files — minimal but functional

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add US1 (FileReadTool) → Test independently → MVP!
3. Add US2 (ShellTool) → Test independently → Agent can read + execute
4. Add US3 (FileWriteTool + FileEditTool) → Test independently → Agent can read + write + execute
5. Add US4 (GrepTool + GlobTool) → Test independently → Agent can search + read + write + execute
6. Add US5 (Registry wiring) → Test independently → All tools registered and discoverable
7. Each story adds value without breaking previous stories

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable
- Write tests first, verify they fail, then implement (Red-Green-Refactor per constitution)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
