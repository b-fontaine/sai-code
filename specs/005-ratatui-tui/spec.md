# Feature Specification: Rich Terminal User Interface

**Feature Branch**: `005-ratatui-tui`
**Created**: 2026-04-07
**Status**: Draft
**Input**: User description: "Create a full TUI with Ratatui. The TUI provides an interactive terminal experience using ratatui and crossterm. It renders streaming LLM responses, tool execution status, and permission prompts in a rich terminal layout."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Structured Terminal Layout on Launch (Priority: P1)

When the user starts the coding agent, instead of seeing plain scrolling text, they are presented with a structured, visually distinct terminal interface. The screen is divided into clearly labeled regions: a scrollable conversation area, a tool activity area, and a fixed input area at the bottom. The user immediately understands where to type and where responses will appear.

**Why this priority**: This is the foundational visual change. Every other user story depends on a structured layout being in place. Without it, the richer display features have no surface to render into.

**Independent Test**: Launch the agent binary. Verify the terminal is cleared and a structured layout appears with visible panel separations, a conversation area, a tool activity area, and a visible input prompt — all without typing any message.

**Acceptance Scenarios**:

1. **Given** the user launches the agent, **When** the process starts, **Then** the terminal is replaced by a structured layout with a conversation area, an activity area, and an input area — each visually separated.
2. **Given** the TUI is active, **When** the terminal window is resized, **Then** all panels resize responsively and no content is cut off or corrupted.
3. **Given** the TUI is active, **When** no action has been taken, **Then** a status line shows the active model name and current working directory.

---

### User Story 2 - Streaming AI Responses in the Conversation Area (Priority: P1)

As the AI generates a response, the user sees tokens appearing progressively in the conversation area — words flowing in as they arrive rather than appearing all at once at the end. Prior exchanges scroll upward to make room for new content.

**Why this priority**: Streaming feedback is the primary interaction quality improvement over plain text output. Without visible streaming, the TUI provides no advantage over the existing interface.

**Independent Test**: Type a question and press Enter. Verify that AI response tokens appear one-by-one in the conversation area, not all at once, and that the user message and AI response are visually distinct from each other.

**Acceptance Scenarios**:

1. **Given** the user submits a message, **When** the AI begins responding, **Then** a visual indicator (e.g., "Thinking…") appears in the conversation area before the first token arrives.
2. **Given** the AI is generating a response, **When** tokens arrive, **Then** each token is appended to the conversation area immediately — the user sees progress without waiting.
3. **Given** the response is complete, **When** the turn ends, **Then** the full response is visible in the conversation area and the input area is ready for the next message.
4. **Given** multiple turns have occurred, **When** the conversation area is full, **Then** older messages scroll up and the user can scroll back to view them.

---

### User Story 3 - Tool Execution Visible in a Dedicated Activity Area (Priority: P1)

When the AI invokes a tool (such as reading a file or running a search), the user sees real-time status in a dedicated section of the screen. The tool name appears when the call starts, and a success or failure indicator appears when it finishes. This area is separate from the AI's text response so the user can distinguish tool activity from conversational output.

**Why this priority**: Tool visibility is critical for user trust. Without a clear indication of what the agent is doing, users cannot tell whether it is working, stuck, or doing something unintended.

**Independent Test**: Ask the agent to read a file. Verify that a dedicated area shows the tool name when execution begins and a completion status when it ends — separate from the AI's text response.

**Acceptance Scenarios**:

1. **Given** the AI invokes a tool, **When** execution begins, **Then** the tool name and a running indicator appear in the activity area.
2. **Given** a tool completes successfully, **When** the result is returned, **Then** the activity area shows a success indicator next to the tool name.
3. **Given** a tool fails, **When** the error is returned, **Then** the activity area shows a failure indicator with a brief description.
4. **Given** multiple tools run in a single turn, **When** each tool starts and ends, **Then** each appears as a separate entry in the activity area with its own status.

---

### User Story 4 - Inline Permission Prompts Within the TUI (Priority: P1)

