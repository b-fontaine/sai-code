# Quickstart: Session Persistence

This guide shows the end-to-end usage of session persistence once the feature is
implemented.

---

## Basic Usage

### Start a new session (automatic, no flags needed)

```
sai-code
```

Every session is automatically saved to `~/.local/share/sai/sessions/` (Linux) or
`~/Library/Application Support/sai/sessions/` (macOS). Nothing special to configure.

---

### Resume the most recent session

```
sai-code --resume
```

Picks up the most recently active session in the current working directory.

---

### Resume a specific session by name

```
sai-code --resume refactor-auth
```

---

### Start a named session

```
sai-code --session-name refactor-auth
```

---

### Resume the most recent session and rename it

```
sai-code --resume --session-name my-task
```

---

## Listing Sessions

```
sai-code sessions list
```

Output:
```
ID                                    NAME             TURNS  LAST ACTIVE      DIR
550e8400-e29b-41d4-a716-446655440000  refactor-auth    12     2 hours ago      ~/project
7c9e6679-7425-40de-944b-e07fc1f90ae7  —                3      yesterday        ~/other
```

### Filter by directory

```
sai-code sessions list --dir ~/project
```

### Show full details of one session

```
sai-code sessions show 550e8400
```

---

## Deleting Sessions

### Delete a specific session

```
sai-code sessions delete refactor-auth
```

or

```
sai-code sessions delete 550e8400-e29b-41d4-a716-446655440000
```

### Delete all sessions

```
sai-code sessions delete --all
```

Prompts for confirmation before deleting.

---

## Environment Variable Override

Set `SAI_SESSION_DIR` to use a custom storage location:

```
export SAI_SESSION_DIR=/tmp/sai-test-sessions
sai-code
```

---

## What Gets Saved

Each completed conversation turn is automatically persisted:
- The user's message
- The AI's full response (including all text and tool calls)
- Tool execution results

Only **completed** turns are saved. If the process is killed mid-response, only turns
that finished before the kill are available on resume.

---

## Verifying Your Data

Session files are stored as human-readable JSON:

```
ls ~/.local/share/sai/sessions/
cat ~/.local/share/sai/sessions/{uuid}/meta.json
cat ~/.local/share/sai/sessions/{uuid}/turns.jsonl
```

Each line of `turns.jsonl` is one JSON object representing a single conversation turn.
