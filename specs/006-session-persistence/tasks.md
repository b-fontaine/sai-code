# Tasks: Session Persistence

**Input**: Design documents from `specs/006-session-persistence/`
**Prerequisites**: plan.md ✅ spec.md ✅ research.md ✅ data-model.md ✅ contracts/ ✅ quickstart.md ✅

**Tests**: Included — Constitution Principle III mandates Test-First Development (TDD).
Write each test task first; verify it FAILS before writing implementation.

**Organization**: Tasks are grouped by user story to enable independent implementation
and testing. US3 (Auto-Save, P1) is sequenced before US1 (Resume, P1) because resuming
requires saved data.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: User story this task belongs to (US1–US5)
- **Tests**: Write FIRST, confirm FAIL, then implement

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Workspace-level changes that enable the new crate and dependencies.

- [x] T001 Add `dirs = "~5"` and `chrono = { version = "~0.4", features = ["serde"] }` to `[workspace.dependencies]` in `Cargo.toml`
- [x] T002 Create `crates/sai-session/` crate skeleton: `Cargo.toml`, `src/lib.rs` (empty `pub use`), add `sai-session` to workspace members in root `Cargo.toml`
- [x] T003 [P] Verify workspace compiles cleanly: `cargo check --workspace`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core domain types, port trait, and `AgentLoop` integration that ALL user
stories depend on. No user story can begin until this phase is complete.

**⚠️ CRITICAL**: Complete this phase before starting any user story.

### Domain Types

- [x] T004 [P] Add `SessionError` variants (`NotFound`, `NameConflict`, `Corrupted`, `Io`, `Serialization`) to `crates/sai-core/src/error.rs` using `thiserror`
- [x] T005 [P] Create `crates/sai-core/src/domain/session.rs`: define `SessionMeta`, `ConversationTurn`, `PersistedSession` structs with full doc comments, `#[derive(Debug, Clone, Serialize, Deserialize)]`, and field-level validation comments per `data-model.md`
- [x] T006 Update `crates/sai-core/src/domain/mod.rs` to `pub mod session` and re-export `SessionMeta`, `ConversationTurn`, `PersistedSession`
- [x] T007 Update `crates/sai-core/src/lib.rs` to re-export `SessionError` alongside existing error types

### Port Trait

- [x] T008 Write unit tests for `NoOpSessionPort` in `crates/sai-core/src/ports/session.rs` (all methods return `Ok`, list returns `vec![]`, load returns `None`) — confirm tests FAIL before T009
- [x] T009 Create `crates/sai-core/src/ports/session.rs`: define `SessionPort` trait (6 methods, `#[async_trait]`, `Send + Sync`, `#[cfg_attr(test, mockall::automock)]`) and `NoOpSessionPort` struct implementing it; pass T008 tests
- [x] T010 Update `crates/sai-core/src/ports/mod.rs` to `pub mod session` and re-export `SessionPort`, `NoOpSessionPort`

### AgentLoop Integration

- [x] T011 Write failing unit tests in `crates/sai-core/src/services/agent_loop.rs` for: (a) `save_turn` called after each successful `run_turn`, (b) `create_session` called on first `run_turn`, (c) failed turns do NOT call `save_turn` — use `MockSessionPort`
- [x] T012 Update `AgentLoop::new()` in `crates/sai-core/src/services/agent_loop.rs` to accept `session: &'a dyn SessionPort` parameter; add `session_port` field; call `create_session` on first turn and `save_turn` after each successful turn; pass T011 tests
- [x] T013 [P] Update all callers of `AgentLoop::new()` in `crates/sai-cli/src/repl.rs` and `crates/sai-cli/src/main.rs` to pass `&NoOpSessionPort` (temporary, replaced in US3)
- [x] T014 [P] Verify full workspace compiles and existing tests still pass: `cargo nextest run --workspace`

**Checkpoint**: Foundation complete. Domain types, port trait, and AgentLoop integration
are in place. All user story phases can now begin.

---

