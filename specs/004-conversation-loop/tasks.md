# Tasks: Interactive Conversation Loop

**Input**: Design documents from `/specs/004-conversation-loop/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/cli-contract.md, quickstart.md

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1–US7)
- Exact file paths are included in each description

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Scaffold the `sai-cli` binary crate and wire it into the workspace

- [x] T001 Add `crates/sai-cli` to workspace `Cargo.toml` members list
- [x] T002 Create `crates/sai-cli/Cargo.toml` with dependencies: sai-core, sai-llm, sai-tools, tokio (full), clap (v4 derive), color-eyre, tracing, tracing-subscriber (env-filter), tokio-util (sync)
- [x] T003 Create minimal `crates/sai-cli/src/main.rs` that initializes color-eyre, parses no args, and exits cleanly — verify `cargo build -p sai-cli` succeeds

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core structs and types shared across all user stories

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T004 Define `Cli` struct with clap derive in `crates/sai-cli/src/cli.rs` — `--model <MODEL>` flag (default `claude-sonnet-4`), `--verbose` flag; parse in `main.rs`
- [x] T005 [P] Define `ReplConfig` struct (prompt_prefix, farewell_message, double_ctrl_c_window_ms) in `crates/sai-cli/src/repl.rs`
- [x] T006 [P] Define `InputResult` enum (Message(String), Exit, Empty) in `crates/sai-cli/src/input.rs`

**Checkpoint**: Foundation ready — user story phases can now proceed

---

## Phase 3: User Story 1 — Start Agent and Send First Message (Priority: P1) 🎯 MVP

**Goal**: Launch the binary, display a banner, accept one user message, stream the response token-by-token to stdout, display a new prompt, and loop.

**Independent Test**: Run `ANTHROPIC_API_KEY=sk-... cargo run -p sai-cli`, type "What is 2+2?", verify streamed tokens appear, then a new prompt follows.

- [x] T007 [US1] Implement `display_banner(model: &str, dir: &Path)` in `crates/sai-cli/src/banner.rs` — prints `sai-code v{version}\nModel: {model}\nDirectory: {dir}` to stderr
- [x] T008 [US1] Implement `TerminalUi` struct (stdout + stderr handles) with `UiPort` in `crates/sai-cli/src/terminal_ui.rs` — handle `TextDelta` → write + flush stdout; `TurnComplete` → newline to stdout; `Error` → formatted message to stderr
- [x] T009 [US1] Implement `TerminalPermissions` struct with `PermissionPort` in `crates/sai-cli/src/terminal_permissions.rs` — read-only tools → Allow; non-interactive stdin → Deny; interactive → prompt "Allow {tool_name}? (y/n): " on stderr, read from /dev/tty
- [x] T010 [US1] Implement async line reader in `crates/sai-cli/src/input.rs` — `read_input(prompt: &str) -> Result<InputResult>` using `tokio::io::AsyncBufReadExt::read_line` on stdin; detect interactive mode via `std::io::IsTerminal`
- [x] T011 [US1] Implement REPL loop `run(config: ReplConfig, agent: AgentLoop) -> Result<()>` in `crates/sai-cli/src/repl.rs` — call `display_banner`, loop: read input → `agent.run_turn()` → display events → repeat
- [x] T012 [US1] Wire DI in `crates/sai-cli/src/main.rs` — create `AgentConfig` from CLI args, `GenaiLlmAdapter`, `InMemoryToolRegistry::with_defaults()`, `TerminalUi`, `TerminalPermissions`, construct `AgentLoop`, call `repl::run()`

**Checkpoint**: User Story 1 fully functional — binary launches, accepts input, streams response, loops

---

## Phase 4: User Story 2 — Multi-Turn Conversation with Context (Priority: P1)

**Goal**: Conversation history is preserved across all turns within a session; user sees a notice when history grows very large.

**Independent Test**: Send "list files in src/", then "read the first one" — verify the second response uses context from the first exchange.

- [x] T013 [US2] Add `HistorySizeWarning` handler to `TerminalUi` in `crates/sai-cli/src/terminal_ui.rs` — print notice "Note: conversation is getting long. Consider starting a new session." to stderr

*(Multi-turn context is preserved automatically by reusing the same `AgentLoop` instance across turns in the REPL loop from T011.)*

**Checkpoint**: Multi-turn context and history warning both work

---

## Phase 5: User Story 3 — Graceful Exit (Priority: P1)

**Goal**: All exit methods (/exit, /quit, Ctrl-D, Ctrl-C at prompt, double Ctrl-C during turn) terminate the process cleanly with exit code 0.

**Independent Test**: Start the agent, type `/exit`, verify the process terminates with exit code 0 and a farewell message.

- [x] T014 [US3] Add `/exit` and `/quit` command detection and EOF (None from read_line) to `read_input()` in `crates/sai-cli/src/input.rs` — return `InputResult::Exit` for these cases
- [x] T015 [US3] Handle `InputResult::Exit` in REPL loop in `crates/sai-cli/src/repl.rs` — print farewell message (from `ReplConfig`), break loop, return `Ok(())`
- [x] T016 [US3] Add Ctrl-C signal handling at input prompt via `tokio::select!` in REPL loop in `crates/sai-cli/src/repl.rs` — Ctrl-C at prompt → exit cleanly
- [x] T017 [US3] Implement `CancellationToken`-based turn cancellation in `crates/sai-cli/src/repl.rs` — create token per turn; 1st Ctrl-C during active turn → cancel token, return to prompt; 2nd Ctrl-C within `double_ctrl_c_window_ms` → exit process

**Checkpoint**: All four exit paths work and terminate with exit code 0

---

## Phase 6: User Story 4 — Activity Feedback During Processing (Priority: P2)

**Goal**: User sees "Thinking..." before first token, tool names during execution, and success/failure indicators after tool completion.

**Independent Test**: Ask the agent to read a file — verify "Thinking..." appears before tokens, tool name is displayed when it runs, and a status indicator follows.

- [x] T018 [P] [US4] Add `StreamStart` handler to `TerminalUi` in `crates/sai-cli/src/terminal_ui.rs` — print "Thinking..." to stderr
- [x] T019 [P] [US4] Add `ToolCallStart` handler to `TerminalUi` in `crates/sai-cli/src/terminal_ui.rs` — print "[tool: {name}]" to stderr
- [x] T020 [P] [US4] Add `ToolCallComplete` handler to `TerminalUi` in `crates/sai-cli/src/terminal_ui.rs` — print "✓" on success or "✗" on failure to stderr

**Checkpoint**: All activity feedback events are visible to the user

---

## Phase 7: User Story 5 — Error Recovery in Conversation (Priority: P2)

**Goal**: LLM and tool errors display a clear human-readable message; the session continues and all conversation history is preserved.

**Independent Test**: Simulate a network failure during a turn — verify the error is displayed clearly, then send another message and verify the session continues normally.

- [x] T021 [US5] Wrap `agent.run_turn()` in error handling in REPL loop in `crates/sai-cli/src/repl.rs` — match `AgentError::Llm(LlmError::Connection(_))` → "Connection error: {msg}. You can try again."; `RateLimited(_)` → "Rate limited. Please wait and try again."; `IterationLimitExceeded` → "Turn stopped: too many tool iterations."; any other error → print message; never break the loop on transient errors

**Checkpoint**: Errors display cleanly and the conversation loop continues after any error

---

## Phase 8: User Story 6 — Empty and Whitespace-Only Input Handling (Priority: P3)

**Goal**: Pressing Enter with no text (or only whitespace) re-displays the prompt without making any LLM call.

**Independent Test**: Press Enter with no text — verify the prompt reappears immediately with no delay or API activity.

- [x] T022 [US6] Trim input in `read_input()` in `crates/sai-cli/src/input.rs` — return `InputResult::Empty` for empty or whitespace-only strings; handle `InputResult::Empty` in REPL loop by re-displaying prompt without calling `run_turn()`

**Checkpoint**: Empty/whitespace input never triggers an LLM call

---

## Phase 9: User Story 7 — Initial Message via Command-Line Argument (Priority: P3)

**Goal**: Running `sai-code "fix the failing test"` processes that message as the first turn immediately, then enters interactive mode.

**Independent Test**: Run the agent with an inline message — verify the first turn executes immediately, then verify the agent enters interactive mode for follow-up.

- [x] T023 [US7] Add optional positional `message: Option<String>` argument to `Cli` struct in `crates/sai-cli/src/cli.rs`
- [x] T024 [US7] Process initial CLI message as first turn before entering the read loop in `crates/sai-cli/src/repl.rs` — if `initial_message.is_some()`, run it through `agent.run_turn()` before the interactive prompt loop

**Checkpoint**: Both `sai-code "prompt"` (immediate first turn) and `sai-code` (direct interactive mode) work correctly

---

## Phase 10: Polish & Cross-Cutting Concerns

**Purpose**: Verification, cleanup, and integration test coverage

- [x] T025 Write integration tests in `crates/sai-cli/tests/cli_integration_test.rs` using assert_cmd + predicates — test: binary launches and exits code 0 on EOF; `--help` flag exits 0; initial message via argument; empty input produces no output
- [x] T026 [P] Run `cargo clippy -p sai-cli -- -D warnings` and fix all warnings in `crates/sai-cli/src/`
- [x] T027 [P] Run `cargo fmt -p sai-cli -- --check` and fix formatting in `crates/sai-cli/src/`
- [x] T028 Run quickstart.md validation: `cargo build -p sai-cli`, clippy clean, format check, `cargo nextest run -p sai-cli`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — **blocks all user story phases**
- **US1 (Phase 3)**: Depends on Phase 2 — no dependencies on other stories
- **US2 (Phase 4)**: Depends on US1 (Phase 3) — adds one handler to TerminalUi
- **US3 (Phase 5)**: Depends on US1 (Phase 3) — extends input.rs and repl.rs
- **US4 (Phase 6)**: Depends on US1 (Phase 3) — adds handlers to TerminalUi (parallelizable with US2/US3)
- **US5 (Phase 7)**: Depends on US1 (Phase 3) — extends REPL error handling
- **US6 (Phase 8)**: Depends on Phase 2 (InputResult enum) — extends input.rs
- **US7 (Phase 9)**: Depends on Phase 2 (Cli struct) and US1 (repl.rs loop)
- **Polish (Phase 10)**: Depends on all desired user stories being complete

### Parallel Opportunities

Within each phase, tasks marked [P] can execute simultaneously.

After Phase 3 (US1) is complete, phases 4–9 can proceed in any order since they extend different aspects (TerminalUi handlers, input.rs, repl.rs error handling, cli.rs). Note that phases 4, 5, and 7 all touch `repl.rs` — coordinate to avoid merge conflicts.

---

## Parallel Example: User Story 4 (Activity Feedback)

All three TerminalUi handler tasks touch different match arms in the same file but can be drafted in parallel and merged:

```
Task T018: StreamStart handler → "Thinking..."
Task T019: ToolCallStart handler → "[tool: {name}]"
Task T020: ToolCallComplete handler → success/failure indicator
```

---

## Implementation Strategy

### MVP First (User Stories 1–3 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational — **critical, blocks all stories**
3. Complete Phase 3: User Story 1 — end-to-end streaming conversation
4. Complete Phase 4: User Story 2 — history warning (one task)
5. Complete Phase 5: User Story 3 — graceful exit
6. **STOP and VALIDATE**: Manual test of interactive conversation with clean exit
7. Deploy if ready — this is a working, usable agent

### Incremental Delivery

1. Setup + Foundational → crate compiles
2. US1 → basic interactive agent (MVP!)
3. US2 → history size warning
4. US3 → graceful exit
5. US4 → activity feedback (thinking, tool display)
6. US5 → error recovery
7. US6 → empty input guard
8. US7 → initial message from CLI arg
9. Polish → clippy, tests, verification

---

## Notes

- [P] tasks = different files or independent match arms, no sequential dependencies
- [Story] label maps each task to its user story for traceability
- `main.rs` is purely DI wiring — no business logic
- All response text goes to stdout; all metadata (banner, tools, errors, prompts) goes to stderr
- `PermissionPort` reads from `/dev/tty` for interactive prompts even when stdin is piped
- Use `CancellationToken` from `tokio-util` for cooperative turn cancellation
- Exit code 0 on all normal termination paths; non-zero only for unrecoverable startup errors
