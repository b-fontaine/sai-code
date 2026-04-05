# Feature Specification: Core Agent Loop

**Feature Branch**: `001-core-agent-loop`
**Created**: 2026-04-05
**Status**: Draft
**Input**: User description: "Implement the core agent loop that sends messages to an LLM, receives responses, detects tool-use requests, executes tools, and loops until the LLM produces a final text response"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Single-Turn Text Response (Priority: P1)

A user types a question or instruction into the CLI agent. The agent
sends the message to the configured language model. The model responds
with a plain text answer (no tool calls). The agent displays the
response and returns to the input prompt.

**Why this priority**: This is the simplest end-to-end path through the
agent loop. Without it, nothing else works. It validates that the core
send-receive cycle is functional.

**Independent Test**: Send a message that requires only a text answer
(e.g., "What is 2+2?"). Verify the agent displays the model's response
and returns to the input prompt within a reasonable time.

**Acceptance Scenarios**:

1. **Given** the agent is running and awaiting input, **When** the user
   submits a text message, **Then** the message is sent to the language
   model and a text response is displayed to the user.
2. **Given** the model returns a text-only response, **When** the
   response is received, **Then** the agent does not attempt any tool
   execution and returns to the input prompt.
3. **Given** the model is streaming its response, **When** tokens arrive
   incrementally, **Then** the user sees partial output appearing
   progressively (not waiting for the full response).

---

### User Story 2 - Single Tool Call and Resolution (Priority: P1)

A user asks the agent to perform a task that requires a tool (e.g.,
"Read the file README.md"). The model responds with a tool-use request.
The agent executes the requested tool, sends the result back to the
model, and the model produces a final text response incorporating the
tool output.

**Why this priority**: Tool execution is the core differentiator of a
coding agent. A single tool call is the minimal viable tool-use flow.

**Independent Test**: Ask the agent to read a known file. Verify the
agent executes the file-read tool, sends the result to the model, and
displays a final response that references the file content.

**Acceptance Scenarios**:

1. **Given** the user submits a message, **When** the model responds
   with a tool-use request, **Then** the agent identifies the tool name
   and arguments from the response.
2. **Given** a tool-use request is detected, **When** the tool executes
   successfully, **Then** the tool result is sent back to the model as
   a follow-up message.
3. **Given** the tool result has been sent, **When** the model responds
   with a text-only answer, **Then** the agent displays it and returns
   to the input prompt.
4. **Given** a tool-use request is detected, **When** the tool execution
   fails, **Then** the error is sent to the model as the tool result so
   the model can respond appropriately.

---

### User Story 3 - Multi-Turn Tool Chain (Priority: P2)

