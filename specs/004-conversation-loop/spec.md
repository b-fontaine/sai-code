# Feature Specification: Interactive Conversation Loop

**Feature Branch**: `004-conversation-loop`  
**Created**: 2026-04-05  
**Status**: Draft  
**Input**: User description: "Implement the interactive conversation loop (REPL) that reads user input, delegates to AgentLoop, streams responses, handles errors and signals, and loops until exit"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Start Agent and Send First Message (Priority: P1)

A user launches the coding agent from the terminal. The agent displays a welcome banner showing the active model and current working directory. The user types a question or instruction and presses Enter. The agent streams the response token-by-token to the terminal, then displays a new input prompt, ready for the next message.

**Why this priority**: This is the minimum viable product — the first end-to-end path from binary launch to a complete conversation turn. Without it, the agent cannot be used at all.

**Independent Test**: Launch the agent binary with valid credentials, type "What is 2+2?", verify a streamed response appears and a new prompt follows.

**Acceptance Scenarios**:

1. **Given** the user runs the agent binary with valid credentials, **When** the process starts, **Then** a welcome banner is displayed showing the model name and working directory, followed by an input prompt.
2. **Given** the input prompt is displayed, **When** the user types a message and presses Enter, **Then** the response streams progressively to the terminal (tokens appear as they arrive, not all at once).
3. **Given** the response finishes streaming, **When** the turn completes, **Then** a new input prompt appears and the user can type another message.

---

### User Story 2 - Multi-Turn Conversation with Context (Priority: P1)

A user sends multiple messages in sequence during a session. Each new message benefits from the full conversation history — the agent remembers prior turns. The user can reference earlier context (e.g., "do the same thing to the other file") and the agent responds correctly.

**Why this priority**: A coding agent without conversation memory is unusable for real tasks. Even basic workflows require multi-turn context (e.g., "list files" then "read the first one").

**Independent Test**: Send "list files in src/", then "read the first one" — verify the second response correctly uses context from the first exchange.

**Acceptance Scenarios**:

1. **Given** the user has completed one or more turns, **When** they send a follow-up message, **Then** the agent's response demonstrates awareness of the prior exchange.
2. **Given** the session has 10+ turns, **When** the user sends another message, **Then** the conversation continues without noticeable degradation.
3. **Given** the conversation history grows very large, **When** a size threshold is reached, **Then** the user sees a notification suggesting they consider starting a new session.

---

### User Story 3 - Graceful Exit (Priority: P1)

The user can exit the agent cleanly through multiple methods: typing `/exit` or `/quit`, pressing Ctrl-D (end-of-input), or pressing Ctrl-C while at the input prompt. The agent cleans up resources, optionally prints a farewell message, and terminates with a success exit code.

**Why this priority**: An application that cannot be exited cleanly is fundamentally broken. This is a basic usability requirement.

**Independent Test**: Start the agent, type `/exit`, verify the process terminates with exit code 0.

**Acceptance Scenarios**:

1. **Given** the agent is waiting for input, **When** the user types `/exit` or `/quit`, **Then** the agent prints a farewell message and exits with code 0.
2. **Given** the agent is waiting for input, **When** the user presses Ctrl-D (end-of-input signal), **Then** the agent exits with code 0.
3. **Given** the agent is waiting for input, **When** the user presses Ctrl-C, **Then** the agent exits cleanly without a panic or error trace.
4. **Given** the agent is mid-response (actively streaming), **When** the user presses Ctrl-C once, **Then** the current turn is cancelled and the user returns to the input prompt.
5. **Given** the agent is mid-response, **When** the user presses Ctrl-C twice in quick succession, **Then** the process exits immediately.

---

### User Story 4 - Activity Feedback During Processing (Priority: P2)

While the agent is processing (waiting for the language model or executing tools), the user sees visual feedback so they know the system has not frozen. This includes an indicator while waiting for the first response token, tool execution notifications showing which tool is running, and clear separation between tool activity and the agent's text response.

**Why this priority**: Without visible feedback, users cannot distinguish "working" from "hung." This is critical for trust and usability but not required for basic functionality.

**Independent Test**: Ask the agent to read a file. Verify that a "thinking" indicator appears before the first token, the tool name is displayed when it runs, and the final text response is visually distinct from tool activity.

