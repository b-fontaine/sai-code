# Session Port Contract

This document defines the behavioral contract for `SessionPort` implementations. Any
adapter — filesystem, in-memory, future cloud-backed — MUST satisfy these invariants.

---

## Port Trait

```
trait SessionPort: Send + Sync {
    async fn create_session(meta: SessionMeta) → Result<(), SessionError>
    async fn save_turn(session_id: Uuid, turn: ConversationTurn) → Result<(), SessionError>
    async fn load_session(session_id: Uuid) → Result<Option<PersistedSession>, SessionError>
    async fn list_sessions() → Result<Vec<SessionMeta>, SessionError>
    async fn delete_session(session_id: Uuid) → Result<bool, SessionError>
    async fn find_by_name(name: &str) → Result<Option<SessionMeta>, SessionError>
}
```

---

## Behavioral Contracts

### `create_session(meta)`

- **Idempotent**: calling with the same `meta.id` a second time MUST succeed and be a no-op.
- **Uniqueness**: if `meta.name` is non-None and a different session already has that
  name, MUST return `Err(SessionError::NameConflict { name })`.
- **Atomicity**: either the session is created fully or not at all. A partial write
  followed by a crash MUST NOT result in a session appearing in `list_sessions()`.

### `save_turn(session_id, turn)`

- **Precondition**: `create_session` MUST have been called for `session_id` first;
  otherwise return `Err(SessionError::NotFound { id: session_id })`.
- **Ordering**: turns MUST be retrievable in the order they were saved.
  `turn.turn_index` MUST equal the number of previously saved turns.
- **Durability**: after `save_turn` returns `Ok`, the turn MUST survive a process crash.
- **No partial turns**: if a turn write fails mid-way, the turn MUST NOT appear in
  subsequent `load_session` calls.
- **Metadata update**: after each `save_turn`, `meta.turn_count` and
  `meta.last_active_at` MUST be updated to reflect the new state.

### `load_session(session_id)`

- Returns `Ok(None)` if the session does not exist (not an error).
- Returns `Ok(Some(persisted))` where `persisted.turns.len() == persisted.meta.turn_count`.
- If on-disk data is corrupted: return `Err(SessionError::Corrupted { id, reason })`.
  MUST NOT panic.
- The returned `PersistedSession` MUST contain all turns in `turn_index` order.

### `list_sessions()`

- Returns all sessions, ordered by `last_active_at` descending (most recent first).
- Returns `Ok(vec![])` if no sessions exist (not an error).
- MUST NOT return sessions that are in a partially-created state (see `create_session`
  atomicity guarantee).
- Corrupted sessions: MUST be omitted from the list with a warning logged via `tracing`,
  rather than causing the entire list call to fail.

### `delete_session(session_id)`

- Returns `Ok(true)` if the session was found and deleted.
- Returns `Ok(false)` if the session did not exist.
- After return `Ok(true)`: the session MUST NOT appear in subsequent `list_sessions()`
  or `load_session()` calls.
- MUST delete all stored data for the session (turns + metadata).

### `find_by_name(name)`

- Case-sensitive match against `meta.name`.
- Returns `Ok(None)` if no session has this name.
- If multiple sessions have the same name (implementation defect), returns the most
  recently active one.

---

## Contract Tests

Every `SessionPort` implementation MUST pass the contract test suite in
`crates/sai-session/tests/contract_tests.rs` (generated as part of Phase 2 tasks).

The contract test suite covers:
1. Create → save → load round-trip with all field types
2. Idempotent `create_session`
3. Ordering of turns after multiple `save_turn` calls
4. `load_session` returns `None` for unknown ID
5. `delete_session` returns `false` for unknown ID
6. `list_sessions` order (most-recent-first)
7. `find_by_name` with named and unnamed sessions
8. Durability: save → reload from cold (adapter re-constructed)
9. Corrupted data: missing turns file → `Corrupted` error from `load_session`
10. Corrupted data: invalid JSON in turns file → `Corrupted` error, not panic