## Phase 3: User Story 3 — Auto-Save During a Session (Priority: P1)

**Goal**: Every completed turn is automatically persisted to disk with no user action.
Crash recovery loses at most the turn in progress.

**Independent Test**: Start the agent, send 3 messages, kill with `SIGKILL`, re-launch
and verify the `~/.local/share/sai/sessions/` directory contains a session with 3 turns.

### Tests (write first, verify FAIL)

- [x] T015 [P] [US3] Write contract tests for `FilesystemSessionAdapter::create_session` in `crates/sai-session/tests/contract_tests.rs`: idempotent, creates directory with mode 0700, creates `meta.json` with mode 0600, `NameConflict` on duplicate name
- [x] T016 [P] [US3] Write contract tests for `FilesystemSessionAdapter::save_turn` in `crates/sai-session/tests/contract_tests.rs`: turn appended to `turns.jsonl`, `meta.json` updated atomically, `NotFound` if session missing, incomplete JSON in file does not appear after reload
- [x] T017 [P] [US3] Write integration test in `crates/sai-cli/tests/autosave_test.rs`: build binary, run one-turn session via stdin, verify session directory created and `turns.jsonl` has one valid JSON line

### Implementation

- [x] T018 [US3] Create `crates/sai-session/Cargo.toml` with deps: `sai-core`, `tokio` (fs + io-util features), `serde`, `serde_json`, `uuid`, `async-trait`, `thiserror`, `dirs`, `chrono`
- [x] T019 [US3] Implement `FilesystemSessionAdapter::new()` in `crates/sai-session/src/adapter.rs`: resolve base dir via `dirs::data_dir().unwrap_or_else(|| PathBuf::from("~")).join("sai/sessions")`; accept optional `SAI_SESSION_DIR` env var override
- [x] T020 [US3] Implement `FilesystemSessionAdapter::create_session()`: create `{base}/{id}/` dir with mode 0700, write `meta.json` atomically (temp file + rename) with mode 0600, no-op if already exists, `NameConflict` if name taken
- [x] T021 [US3] Implement `FilesystemSessionAdapter::save_turn()`: append one-line JSON to `{base}/{id}/turns.jsonl` (mode 0600 on first write), atomically rewrite `meta.json` with updated `turn_count` and `last_active_at`; pass T015 and T016
- [x] T022 [US3] Export `FilesystemSessionAdapter` from `crates/sai-session/src/lib.rs`; add `crates/sai-session` as dependency in `crates/sai-cli/Cargo.toml`
- [x] T023 [US3] Replace `NoOpSessionPort` with `FilesystemSessionAdapter` in `crates/sai-cli/src/repl.rs` — create adapter from `dirs::data_dir()` and inject into `AgentLoop::new()`; pass T017 integration test
- [x] T024 [US3] Run `cargo nextest run --workspace` and confirm all T015–T017 tests pass; verify `cargo clippy --workspace` clean

**Checkpoint**: Auto-save is working. Every completed turn is persisted. Crash-recovery
can be manually verified via `SIGKILL` test.

---

## Phase 4: User Story 1 — Resume a Previous Conversation (Priority: P1)

**Goal**: User can resume any saved session by providing `--resume [ID_OR_NAME]`.
Full conversation history is injected into `AgentLoop` before the first turn.

**Independent Test**: Run one-turn session, exit cleanly, re-launch with `--resume`,
verify agent opens with a "Resumed session…" banner and responds with awareness of the
prior message.

### Tests (write first, verify FAIL)

- [x] T025 [P] [US1] Write contract tests for `FilesystemSessionAdapter::load_session` in `crates/sai-session/tests/contract_tests.rs`: returns `None` for unknown ID, returns `Some` with correct turns after save, `Corrupted` on invalid JSON line, `Corrupted` on missing turns file
- [x] T026 [P] [US1] Write contract tests for `FilesystemSessionAdapter::find_by_name` in `crates/sai-session/tests/contract_tests.rs`: returns `None` for unknown name, returns matching `SessionMeta` for named session
- [x] T027 [US1] Write integration test in `crates/sai-cli/tests/resume_test.rs`: create session via stdin, exit, re-invoke with `--resume`, assert "Resumed session" banner on stderr and agent history contains prior messages

