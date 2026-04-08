# Feature Specification: Session Persistence

**Feature Branch**: `006-session-persistence`
**Created**: 2026-04-08
**Status**: Draft
**Input**: User description: "Allow users to save conversations to disk and resume them in a future session"

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Resume a Previous Conversation (Priority: P1)

A user was debugging a complex problem with the agent. They close the terminal. The next
day, they reopen their terminal in the same working directory and continue the
conversation from where they left off, with the AI having full context of the prior
exchange.

**Why this priority**: This is the core value proposition. Every other story is secondary
to the ability to resume. Without it, the feature has no point.

**Independent Test**: Start a session, exchange at least three messages, exit the agent.
Re-launch with a resume flag. Verify the AI acknowledges the prior context and can
continue the conversation coherently.

**Acceptance Scenarios**:

1. **Given** the user has had a prior session, **When** they launch the agent with a
   resume option, **Then** the previous conversation history is visible and the AI
   responds with awareness of the prior context.
2. **Given** a session was not ended cleanly (process crash), **When** the user resumes,
   **Then** all messages up to the last successful save are available.
3. **Given** the user resumes a session from a different working directory, **When** the
   agent starts, **Then** it notifies the user that the original session was started in a
   different directory.

---

### User Story 2 — List Available Sessions (Priority: P2)

A user wants to find and continue work on a conversation from two days ago. They run the
agent with a list option and sees a table of past sessions with timestamps, working
directories, and message counts. They pick the one they want and resume it.

**Why this priority**: Once sessions are persisted, users need a way to discover and
navigate them. Without listing, resuming old sessions requires knowing the session ID
by memory.

**Independent Test**: Create at least three sessions (start agent, chat, exit, repeat).
Run the list command. Verify all three sessions appear with date, working directory, and
message count.

**Acceptance Scenarios**:

1. **Given** the user has multiple saved sessions, **When** they run the list command,
   **Then** a list of sessions is shown including the date started, working directory, and
   number of conversation turns.
2. **Given** the user has no saved sessions, **When** they run the list command, **Then**
   a clear message indicates no sessions exist.
3. **Given** a large number of sessions exist, **When** listing, **Then** the most recent
   sessions are shown first.

---

### User Story 3 — Auto-Save During a Session (Priority: P1)

A user does not have to explicitly save their conversation. The agent writes conversation
turns to disk automatically as they happen. If the process crashes or is killed, no more
than the last few seconds of conversation is lost.

**Why this priority**: Explicit saving would require user discipline and a new command.
Automatic saving removes this burden entirely and makes data loss from crashes
negligible.

**Independent Test**: Start a session, send several messages, kill the process with
SIGKILL (simulating a crash). Re-launch and resume. Verify all completed turns are
present.

**Acceptance Scenarios**:

1. **Given** the user submits a message and receives a response, **When** the turn
   completes, **Then** the exchange is durably written to disk without any user action.
2. **Given** the agent is forcibly killed mid-response, **When** the user resumes,
   **Then** all completed turns before the kill are present; the incomplete response is
   not included.
3. **Given** the agent starts a fresh session, **When** no prior session exists for this
   directory, **Then** a new session file is created automatically.

---

### User Story 4 — Name a Session (Priority: P3)

A user wants to start a new session focused on a specific task and give it a memorable
name (e.g., "refactor-auth"). Later they can resume it by name rather than navigating a
list of dated entries.

**Why this priority**: Useful for power users managing multiple parallel work streams,
but not required for the core save/resume flow.

**Independent Test**: Launch the agent with a name flag. Verify the session is listed by
that name. Resume it by name.

**Acceptance Scenarios**:

1. **Given** the user launches the agent with a session name, **When** they later list
   sessions, **Then** the named session appears with the provided name.
2. **Given** a named session exists, **When** the user resumes by name, **Then** the
   correct session is loaded.
3. **Given** two sessions have the same name, **When** the user tries to create a
   duplicate name, **Then** the agent warns and asks for confirmation or a different name.

---

### User Story 5 — Delete Sessions (Priority: P3)