When the AI requests permission to run a potentially impactful action (such as writing to a file or executing a shell command), the user sees a prompt directly within the TUI. The prompt clearly identifies which tool is requesting permission. The user confirms or denies with a single keypress. The rest of the interface remains visible while the prompt is displayed.

**Why this priority**: Permission prompts are a core security feature. In the TUI they must integrate into the structured layout so they are not missed and do not break the visual hierarchy.

**Independent Test**: Trigger an action that requires permission (e.g., write to a file). Verify a clearly distinguished permission prompt appears in the TUI and that pressing "y" or "n" resolves it and returns to normal interaction.

**Acceptance Scenarios**:

1. **Given** a tool requires user approval, **When** the agent requests permission, **Then** a permission prompt is displayed within the TUI showing the tool name and requested action.
2. **Given** the permission prompt is active, **When** the user presses "y" or Enter, **Then** the tool is allowed and the TUI returns to its normal layout.
3. **Given** the permission prompt is active, **When** the user presses "n" or Escape, **Then** the tool is denied and the TUI returns to its normal layout.
4. **Given** the permission prompt is active, **When** no action is taken, **Then** the prompt remains visible until explicitly resolved — it does not time out automatically.

---

### User Story 5 - Scrollable Conversation History (Priority: P2)

The conversation area holds more messages than fit on screen at once. The user can scroll upward through the history to review earlier exchanges without losing access to the current context. Scrolling does not interrupt an ongoing AI response.

**Why this priority**: Users frequently need to refer back to prior messages. This is a quality-of-life feature that requires the core layout (P1) to be in place first.

**Independent Test**: Have 20 or more turns of conversation. Verify that pressing a scroll-up key shows older messages, that the input area remains functional, and that a new message can be submitted while scrolled up.

**Acceptance Scenarios**:

1. **Given** the conversation history exceeds the visible area, **When** the user scrolls up, **Then** older messages become visible.
2. **Given** the user has scrolled up, **When** a new AI response streams in, **Then** the scroll position stays where the user left it.
3. **Given** the user is viewing old history, **When** they press a key to jump to the bottom, **Then** the view returns to the latest message.

---

### User Story 6 - Keyboard Shortcuts for Common Actions (Priority: P3)

The user can perform common actions using keyboard shortcuts without interrupting their workflow. This includes exiting the agent and clearing the conversation view. A help overlay shows available shortcuts on demand.

**Why this priority**: Nice-to-have efficiency improvement for power users. The agent is fully usable without shortcuts, but they add polish for frequent users.

**Independent Test**: Press the help shortcut key. Verify a list of available keyboard shortcuts is displayed. Press each shortcut and verify it performs its documented action.

**Acceptance Scenarios**:

1. **Given** the TUI is active, **When** the user presses the help key (e.g., "?"), **Then** a key bindings reference is shown.
2. **Given** the TUI is active, **When** the user presses the exit shortcut (Ctrl-C or Ctrl-Q), **Then** the agent exits cleanly with a farewell message and exit code 0.
3. **Given** the conversation area has content, **When** the user presses the clear shortcut, **Then** the conversation area is cleared visually without ending the session.

---

### Edge Cases