### Implementation

- [x] T028 [US1] Implement `FilesystemSessionAdapter::load_session()` in `crates/sai-session/src/adapter.rs`: read `meta.json`, stream-parse `turns.jsonl` line-by-line, return `Corrupted` on bad JSON, reconstruct `PersistedSession`; pass T025
- [x] T029 [US1] Implement `FilesystemSessionAdapter::find_by_name()` in `crates/sai-session/src/adapter.rs`: scan session dirs, read each `meta.json`, match on name field; pass T026
- [x] T030 [P] [US1] Add `--resume [SESSION_ID]` and `--session-name <NAME>` args to `Cli` struct in `crates/sai-cli/src/cli.rs` using Clap derive; validate `session-name` matches `[a-zA-Z0-9_-]+`
- [x] T031 [US1] Add `load_session_for_resume()` helper in `crates/sai-cli/src/repl.rs`: resolves `--resume` (explicit ID, by name, or most-recent-in-cwd), calls `adapter.load_session()`, reconstructs `Vec<Message>` by flattening turns, prints banner to stderr; returns `SessionResumeResult` enum
- [x] T032 [US1] Update `AgentSession::new()` in `crates/sai-core/src/domain/session.rs` (or `AgentLoop::new()`) to accept optional pre-loaded `Vec<Message>` for resume; inject as initial `messages` before first `run_turn`
- [x] T033 [US1] Wire resume flow in `crates/sai-cli/src/repl.rs`: if `--resume` present, call `load_session_for_resume()`, inject messages; if `--session-name` present, set name in `SessionMeta`; pass T027
- [x] T034 [US1] Run `cargo nextest run --workspace`; confirm T025–T027 pass; verify `cargo clippy` clean

**Checkpoint**: Full save + resume cycle works end-to-end. US1 and US3 together satisfy
the P1 requirements.

---

## Phase 5: User Story 2 — List Available Sessions (Priority: P2)

**Goal**: `sai-code sessions list` outputs a table of saved sessions ordered by most
recent activity.

**Independent Test**: Create 3 sessions via `repl.run()` in tests, call `sessions list`,
verify output contains all 3 with correct metadata columns.

### Tests (write first, verify FAIL)

- [x] T035 [P] [US2] Write contract test for `FilesystemSessionAdapter::list_sessions` in `crates/sai-session/tests/contract_tests.rs`: returns empty vec when no sessions, returns all sessions sorted by `last_active_at` descending, corrupted session omitted with tracing warning
- [x] T036 [P] [US2] Write integration test in `crates/sai-cli/tests/sessions_list_test.rs`: create 2 sessions, run `sai-code sessions list`, assert both appear in stdout with expected columns; run with piped output and assert JSON array

### Implementation

- [x] T037 [US2] Implement `FilesystemSessionAdapter::list_sessions()` in `crates/sai-session/src/adapter.rs`: scan base dir entries, read each `meta.json`, skip unreadable with `tracing::warn!`, sort by `last_active_at` descending; pass T035
- [x] T038 [P] [US2] Add `sessions` subcommand to `Cli` in `crates/sai-cli/src/cli.rs` with `list`, `show`, and `delete` sub-subcommands; `list` accepts `--dir <PATH>` and `--limit <N>` options
- [x] T039 [US2] Create `crates/sai-cli/src/sessions.rs`: implement `cmd_list()` — format table with columns ID, NAME, TURNS, LAST ACTIVE (human-relative via `chrono`), DIR (home-relative path); JSON output when stdout is not a terminal
- [x] T040 [US2] Implement `cmd_show()` in `crates/sai-cli/src/sessions.rs`: load full session, print metadata block + turn-by-turn summary (truncate user messages at 80 chars)
- [x] T041 [US2] Dispatch `sessions` subcommand in `crates/sai-cli/src/main.rs`: call `sessions::cmd_list()`, `sessions::cmd_show()`, or `sessions::cmd_delete()` as appropriate
- [x] T042 [US2] Pass T035 and T036 tests; run `cargo nextest run --workspace`