**Acceptance Scenarios**:

1. **Given** the user has submitted a message, **When** the language model has not yet returned the first token, **Then** a visual indicator (e.g., "Thinking...") is displayed.
2. **Given** the agent is executing a tool, **When** the tool starts, **Then** the tool name is displayed to the user.
3. **Given** a tool completes, **When** the result is available, **Then** the user sees a brief status indicator (success or failure).
4. **Given** the model responds with text after tool execution, **When** the text streams, **Then** it is visually distinct from the tool activity output.

---

### User Story 5 - Error Recovery in Conversation (Priority: P2)

When a language model call fails (network error, rate limit, provider outage), the agent displays a clear, human-readable error message and allows the user to continue. The conversation history is preserved — the user does not lose their session. They can retry their message or send a different one.

**Why this priority**: Errors are inevitable in real usage. Losing an entire session to a transient network issue makes the tool unreliable.

**Independent Test**: Simulate a network failure during a turn. Verify the error is displayed clearly, then send another message and verify the session continues normally.

**Acceptance Scenarios**:

1. **Given** a language model request fails with a transient error (e.g., network timeout), **When** the error is displayed, **Then** the user can type a new message and the session continues.
2. **Given** a language model request fails with a configuration error (e.g., invalid credentials), **When** the error is displayed, **Then** the message clearly identifies the cause so the user can fix it.
3. **Given** a tool execution fails during a turn, **When** the failure is reported back to the model, **Then** the conversation continues normally with the model explaining the issue.
4. **Given** the user has built up 10+ turns of context, **When** an error occurs on the next turn, **Then** all prior conversation history is preserved and available for subsequent turns.

---

### User Story 6 - Empty and Whitespace-Only Input Handling (Priority: P3)

When the user submits an empty line or whitespace-only input (e.g., accidental Enter press), the agent simply re-displays the input prompt without making any calls to the language model. This prevents wasted API calls and confusing responses.

**Why this priority**: Quality-of-life improvement. Accidental Enter presses should be harmless, not trigger API calls.

**Independent Test**: Press Enter with no text typed. Verify the prompt reappears immediately with no delay or API activity.

**Acceptance Scenarios**:

1. **Given** the agent is at the input prompt, **When** the user presses Enter with no text, **Then** the prompt re-appears immediately without any language model call.
2. **Given** the agent is at the input prompt, **When** the user enters only spaces or tabs, **Then** the prompt re-appears immediately without any language model call.

---

### User Story 7 - Initial Message via Command-Line Argument (Priority: P3)

A user can pass an initial prompt as a command-line argument (e.g., `sai-code "fix the failing test"`). The agent starts, immediately processes that message as the first turn without waiting for interactive input, then enters the interactive loop for follow-up conversation.

**Why this priority**: Power-user convenience that enables scripting and faster workflows. Interactive mode alone is sufficient for v1, making this a nice-to-have.

**Independent Test**: Run the agent with an inline message argument. Verify the first turn executes immediately, then verify the agent enters interactive mode for follow-up.

**Acceptance Scenarios**:

1. **Given** the user runs the agent with an inline message argument, **When** the process starts, **Then** the first turn executes immediately without waiting for interactive input.
2. **Given** the initial message has been processed, **When** the response is complete, **Then** the agent enters interactive mode with a prompt for follow-up messages.
3. **Given** the user runs the agent with no arguments, **When** the process starts, **Then** the agent enters interactive mode directly with a prompt.

---

### Edge Cases