A user wants to clean up old sessions that are no longer relevant. They can delete
individual sessions or clear all sessions at once.

**Why this priority**: Disk hygiene and privacy control. Not blocking for core
save/resume but expected by users over time.

**Independent Test**: Delete a specific session. Verify it no longer appears in the list.
Verify its data is removed from disk.

**Acceptance Scenarios**:

1. **Given** a session exists, **When** the user deletes it by ID or name, **Then** it is
   removed from the list and its data is deleted.
2. **Given** the user requests deletion of all sessions, **When** they confirm the
   action, **Then** all session data is removed.
3. **Given** the user attempts to resume a deleted session, **When** no matching session
   exists, **Then** a clear error message is shown.

---

### Edge Cases

- What happens when the session storage location is out of disk space? The agent should
  warn the user and continue in-memory without data loss for the current turn.
- What happens when the session file is corrupted or unreadable? The agent should report
  the issue and start a fresh session rather than crashing.
- What happens when two agent processes try to write to the same session file
  simultaneously? The last writer wins, or writes are serialized — data must not be
  silently lost or corrupted.
- What happens when the user moves the session storage directory? Sessions become
  unavailable but no crash occurs; the agent starts fresh and informs the user.
- What happens when the model used in a prior session is no longer available? The session
  is resumable; the user is informed the model has changed and asked to confirm.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST automatically save each completed conversation turn
  (user message + AI response + tool activity) to durable storage without user action.
- **FR-002**: The system MUST allow users to resume any saved session by providing a
  session identifier (ID or name) at launch time.
- **FR-003**: The system MUST provide a command to list all saved sessions with metadata:
  date started, working directory, session name or ID, and turn count.
- **FR-004**: The system MUST allow users to assign a human-readable name to a session
  at launch time.
- **FR-005**: The system MUST allow users to delete individual sessions or all sessions.
- **FR-006**: The system MUST detect and report corrupted or unreadable session files
  without crashing; it MUST fall back to starting a fresh session.
- **FR-007**: The system MUST store session data in the user's home directory under a
  standard application data path (e.g., `~/.local/share/sai/sessions/`).
- **FR-008**: When resuming a session, the system MUST reconstruct the full conversation
  history and pass it to the AI as prior context.
- **FR-009**: An incomplete AI response (from a crash mid-stream) MUST NOT be included
  when the session is resumed.
- **FR-010**: The system MUST support at least 500 saved sessions without degraded
  listing or resume performance.

### Key Entities

- **Session**: A named, timestamped conversation with a unique identifier, a working
  directory, and a sequence of conversation turns.
- **Conversation Turn**: A single exchange consisting of a user message, the AI response
  text, and any tool calls and results that occurred during that response.
- **Session Metadata**: Lightweight record of a session (ID, name, start time, last
  activity time, working directory, turn count) — sufficient for the list view without
  loading full turn data.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can resume any of their last 100 sessions in under 2 seconds from
  command invocation to first interactive prompt.
- **SC-002**: No more than one completed turn is lost when the agent process is killed
  without warning (SIGKILL).
- **SC-003**: The session list renders all sessions (up to 500) in under 1 second.
- **SC-004**: A session with 200 turns resumes and presents the full history to the AI
  without the user noticing any additional latency beyond a normal fresh start.
- **SC-005**: 100% of completed turns are recoverable after a clean shutdown.

## Assumptions

- Sessions are stored on the local filesystem. Remote or cloud-based session storage is
  out of scope.
- Each user has one session store per machine. Multi-user or shared session stores are
  out of scope.
- The TUI and plain-text CLI modes both support session persistence identically. The
  underlying session port is mode-agnostic.
- Input history (arrow-key recall of typed messages within the terminal) is a separate
  feature and remains out of scope.
- Sessions do not expire automatically. Retention is entirely user-managed via the delete
  command.
- The session storage format may be changed in future steps; no compatibility guarantee
  is made across versions at this stage.
- The working directory associated with a session is the directory from which the agent
  was launched, not any directory the agent navigated to during the session.