**Checkpoint**: `sessions list` and `sessions show` are fully functional.

---

## Phase 6: User Story 4 — Name a Session (Priority: P3)

**Goal**: `--session-name NAME` assigns a human-readable name to a session at launch
time; named sessions are resumable by name.

**Independent Test**: Launch with `--session-name my-task`, exit, run `sessions list`,
verify "my-task" appears; re-launch with `--resume my-task` and verify correct session
loads.

### Tests (write first, verify FAIL)

- [x] T043 [P] [US4] Write integration test in `crates/sai-cli/tests/session_name_test.rs`: launch with `--session-name foo`, exit, list sessions and assert "foo" appears, resume with `--resume foo` and assert correct session loads
- [x] T044 [P] [US4] Write test for name conflict: create session named "foo", create another with `--session-name foo`, assert name becomes "foo-2" (warning on stderr)

### Implementation

- [x] T045 [US4] Add name uniqueness check in `FilesystemSessionAdapter::create_session()`: if `meta.name` is `Some` and a session with that name already exists, find next available suffix (`-2`, `-3`, …) and update `meta.name`; emit `tracing::warn!`
- [x] T046 [US4] Pass T043 and T044; run `cargo nextest run --workspace`

**Checkpoint**: Named sessions work — launch, list, and resume by name all function.

---

## Phase 7: User Story 5 — Delete Sessions (Priority: P3)

**Goal**: `sai-code sessions delete SESSION_ID` removes a session; `--all` deletes all
with confirmation.

**Independent Test**: Create a session, delete it by name, verify `list` no longer shows
it and the session directory is gone from disk.

### Tests (write first, verify FAIL)

- [x] T047 [P] [US5] Write contract test for `FilesystemSessionAdapter::delete_session` in `crates/sai-session/tests/contract_tests.rs`: returns `true` and removes dir for known session, returns `false` for unknown session, subsequent `load_session` returns `None`
- [x] T048 [P] [US5] Write integration test in `crates/sai-cli/tests/sessions_delete_test.rs`: create session, delete by UUID, verify not in `list` output; attempt to resume deleted session and assert exit code 1

### Implementation

- [x] T049 [US5] Implement `FilesystemSessionAdapter::delete_session()` in `crates/sai-session/src/adapter.rs`: remove `{base}/{id}/` directory tree; return `true` if existed, `false` if not; pass T047
- [x] T050 [US5] Implement `cmd_delete()` in `crates/sai-cli/src/sessions.rs`: resolve by UUID or name, call `adapter.delete_session()`, print confirmation; for `--all`, count sessions, prompt for confirmation (require `y` from tty; exit 1 if stdin not terminal), delete all
- [x] T051 [US5] Pass T047 and T048; run `cargo nextest run --workspace`

**Checkpoint**: All five user stories are fully functional and independently testable.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [x] T052 [P] Verify all session files created during tests use mode 0600/0700: add `std::os::unix::fs::PermissionsExt` assertions to contract tests in `crates/sai-session/tests/contract_tests.rs`
- [x] T053 [P] Add `SAI_SESSION_DIR` env var support to `FilesystemSessionAdapter::new()` in `crates/sai-session/src/adapter.rs` if not already present; add unit test
- [x] T054 Run full verification sequence from plan.md: `cargo build -p sai-cli`, `cargo nextest run --workspace`, `cargo clippy --workspace`, `cargo fmt --check`
- [x] T055 Execute quickstart.md manual walkthrough end-to-end: create named session, list, show, resume (with crash recovery test), delete

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — BLOCKS all user stories
- **US3 Auto-Save (Phase 3)**: Depends on Phase 2
- **US1 Resume (Phase 4)**: Depends on Phase 3 (needs saved data to resume)
- **US2 List Sessions (Phase 5)**: Depends on Phase 2; can run in parallel with Phase 4
- **US4 Name (Phase 6)**: Depends on Phase 4 (resume by name) and Phase 5 (name in list)
- **US5 Delete (Phase 7)**: Depends on Phase 3 (sessions must exist to delete)
- **Polish (Phase 8)**: Depends on all user story phases