- What happens when the user presses Ctrl-C while a tool (especially a shell command) is actively running? The running operation must be terminated cleanly and the user must return to the input prompt without the agent crashing.
- What happens when standard input is not an interactive terminal (e.g., piped input like `echo "hello" | sai-code`)? The agent should process all input, then exit when end-of-input is reached, without displaying interactive prompts.
- What happens when the user submits an extremely long message (>100KB)? The input should be accepted without truncation — the language model's context window is the natural limit.
- What happens when the model's response is truncated due to output length limits? The agent should inform the user that the response was cut short.
- What happens when the conversation history size warning fires? The agent should display a notice suggesting the user consider starting a new session.
- What happens when no credentials are configured for the language model provider? The agent should fail at startup with a clear, actionable error message — not silently fail on the first turn.
- What happens when an invalid model name is specified? The agent should report a clear error at startup or on first use.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide a binary entry point that can be invoked from the terminal as a single command.
- **FR-002**: The system MUST read user input from standard input, one message per submission.
- **FR-003**: The system MUST pass each user message to the agent loop and display the resulting response.
- **FR-004**: The system MUST display responses progressively as tokens arrive (streaming output), not wait for the full response before displaying.
- **FR-005**: The system MUST loop after each completed turn, presenting a new input prompt for the next message.
- **FR-006**: The system MUST support graceful exit via Ctrl-C at the prompt, Ctrl-D (end-of-input), `/exit`, and `/quit`.
- **FR-007**: The system MUST handle Ctrl-C during an active turn by cancelling the current operation and returning to the input prompt (first signal) or exiting the process (second signal in quick succession).
- **FR-008**: The system MUST discard empty or whitespace-only input without sending it to the language model.
- **FR-009**: The system MUST display errors from language model calls and tool executions as clear, human-readable messages — never raw stack traces or panic output.
- **FR-010**: The system MUST preserve conversation history across all turns within a single session (process invocation).
- **FR-011**: The system MUST wire all required subsystems (language model, tool registry, user interface, permissions) with concrete adapters at startup.
- **FR-012**: The system MUST display tool execution activity to the user, showing the tool name when execution starts and a status indicator when it completes.
- **FR-013**: The system MUST accept an optional initial message as a command-line argument, processing it as the first turn before entering interactive mode.
- **FR-014**: The system MUST exit with code 0 on normal termination and a non-zero code on unrecoverable errors.
- **FR-015**: The system MUST display a startup banner showing the active model name and current working directory.

### Key Entities

- **Conversation Loop**: The outer read-eval-print loop that reads user input, delegates to the agent loop for processing, displays results, and repeats. Distinct from the agent loop which handles a single turn's model-tool cycle.
- **Terminal UI Adapter**: A concrete adapter that translates agent events (text deltas, tool activity, errors) into terminal output. The bridge between the domain event system and the user's screen.
- **Terminal Permission Adapter**: A concrete adapter that prompts the user interactively for approval when a tool requires explicit permission (e.g., writing files, running shell commands).
- **Input Reader**: The component responsible for reading user input from standard input, detecting end-of-input and interrupt signals, and handling non-interactive (piped) input modes.
- **Startup Configuration**: The resolution of agent settings (model name, credentials, system prompt, working directory) from environment variables and command-line arguments.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can install and run the agent binary and complete a working interactive conversation within 60 seconds of setup (assuming valid credentials).
- **SC-002**: The first response token appears on screen within 500 milliseconds of the language model beginning its response (measuring only the agent's overhead, not provider latency).
- **SC-003**: The agent handles 50 or more consecutive turns in a single session without crashing, leaking resources, or losing conversation context.
- **SC-004**: All defined exit methods (Ctrl-C at prompt, Ctrl-D, `/exit`, `/quit`, double Ctrl-C during a turn) terminate the process cleanly with exit code 0.
- **SC-005**: When a language model call fails with a transient error, the user can send another message and the session continues — 100% of the time.
- **SC-006**: Empty or whitespace-only input never results in a language model call.
- **SC-007**: Every tool execution is visible to the user — each tool start event produces a visible line of output.

## Assumptions

- Steps 1-3 (agent loop, language model adapter, tools) are fully implemented and their interfaces are stable. This feature depends on those contracts.
- The terminal output will use simple standard output for v1. A richer terminal UI (color formatting, panels, etc.) is a separate future feature.
- Configuration (credentials, model name) will be resolved from environment variables for v1. A dedicated configuration file system is out of scope.
- Basic line-by-line input reading is acceptable for v1. Enhanced line editing (arrow keys, input history recall) can be added as a follow-up improvement.
- The permission system for tool execution will use a simple interactive prompt. Sophisticated permission policies (auto-approve rules, deny lists) are out of scope.
- There is no session persistence (save and resume). Each process invocation starts a fresh session. Session persistence is a separate future feature.