A user asks the agent to perform a complex task requiring multiple
sequential tool calls (e.g., "Find all TODO comments in the project
and summarize them"). The model issues a tool call, receives the result,
then issues another tool call, and so on until it has enough information
to produce a final text response.

**Why this priority**: Real-world coding tasks almost always require
multiple tool invocations. This story validates the loop's ability to
iterate beyond a single tool call.

**Independent Test**: Ask the agent a question requiring at least two
different tools (e.g., search for files, then read one). Verify all
tool calls execute in sequence, each result is fed back, and the final
response integrates information from all tools.

**Acceptance Scenarios**:

1. **Given** the model has received a tool result, **When** it responds
   with another tool-use request instead of a text answer, **Then** the
   agent executes the new tool and sends its result back to the model.
2. **Given** a chain of 3 or more tool calls, **When** each tool
   completes, **Then** the agent continues the loop until the model
   produces a final text response.
3. **Given** one tool in a chain fails, **When** the error is sent to
   the model, **Then** the model can decide to try an alternative
   approach or report the issue to the user.

---

### User Story 4 - Parallel Tool Execution (Priority: P2)

The model responds with multiple tool-use requests in a single
response (e.g., "read file A" and "read file B" simultaneously). The
agent detects that the tools are safe to run concurrently, executes
them in parallel, and sends all results back in one follow-up message.

**Why this priority**: Parallel tool execution significantly improves
agent responsiveness for multi-file operations, which are common in
coding tasks.

**Independent Test**: Trigger a scenario where the model requests two
independent file reads. Verify both execute concurrently and results
are returned together.

**Acceptance Scenarios**:

1. **Given** the model returns multiple tool-use requests in one
   response, **When** all requested tools are marked as safe for
   concurrent execution, **Then** the agent executes them in parallel.
2. **Given** parallel tool execution completes, **When** all results
   are collected, **Then** they are sent back to the model in a single
   follow-up message preserving the order of the original requests.
3. **Given** the model returns multiple tool-use requests, **When** any
   tool is not safe for concurrent execution, **Then** those tools
   execute sequentially while concurrency-safe tools run in parallel.

---

### User Story 5 - Conversation History Continuity (Priority: P3)

The user has an ongoing conversation with the agent across multiple
turns. Each new user message is sent to the model along with the full
conversation history (prior messages, tool calls, and tool results),
so the model maintains context across the session.

**Why this priority**: Without history continuity, every user message
is treated as a fresh conversation. This story is lower priority
because the core loop works without it (single-turn is P1), but it is
essential for a usable agent.

**Independent Test**: Send two related messages in sequence (e.g.,
"What files are in src/" followed by "Read the first one"). Verify the
second response correctly references context from the first exchange.

**Acceptance Scenarios**:

1. **Given** the user has completed one or more prior exchanges, **When**
   a new message is submitted, **Then** the full conversation history
   (user messages, model responses, tool calls, tool results) is
   included in the request to the model.
2. **Given** conversation history grows large, **When** the history
   exceeds a configured size limit, **Then** the agent signals that
   compression or truncation is needed (actual compression is out of
   scope for this feature).

---

### Edge Cases

- What happens when the model returns an empty response (no text, no
  tool calls)? The agent MUST treat this as an end-of-turn and return
  to the input prompt without error.
- What happens when the model requests a tool that is not registered?
  The agent MUST return an error result to the model indicating the tool
  is unknown, allowing the model to recover.
- What happens when the model enters an infinite tool-call loop? The
  agent MUST enforce a configurable maximum iteration limit per turn
  (default: 50 iterations). When reached, the agent stops the loop and
  informs the user.
- What happens when the connection to the model is lost mid-response?
  The agent MUST report a clear error to the user and allow them to
  retry the last message.
- What happens when the model response exceeds the maximum token limit?
  The agent MUST handle this gracefully by signaling that the response
  was truncated and re-requesting with an increased output budget if
  possible.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The agent MUST maintain an ordered list of messages
  representing the conversation history (user messages, model responses,
  tool calls, and tool results).
- **FR-002**: The agent MUST send the conversation history to the
  configured language model and receive a response for each turn.
- **FR-003**: The agent MUST support streaming responses, delivering
  partial output to the user as tokens arrive.
- **FR-004**: The agent MUST inspect each model response to determine
  whether it contains tool-use requests, a text-only answer, or both.
- **FR-005**: When a tool-use request is detected, the agent MUST
  look up the requested tool by name in the tool registry.
- **FR-006**: The agent MUST execute the requested tool with the
  provided arguments and capture the result (success or error).
- **FR-007**: After tool execution, the agent MUST append the tool
  result to the conversation history and re-submit to the model.
- **FR-008**: The loop MUST continue (model call -> tool execution ->
  model call) until the model produces a response with no tool-use
  requests (end-of-turn).
- **FR-009**: The agent MUST support multiple tool-use requests in a
  single model response, executing them in parallel when safe to do so.
- **FR-010**: The agent MUST enforce a configurable maximum iteration
  limit per turn to prevent infinite loops (default: 50).
- **FR-011**: When a requested tool is not found in the registry, the
  agent MUST return an error result to the model (not crash).
- **FR-012**: When tool execution fails, the agent MUST send the error
  as the tool result so the model can handle it.
- **FR-013**: The agent MUST handle model-level errors (connection
  failures, rate limits, token limit exceeded) with clear user-facing
  messages and retry capability.
- **FR-014**: The agent MUST support a configurable system prompt that
  is prepended to the conversation history on every model call.

### Key Entities

- **Message**: A single entry in the conversation history. Has a role
  (user, assistant, tool-result), content (text or structured data),
  and optional metadata (timestamp, token count).
- **Tool Call**: A structured request from the model to invoke a tool.
  Contains the tool name, a unique call identifier, and input arguments.
- **Tool Result**: The output of a tool execution. Contains the call
  identifier, success/error status, and result content.
- **Conversation Turn**: One cycle of user-input -> model-response(s)
  -> optional tool execution(s) -> final model response.
- **Agent Session**: The full stateful context of an ongoing
  conversation, including all turns and configuration.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users receive the first visible token of a streaming
  response within 2 seconds of submitting a message (excluding model
  provider latency).
- **SC-002**: The agent correctly resolves 95% of single-tool-call
  tasks (tool is executed, result is returned, model produces a
  coherent final answer).
- **SC-003**: Multi-tool chains of up to 10 sequential tool calls
  complete without the agent losing context or producing errors.
- **SC-004**: Parallel tool execution completes at least 30% faster
  than sequential execution for 2+ independent tool calls.
- **SC-005**: The agent gracefully handles all defined edge cases
  (unknown tool, tool failure, empty response, iteration limit,
  connection loss) without crashing.
- **SC-006**: Conversation history is preserved across at least 20
  consecutive turns within a single session.
- **SC-007**: The agent correctly detects end-of-turn (no more tool
  calls) and returns to the user prompt within 500ms of receiving the
  final model token.

## Assumptions

- The language model provider is already configured and accessible via
  a provider abstraction. This spec does not cover provider selection
  or authentication.
- A tool registry exists and is populated with at least one tool
  before the agent loop starts. Tool implementation is out of scope.
- The user interface (TUI or minimal stdout) exists and can display
  streaming text. UI implementation is out of scope for this feature.
- Permission checks on tool execution are handled by a separate system.
  The agent loop calls into the permission layer but does not implement
  permission logic itself.
- Context window management (compression, truncation) is a separate
  feature. This spec only requires the loop to signal when history
  exceeds a configured limit.
- The system prompt content is provided by configuration; this spec
  does not define what the system prompt contains.