- What happens when the terminal is too small to render the minimum layout? The agent should display a message asking the user to resize, rather than rendering a broken layout.
- What happens when a streaming response contains very long lines with no whitespace? Lines should wrap at the panel boundary without corrupting the layout.
- What happens when the user resizes the terminal while an AI response is streaming? The layout should adjust responsively without losing any response tokens.
- What happens when an AI response contains special characters that could corrupt terminal rendering? They should be displayed as literal text.
- What happens when the user scrolls during an ongoing AI response? The response continues rendering; the user can choose to jump back to follow it live.
- What happens when a permission prompt appears while the user is actively typing? The typed text is preserved and restored after the permission prompt is resolved.
- What happens when the agent runs in a non-interactive terminal (stdin is piped)? The TUI is not activated; the plain-text streaming interface is used instead.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST display a structured multi-panel terminal layout upon launch, replacing the existing plain-text scrolling output for interactive sessions.
- **FR-002**: The system MUST render AI response tokens progressively in the conversation panel as they arrive.
- **FR-003**: The system MUST display a "thinking" indicator in the conversation panel between user message submission and the arrival of the first response token.
- **FR-004**: The system MUST maintain a scrollable conversation history showing all exchanges in the current session.
- **FR-005**: The system MUST display tool execution events (start, success, failure) in a dedicated activity panel, visually separate from the conversation area.
- **FR-006**: The system MUST present permission prompts as a focused prompt within the TUI — not as inline scrolling text.
- **FR-007**: The system MUST support keyboard-driven approval and denial of permission prompts with a single keypress each.
- **FR-008**: The system MUST provide a fixed input area at the bottom of the layout where the user types and submits messages.
- **FR-009**: The system MUST display the active model name and current working directory in a persistent status area.
- **FR-010**: The system MUST handle terminal resize events gracefully, reflowing all panels to fit the new dimensions without data loss.
- **FR-011**: The system MUST support exiting via standard keyboard shortcuts (Ctrl-C, Ctrl-Q) with a clean farewell and exit code 0.
- **FR-012**: The system MUST enforce a minimum terminal size and display a visible warning if the window is too small to render the layout.
- **FR-013**: The system MUST allow the user to scroll the conversation history without interrupting an in-progress AI response.
- **FR-014**: The system MUST visually distinguish user messages from AI responses in the conversation panel (e.g., different labels or alignment).
- **FR-015**: The system MUST display errors (connection failures, rate limits, tool failures) in the conversation panel or a visible error area without exiting the TUI.
- **FR-016**: The system MUST fall back to plain-text streaming when the terminal is not interactive (stdin is a pipe or redirect).

### Key Entities

- **Conversation Panel**: The primary area showing the exchange history — user messages and AI responses in chronological order, visually distinguished by author label.
- **Activity Panel**: A dedicated section showing tool execution events (tool name, status: running / success / failure) updated in real time during a turn.
- **Input Area**: A persistent, focused text entry field at the bottom of the layout where the user composes and submits messages.
- **Status Bar**: A persistent line showing session-level metadata: active model name, working directory, and current agent state (idle, thinking, streaming).
- **Permission Prompt**: A focused prompt that appears when a tool requires explicit user approval, showing the tool name, action description, and available keybindings.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A first-time user can identify the input area, the response area, and the tool activity area within 5 seconds of the TUI appearing — without reading any documentation.
- **SC-002**: Response tokens begin appearing in the conversation panel within 500 milliseconds of the model starting its output (measuring only the TUI rendering overhead, not model latency).
- **SC-003**: The terminal layout remains correctly rendered after at least 50 consecutive turns in a single session — no visual corruption, no missing panels.
- **SC-004**: Terminal resize events are handled within 100 milliseconds, with no visible rendering artifacts after the resize completes.
- **SC-005**: Permission prompts can be resolved (approved or denied) with a single keypress — no multi-step confirmation required.
- **SC-006**: At least 20 tool invocations per session can be displayed in the activity panel without the panel overflowing or corrupting surrounding areas.
- **SC-007**: All exit methods (Ctrl-C, Ctrl-Q, /exit) terminate the TUI cleanly with exit code 0 and full terminal restoration — the terminal returns to its normal scrolling mode with no artifacts.

## Assumptions

- The TUI replaces the existing plain-text streaming interface for interactive sessions only. When stdin is not a terminal (piped input), the plain-text interface continues to be used.
- The target environment is a POSIX terminal on macOS or Linux. Windows support is a future enhancement and not in scope for this feature.
- The layout uses three distinct visual regions: conversation history, tool activity, and user input. Exact visual styling (colors, borders, icons) is left to the implementation phase.
- No mouse support is required. All interactions are keyboard-only.
- Session history is not persisted to disk. Each process invocation starts a fresh session — session persistence is a separate future feature.
- The TUI supports one active conversation per window. Split-view or multi-agent layouts are out of scope.
- Input history (arrow-key recall of previous typed messages) is out of scope for this feature.
- The existing `/exit` and `/quit` text commands continue to work within the TUI input area.
