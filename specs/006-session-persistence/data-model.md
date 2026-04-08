# Data Model: Session Persistence

## Overview

Session persistence adds three new domain types to `sai-core` and one new port trait.
All types are serializable. The `AgentSession` entity is augmented with an optional
persisted-session origin.

---

## New Domain Types (`crates/sai-core/src/domain/session.rs`)

### `SessionMeta`

Lightweight summary of a session, sufficient for the list view without loading full
turn data.

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | Unique session identifier (UUID v4) |
| `name` | `Option<String>` | User-provided display name; `None` if unnamed |
| `model_name` | `String` | Model active when the session was created |
| `working_dir` | `PathBuf` | Absolute path of the working directory at session start |
| `created_at` | `DateTime<Utc>` | When the session was first created |
| `last_active_at` | `DateTime<Utc>` | When the last turn was persisted |
| `turn_count` | `usize` | Number of completed turns saved |

**Constraints**:
- `name`, if present, MUST be non-empty and contain no path separator characters
- `working_dir` MUST be an absolute path
- `turn_count` MUST equal the number of turn records in the turns file

---

### `ConversationTurn`

A single completed exchange: the user's message, the AI's full response (including all
content blocks), and any tool interactions that occurred during that response.

| Field | Type | Description |
|-------|------|-------------|
| `turn_index` | `usize` | Zero-based position of this turn in the session |
| `user_message` | `String` | Verbatim text the user submitted |
| `messages` | `Vec<Message>` | All messages generated during this turn — the user `Message::User`, the assistant `Message::Assistant`, and any `Message::ToolResult` entries, in order |
| `completed_at` | `DateTime<Utc>` | When `run_turn` returned successfully |

**Constraints**:
- `messages` MUST begin with exactly one `Message::User` entry
- `messages` MUST contain at least one `Message::Assistant` entry
- A `ConversationTurn` record is only written after `run_turn` returns `Ok`; a turn
  that errored mid-stream is not persisted

---

### `PersistedSession`

The full, loadable representation of a saved session.

| Field | Type | Description |
|-------|------|-------------|
| `meta` | `SessionMeta` | Session metadata |
| `turns` | `Vec<ConversationTurn>` | All completed turns in order |

**Derived fields** (computed, not stored):
- Full message history: flatten `turns[*].messages` → `Vec<Message>` to inject into
  `AgentSession::messages` on resume
- Current `turn_count`: `turns.len()` (used to validate consistency with `meta.turn_count`)

---

## New Port Trait (`crates/sai-core/src/ports/session.rs`)

### `SessionPort`

```
SessionPort
├── create_session(meta: SessionMeta) → Result<(), SessionError>
├── save_turn(session_id: Uuid, turn: ConversationTurn) → Result<(), SessionError>
├── load_session(session_id: Uuid) → Result<Option<PersistedSession>, SessionError>
├── list_sessions() → Result<Vec<SessionMeta>, SessionError>
├── delete_session(session_id: Uuid) → Result<bool, SessionError>
│   (returns false if not found, true if deleted)
└── find_by_name(name: &str) → Result<Option<SessionMeta>, SessionError>
```

All methods are `async`. The trait is `Send + Sync`.

**Annotated with** `#[cfg_attr(test, mockall::automock)]` to generate `MockSessionPort`
for unit tests.

---

### `NoOpSessionPort`

A no-op implementation of `SessionPort` provided in `sai-core` for use in tests and
non-interactive mode. All methods return `Ok(())` or `Ok(None)`/`Ok(vec![])`.

---

## New Error Type (`crates/sai-core/src/error.rs` — addition)

### `SessionError`

| Variant | Description |
|---------|-------------|
| `NotFound { id: Uuid }` | Session with given ID does not exist |
| `NameConflict { name: String }` | A session with this name already exists |
| `Corrupted { id: Uuid, reason: String }` | Session data is unreadable or invalid |
| `Io(std::io::Error)` | Underlying I/O failure |
| `Serialization(serde_json::Error)` | JSON encoding/decoding failure |

---

## Modified Types

### `AgentLoop` (`crates/sai-core/src/services/agent_loop.rs`)

The `AgentLoop` struct gains a `session_port: &'a dyn SessionPort` field.

`AgentLoop::new()` signature changes from:

```
new(config, llm, tools, ui, permissions)
```

to:

```
new(config, llm, tools, ui, permissions, session)
```

**Session lifecycle within `run_turn`**:
1. On first call: `session_port.create_session(meta)` (no-op if session already exists)
2. After each successful turn: `session_port.save_turn(session.id, turn)`
3. On error: turn is NOT saved (partial turns are not persisted)

---

## On-Disk Layout (`~/.local/share/sai/sessions/` on Linux)

```
~/.local/share/sai/sessions/
├── {uuid-1}/
│   ├── meta.json          ← SessionMeta (atomically rewritten after each turn)
│   └── turns.jsonl        ← One ConversationTurn JSON object per line (append-only)
├── {uuid-2}/
│   ├── meta.json
│   └── turns.jsonl
└── ...
```

**File permissions**: directories `0700`, files `0600`.

**`meta.json` example**:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "refactor-auth",
  "model_name": "claude-sonnet-4",
  "working_dir": "/home/user/project",
  "created_at": "2026-04-08T10:00:00Z",
  "last_active_at": "2026-04-08T10:45:00Z",
  "turn_count": 12
}
```

**`turns.jsonl` example** (one JSON object per line):
```jsonl
{"turn_index":0,"user_message":"What does this file do?","messages":[...],"completed_at":"2026-04-08T10:00:05Z"}
{"turn_index":1,"user_message":"Can you refactor it?","messages":[...],"completed_at":"2026-04-08T10:15:22Z"}
```

---

## New Workspace Crate: `sai-session`

**Location**: `crates/sai-session/`

**Contents**:

| File | Purpose |
|------|---------|
| `src/lib.rs` | Public re-exports: `FilesystemSessionAdapter` |
| `src/adapter.rs` | `FilesystemSessionAdapter` — implements `SessionPort` |
| `src/error.rs` | Internal error-to-`SessionError` conversions |

**Dependencies** (additions to `Cargo.toml`):
- `sai-core` (workspace)
- `tokio` (workspace, features: `fs`, `io-util`)
- `serde` + `serde_json` (workspace)
- `uuid` (workspace)
- `async-trait` (workspace)
- `dirs` v5 (new workspace dep)
- `chrono` v0.4 with `serde` feature (new workspace dep)
- `thiserror` (workspace)