### User Story Dependencies

```
Phase 1 (Setup)
    └── Phase 2 (Foundation)
            ├── Phase 3 (US3 Auto-Save) ──────────────────┐
            │       └── Phase 4 (US1 Resume)              │
            │               └── Phase 6 (US4 Name)        ├── Phase 8 (Polish)
            ├── Phase 5 (US2 List Sessions) ──────────────┤
            │       └── Phase 6 (US4 Name)                │
            └── Phase 7 (US5 Delete) ────────────────────┘
```

### Within Each Phase

1. Test tasks — write FIRST, confirm they FAIL
2. Domain/model tasks (can run in parallel with each other)
3. Adapter/service tasks (depend on domain tasks)
4. CLI/integration tasks (depend on adapter tasks)

### Parallel Opportunities

- T004 and T005 (domain types) — parallel within Phase 2
- T015, T016, T017 (US3 test tasks) — parallel
- T025, T026 (US1 contract tests) — parallel with T027
- T035, T036 (US2 tests) — parallel
- T038, T037 (US2 list impl) — parallel
- T043, T044 (US4 tests) — parallel
- T047, T048 (US5 tests) — parallel
- Phase 5 and Phase 4 — parallel after Foundation complete

---

## Parallel Example: Phase 3 (US3 Auto-Save)

```
# Launch all test tasks together first (write & verify FAIL):
Task T015: Contract tests for create_session in crates/sai-session/tests/contract_tests.rs
Task T016: Contract tests for save_turn in crates/sai-session/tests/contract_tests.rs
Task T017: Integration test in crates/sai-cli/tests/autosave_test.rs

# Then parallel implementation setup:
Task T018: Create crates/sai-session/Cargo.toml
Task T019: Implement FilesystemSessionAdapter::new() in crates/sai-session/src/adapter.rs
```

---

## Implementation Strategy

### MVP First (US3 + US1 Only — P1 stories)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundation (CRITICAL — blocks all stories)
3. Complete Phase 3: US3 Auto-Save
4. Complete Phase 4: US1 Resume
5. **STOP and VALIDATE**: `cargo nextest run --workspace` + manual quickstart walkthrough
6. Users can now save and resume conversations — core value delivered

### Incremental Delivery

1. Setup + Foundation → build passes, AgentLoop accepts SessionPort
2. US3 Auto-Save → every session persisted, crash-safe
3. US1 Resume → full save + resume cycle works (**MVP complete**)
4. US2 List Sessions → session discovery and management
5. US4 + US5 → named sessions and deletion (power user features)

---

## Notes

- **TDD is mandatory** (Constitution Principle III): each test task must FAIL before its
  implementation task runs
- `[P]` tasks = different files, no dependencies on incomplete tasks in same phase
- File permissions (0600/0700) must be verified in contract tests, not just assumed
- `NoOpSessionPort` in Phase 2 keeps existing tests green while foundation is built
- `cargo nextest run --workspace` at each checkpoint confirms no regressions
- Total tasks: **55** (T001–T055)

---

## Task Count Summary

| Phase | Story | Tasks | Parallel |
|-------|-------|-------|---------|
| Phase 1 | Setup | 3 | 1 |
| Phase 2 | Foundation | 11 | 4 |
| Phase 3 | US3 Auto-Save (P1) | 10 | 3 |
| Phase 4 | US1 Resume (P1) | 10 | 3 |
| Phase 5 | US2 List Sessions (P2) | 8 | 2 |
| Phase 6 | US4 Name a Session (P3) | 4 | 2 |
| Phase 7 | US5 Delete Sessions (P3) | 5 | 2 |
| Phase 8 | Polish | 4 | 2 |
| **Total** | | **55** | **19** |
