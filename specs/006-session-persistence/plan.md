# Implementation Plan: Session Persistence

**Branch**: `006-session-persistence` | **Date**: 2026-04-08 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `specs/006-session-persistence/spec.md`

## Summary

Users want to save conversations to disk and resume them in a future session. The agent
currently starts fresh on every invocation (explicitly deferred in step 005). This plan
adds a new `SessionPort` trait in `sai-core`, a `FilesystemSessionAdapter` in a new
`sai-session` crate, and new CLI flags/subcommands in `sai-cli`. Conversations are
auto-saved to `~/.local/share/sai/sessions/` as append-only JSONL (turns) + atomic JSON
(metadata) per session. Resuming reconstructs the message history and injects it into
`AgentLoop` before the first turn.

## Technical Context

**Language/Version**: Rust 2021 edition, MSRV 1.80.0
**Primary Dependencies**: tokio (workspace), serde/serde_json (workspace), uuid v1 (workspace), async-trait (workspace), thiserror (workspace), dirs v5 (NEW), chrono v0.4 with serde feature (NEW)
**Storage**: Local filesystem — `~/.local/share/sai/sessions/` (Linux) / `~/Library/Application Support/sai/sessions/` (macOS) via `dirs::data_dir()`
**Testing**: cargo-nextest, mockall (workspace) for `MockSessionPort`; contract test suite in `sai-session/tests/`
**Target Platform**: macOS and Linux (POSIX file operations, `rename` atomicity)
**Project Type**: CLI application with hexagonal architecture
**Performance Goals**: Resume any of 100 sessions in <2s; list 500 sessions in <1s
**Constraints**: MSRV 1.80.0; no `unsafe`; session files at `0600`, dirs at `0700`
**Scale/Scope**: ≤500 sessions, ≤200 turns/session, single-writer per session

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Hexagonal Architecture | ✅ PASS | `SessionPort` trait in `sai-core`; `FilesystemSessionAdapter` in new `sai-session` crate; wired in `sai-cli`. `sai-core` gains no I/O dependency. |
| II. Multi-Provider LLM | ✅ PASS | Not affected. Session persistence is orthogonal to LLM provider. |
| III. Test-First Development | ✅ PASS | `MockSessionPort` via `mockall::automock`; contract tests required for `FilesystemSessionAdapter`; TDD cycle enforced in task ordering. |
| IV. Type-Safe Domain Modeling | ✅ PASS | `SessionError` uses `thiserror`; `SessionMeta`, `ConversationTurn`, `PersistedSession` fully typed; all domain types `Serialize/Deserialize`. |
| V. Security by Default | ✅ PASS (with requirement) | Session files MUST be created with mode `0600`; directories `0700`. Conversations may contain sensitive data. Implemented via `std::os::unix::fs::PermissionsExt`. |

**New crate justification** (`sai-session`): The filesystem I/O adapter has a clear
infrastructure boundary. Placing it in `sai-cli` would violate hexagonal architecture
and make the adapter untestable in isolation. This satisfies the constitution's
requirement for clear domain boundary justification.

**Post-design re-check**: ✅ Data model and contracts (see `data-model.md`,
`contracts/session-port-contract.md`) confirm no infrastructure leaks into `sai-core`.

## Project Structure

### Documentation (this feature)

```text
specs/006-session-persistence/
├── plan.md                              # This file
├── spec.md                              # Feature specification
├── research.md                          # Phase 0 output
├── data-model.md                        # Phase 1 output
├── quickstart.md                        # Phase 1 output
├── contracts/
│   ├── cli-contract.md                  # CLI flags and subcommands
│   └── session-port-contract.md         # SessionPort behavioral contract
├── checklists/
│   └── requirements.md                  # Spec quality checklist
└── tasks.md                             # Phase 2 output (/speckit.tasks)
```

### Source Code Layout (additions and modifications)

```text
crates/
├── sai-core/src/
│   ├── domain/
│   │   └── session.rs          ← NEW: SessionMeta, ConversationTurn, PersistedSession
│   ├── ports/
│   │   ├── mod.rs              ← MODIFIED: add session module
│   │   └── session.rs          ← NEW: SessionPort trait + NoOpSessionPort
│   ├── error.rs                ← MODIFIED: add SessionError
│   ├── lib.rs                  ← MODIFIED: re-export new types
│   └── services/
│       └── agent_loop.rs       ← MODIFIED: add session_port param, save_turn after each turn
│
├── sai-session/                ← NEW CRATE
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              ← pub use FilesystemSessionAdapter
│       ├── adapter.rs          ← FilesystemSessionAdapter (implements SessionPort)
│       └── error.rs            ← I/O error → SessionError conversions
│
└── sai-cli/src/
    ├── cli.rs                  ← MODIFIED: add --resume, --session-name; add sessions subcommand
    ├── repl.rs                 ← MODIFIED: wire FilesystemSessionAdapter; pass to AgentLoop
    ├── sessions.rs             ← NEW: list, show, delete session commands
    └── main.rs                 ← MODIFIED: dispatch sessions subcommand

Cargo.toml (workspace root)     ← MODIFIED: add sai-session member, dirs, chrono deps
```

**Structure Decision**: Rust workspace (Option 1 variant). One new crate (`sai-session`)
added to the workspace. All other changes are modifications to existing crates. The new
crate follows the existing adapter-crate naming convention (`sai-*`).

## Key Files to Read During Implementation

| File | Reason |
|------|--------|
| `crates/sai-core/src/services/agent_loop.rs` | Add `session_port` parameter and `save_turn` call |
| `crates/sai-core/src/domain/` (all files) | Follow existing entity patterns (derive macros, doc comments) |
| `crates/sai-core/src/ports/` (all files) | Follow existing port trait patterns (`#[async_trait]`, `mockall::automock`) |
| `crates/sai-core/src/error.rs` | Add `SessionError` following existing `thiserror` patterns |
| `crates/sai-cli/src/cli.rs` | Clap derive structure to extend with new args/subcommand |
| `crates/sai-cli/src/repl.rs` | Port wiring pattern to replicate for `SessionPort` |
| `crates/sai-tui/src/adapters/ui.rs` | Reference for adapter struct + port impl boilerplate |
| `specs/005-ratatui-tui/plan.md` | Prior plan for structural reference |

## Verification

After implementation is complete, verify end-to-end with the following steps:

1. **Build**: `cargo build -p sai-cli` — must compile with no warnings
2. **Tests**: `cargo nextest run --workspace` — all tests must pass
3. **Lint**: `cargo clippy --workspace` — no warnings
4. **Format**: `cargo fmt --check` — no diff
5. **Contract tests**: `cargo nextest run -p sai-session` — all contract tests pass

**Manual verification**:
```sh
# Start a session, exchange 3 messages, exit
sai-code --session-name test-session

# List sessions — should show test-session
sai-code sessions list

# Resume and verify context
sai-code --resume test-session

# Kill mid-response (open second terminal, kill -9 the process)
# Resume — all completed turns still present
sai-code --resume test-session

# Delete
sai-code sessions delete test-session
sai-code sessions list  # should no longer show it
```
