# Research: Session Persistence

## Storage Format for Turn Data

**Decision**: Append-only JSONL file per session for conversation turns.

**Rationale**: Each completed turn is appended as a single JSON line. Appending to a
file is atomic enough on POSIX systems that a crash between turns leaves the file in a
consistent state — the last partial line (if any) is simply ignored on resume. No
re-writing of historical data is ever needed.

**Alternatives considered**:
- Single JSON file: entire file must be rewritten on each turn → not crash-safe
- SQLite: robust but adds a large dependency with no benefit at this scale
- MessagePack/CBOR: binary, harder to debug, no meaningful size benefit at this scale

---

## Storage Format for Session Metadata

**Decision**: A separate `meta.json` file per session, written atomically (write to
`.meta.json.tmp`, then `rename`).

**Rationale**: POSIX `rename(2)` is atomic on the same filesystem. Readers always see
either the old or the new version, never a partial write. Metadata (turn count, last
active time) is updated after each turn; atomic rename prevents corruption.

**Alternatives considered**:
- Inline in turns.jsonl header line: special-casing the first line adds fragility
- Separate index file: two-level indirection with no benefit for ≤500 sessions

---

## Session Store Location

**Decision**: `{data_dir}/sai/sessions/` where `data_dir` is resolved via the `dirs`
crate (`dirs::data_dir()`).

**Rationale**: Follows platform conventions:
- macOS: `~/Library/Application Support/sai/sessions/`
- Linux (XDG): `~/.local/share/sai/sessions/`

Using the `dirs` crate (lightweight, no transitive deps beyond `libc`) avoids
hardcoding platform-specific paths.

**Alternatives considered**:
- `~/.sai/sessions/`: simple but not XDG-compliant; clutters home directory
- Configurable path: out of scope for this feature; config system is a separate step

---

## File Permissions

**Decision**: Session directories created with mode `0700`, session files with mode
`0600` (owner-only read/write).

**Rationale**: Conversations contain potentially sensitive information (code, file
contents from tool calls, business logic). Restricting access to the owner prevents
other local users from reading session history.

**Implementation note**: Set via `std::os::unix::fs::PermissionsExt` after creating
each file/directory.

---

## Concurrent Write Safety

**Decision**: Single-writer model — no file locking required for the common case. The
JSONL append + atomic metadata rename is safe for one writer process.

**Rationale**: The agent is a single-process application. Multiple simultaneous agent
instances writing to the same session would require a unique session ID per invocation
(UUID v4 guarantees this). There is no shared-write scenario.

**Edge case**: If two processes happen to resume the same named session and both write,
the last writer's metadata wins but turns from both are interleaved in the JSONL. This
is an accepted limitation; the session ID is what uniquely identifies a session, not the
name.

---

## New Workspace Crate: `sai-session`

**Decision**: Add a new `sai-session` adapter crate.

**Rationale**: The filesystem session adapter is an infrastructure concern (I/O,
platform paths, file formats). Per the constitution's Hexagonal Architecture principle,
infrastructure adapters live in their own crate and implement port traits defined in
`sai-core`. Session persistence has a clear domain boundary.

**Alternatives considered**:
- Implement in `sai-cli` directly: violates hexagonal architecture, makes the adapter
  untestable in isolation
- Implement in `sai-core`: violates the zero-infrastructure-dependency rule

---

## Timestamp Representation

**Decision**: Use `chrono::DateTime<chrono::Utc>` serialized as RFC 3339 strings.

**Rationale**: Human-readable in raw JSON files, unambiguous timezone (UTC), and
`chrono` is already a transitive dependency via `genai`. Adding it explicitly to
workspace deps is a minor addition.

**Alternatives considered**:
- `std::time::SystemTime` as Unix timestamp (u64): not human-readable in raw files
- `time` crate: less ecosystem adoption in this codebase

---

## New Dependencies Required

| Crate | Version | Where | Reason |
|-------|---------|-------|--------|
| `dirs` | `~5` | workspace | XDG/platform data directory resolution |
| `chrono` | `~0.4` with serde feature | workspace | Human-readable UTC timestamps |

Both are lightweight with minimal transitive dependencies.

---

## Integration with AgentLoop

**Decision**: Add `session: &'a dyn SessionPort` as a required parameter to
`AgentLoop::new()`. Provide a `NoOpSessionPort` implementation in `sai-core` for
callers that do not want persistence (tests, non-interactive mode).

**Rationale**: Session persistence is a cross-cutting concern for every turn. Making it
optional via an `Option<&dyn SessionPort>` would add null-checks throughout the hot
path. A `NoOpSessionPort` that does nothing is cleaner and avoids changing the trait.

**Alternatives considered**:
- Post-turn callback passed to `run_turn`: more ergonomic but harder to test holistically
- Event-driven (emit a `TurnComplete` event carrying messages): would require `UiPort`
  to handle persistence, mixing concerns

---

## Session Resume Flow

**Decision**: On resume, load the full `Vec<Message>` from the persisted turns and
inject it into `AgentSession::messages` before the first `run_turn` call.

**Rationale**: The LLM receives the full message history on each turn anyway (via
`ChatRequest::messages`). Injecting history at session start is the simplest approach
and requires no changes to the `AgentLoop` or `LlmPort` contracts.

**Alternatives considered**:
- Summarize history before injecting: reduces token usage but changes AI context;
  context compression is a separate future feature
- Store a summary alongside full history: premature; no context limit issues at this
  scale for typical sessions (≤200 turns)
