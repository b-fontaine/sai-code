# CLI Contract: Session Persistence

This document defines the command-line interface changes introduced by the session
persistence feature. It extends the existing CLI contract from step 004.

---

## Updated Entry Point: `sai-code`

### Revised Argument Structure

The existing `sai-code [OPTIONS] [MESSAGE]` invocation is preserved unchanged. New
session management flags are added to the top-level `Cli` struct, and a `sessions`
subcommand is added for management operations.

```
sai-code [OPTIONS] [MESSAGE]
sai-code sessions [SUBCOMMAND]
```

---

## New Flags (top-level, interactive mode)

### `--resume [SESSION_ID]`

```
--resume [SESSION_ID]
```

Resume a previously saved session.

- If `SESSION_ID` is provided: load the session with that exact ID (UUID or name).
- If `SESSION_ID` is omitted: resume the most recently active session in the current
  working directory. If no such session exists, start a new session (no error).
- On resume: print a one-line banner to stderr:
  `Resumed session {id} ({turn_count} turns, {relative_date})`
- If the session ID does not exist: print an error to stderr and exit with code 1.
- If the session file is corrupted: warn and start a fresh session (no exit).

**Examples**:
```
sai-code --resume                                    # resume most recent
sai-code --resume 550e8400-e29b-41d4-a716-446655440000  # resume by UUID
sai-code --resume refactor-auth                      # resume by name
```

---

### `--session-name NAME`

```
--session-name NAME
```

Assign a human-readable name to the current session at launch time.

- Applies to both new sessions and resumed sessions (renames on resume).
- `NAME` must match `[a-zA-Z0-9_-]+` (alphanumeric, hyphens, underscores only).
- If a session with this name already exists: print a warning and append `-2`, `-3`, etc.
  until a unique name is found.
- The name is shown in `sessions list` output.

**Example**:
```
sai-code --session-name refactor-auth
```

---

## `sessions` Subcommand

### `sessions list`

```
sai-code sessions list [--dir PATH] [--limit N]
```

List saved sessions, most-recently-active first.

**Options**:
- `--dir PATH`: filter to sessions started in the given working directory (default: all)
- `--limit N`: show at most N sessions (default: 20)

**Output format** (table to stdout):

```
ID                                    NAME             TURNS  LAST ACTIVE      DIR
550e8400-e29b-41d4-a716-446655440000  refactor-auth    12     2 hours ago      ~/project
7c9e6679-7425-40de-944b-e07fc1f90ae7  —                3      yesterday        ~/other
```

- Columns: ID (truncated to 8 chars if `--short` flag present), NAME (`—` if unnamed),
  TURNS, LAST ACTIVE (human-relative), DIR (home-relative path).
- If no sessions exist: print `No saved sessions.` and exit 0.
- Non-interactive output (piped): emit JSON array of session metadata objects.

---

### `sessions show SESSION_ID`

```
sai-code sessions show SESSION_ID
```

Show full metadata for one session and print a summary of its turns.

**Output**:
```
Session: 550e8400-e29b-41d4-a716-446655440000
Name:    refactor-auth
Model:   claude-sonnet-4
Dir:     ~/project
Created: 2026-04-08 10:00:00 UTC
Active:  2026-04-08 10:45:00 UTC
Turns:   12

Turn 1 (2026-04-08 10:00:05): What does this file do?
Turn 2 (2026-04-08 10:15:22): Can you refactor it?
...
```

- Truncates turn user messages at 80 characters.

---

### `sessions delete SESSION_ID [--all]`

```
sai-code sessions delete SESSION_ID
sai-code sessions delete --all
```

Delete one or all sessions.

- `SESSION_ID`: delete session by UUID or name. If not found, exit 1 with error.
- `--all`: delete all sessions. Requires explicit confirmation:
  ```
  Delete all 42 sessions? This cannot be undone. [y/N]:
  ```
  Default is `N` (no). Non-interactive mode (stdin not a terminal) exits 1 with an
  error rather than silently deleting.
- On success: print `Deleted session {id}.`

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Session not found, or invalid argument, or I/O error |
| `2` | Corrupted session (only when `--resume` with explicit ID) |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `SAI_SESSION_DIR` | Override the default session storage directory |

This allows users to point the agent at a custom or shared directory in special
configurations, without changing the default XDG behavior.

---

## Backward Compatibility

All existing invocation forms continue to work unchanged:

```
sai-code                          # new session, TUI mode
sai-code "initial message"        # new session with first message
sai-code --model gpt-4o           # new session with specified model
```

Sessions are created automatically in the background. Users who do not use `--resume`
or `sessions list` are unaffected.
