# Feature Specification: Multi-Provider LLM Adapter

**Feature Branch**: `002-llm-provider-adapter`
**Created**: 2026-04-05
**Status**: Draft
**Input**: User description: "Multi-provider LLM adapter — genai-based LlmPort implementation with streaming, tool calling normalization"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Text Conversation with Any Provider (Priority: P1)

A user configures a model identifier (e.g., "claude-sonnet-4" or
"gpt-4o") and sends a message. The adapter routes the request to
the correct provider, streams the response token by token, and
delivers it to the agent loop as a normalized stream of events.
The user sees the response appear progressively regardless of which
provider is active.

**Why this priority**: Without basic provider connectivity and
streaming, no other agent functionality works. This is the
foundational path.

**Independent Test**: Configure a model identifier, send a simple
text prompt, verify that a streaming response is received and
contains meaningful text content.

**Acceptance Scenarios**:

1. **Given** a valid model identifier and API key, **When** the
   agent sends a text prompt, **Then** the adapter streams a
   response with text delta events followed by a stream-end event.
2. **Given** a model identifier for a cloud provider (e.g.,
   Anthropic, OpenAI), **When** the response is complete, **Then**
   the adapter reports the stop reason (end-of-turn, max tokens).
3. **Given** a model identifier for a local provider (e.g.,
   Ollama), **When** the agent sends the same prompt, **Then** the
   response format is identical to cloud providers from the agent
   loop's perspective.

---

### User Story 2 - Tool Calling Across Providers (Priority: P1)

The agent loop sends a request that includes tool definitions. The
adapter translates these definitions into the provider's expected
format, sends the request, and normalizes the provider's tool-call
response back into the agent loop's standard format. Differences
in how providers represent tool calls (argument format, call IDs,
result roles) are invisible to the agent loop.

**Why this priority**: Tool calling is the core differentiator of
a coding agent. If tool definitions and results can't be normalized,
the agent loop cannot function with multiple providers.

**Independent Test**: Send a request with tool definitions to two
different providers. Verify both return tool calls in identical
normalized format (same field names, same types).

**Acceptance Scenarios**:

1. **Given** a request with tool definitions, **When** the provider
   responds with a tool-use request, **Then** the adapter returns
   a normalized tool call with a unique call ID, tool name, and
   parsed input arguments.
2. **Given** a tool result to send back, **When** the adapter
   constructs the follow-up request, **Then** the tool result is
   formatted according to the active provider's expected format.
3. **Given** the provider returns tool call arguments as a JSON
   string (e.g., OpenAI), **When** the adapter normalizes it,
   **Then** the arguments are delivered as a parsed object.
4. **Given** a provider that does not use call IDs for tool results
   (e.g., Ollama), **When** the adapter normalizes the response,
   **Then** a synthetic call ID is generated and matched correctly.

---

### User Story 3 - Runtime Provider Switching (Priority: P2)

The user changes the model identifier during a session (e.g., from
"claude-sonnet-4" to "gpt-4o"). The adapter detects the new provider
from the model string and routes subsequent requests accordingly.
No restart or reconfiguration is needed — the conversation continues
seamlessly.

**Why this priority**: Users need flexibility to switch between
providers (e.g., cost optimization, capability matching) without
interrupting their workflow.

**Independent Test**: Start a session with one model, switch to
another model mid-session, send a new prompt, and verify the
response comes from the new provider.

**Acceptance Scenarios**:

1. **Given** an active session with provider A, **When** the user
   changes the model identifier to a provider B model, **Then** the
   next request is routed to provider B.
2. **Given** a model switch, **When** the conversation history is
   sent to the new provider, **Then** the message format is adapted
   to the new provider's expectations.

---

### User Story 4 - API Key Resolution (Priority: P2)

The adapter resolves API credentials from environment variables
based on the active provider. Each provider has a standard
environment variable name (e.g., `ANTHROPIC_API_KEY`,
`OPENAI_API_KEY`). If the key is missing, the user receives a
clear error message identifying which variable to set.

**Why this priority**: Without credential resolution, no provider
can be contacted. This is essential but lower priority than the
core streaming/tool-calling normalization since it's a simpler
integration point.

**Independent Test**: Unset an API key, attempt to send a request,
verify the error message names the exact missing environment
variable.

**Acceptance Scenarios**:

1. **Given** the environment variable for the active provider is
   set, **When** a request is made, **Then** the adapter
   authenticates successfully.
2. **Given** the environment variable is missing, **When** a
   request is attempted, **Then** the adapter returns a clear
   error naming the required variable.
3. **Given** an Ollama (local) provider, **When** a request is
   made, **Then** no API key is required.

---

### User Story 5 - Error Normalization (Priority: P3)

Provider-specific errors (rate limits, connection timeouts, invalid
requests, quota exceeded) are normalized into a standard set of
error categories. The agent loop receives the same error types
regardless of which provider produced them, enabling consistent
retry and error-handling logic.

**Why this priority**: The agent loop's retry logic depends on
classifying errors (retryable vs non-retryable). Without
normalization, every provider would need custom error handling.

**Independent Test**: Simulate a rate-limit error from two
different providers. Verify both produce the same error type with
a retry-after duration.

**Acceptance Scenarios**:

1. **Given** a provider returns a rate-limit response, **When** the
   adapter normalizes it, **Then** the error includes the
   recommended wait duration.
2. **Given** a provider returns a connection timeout, **When** the
   adapter normalizes it, **Then** the error is classified as a
   retryable connection error.
3. **Given** a provider returns an invalid-request error, **When**
   the adapter normalizes it, **Then** the error is classified as
   non-retryable with a descriptive message.

---

### Edge Cases

- What happens when the model identifier string doesn't match any
  known provider? The adapter MUST return a clear error identifying
  the unrecognized model and listing supported provider prefixes.
- What happens when the provider's streaming connection drops
  mid-response? The adapter MUST surface a connection error to the
  agent loop so it can retry.
- What happens when the provider sends an unexpected response
  format? The adapter MUST return a provider error with the raw
  response for debugging, rather than panicking.
- What happens when tool call arguments contain invalid JSON? The
  adapter MUST deliver the raw string as the argument value and let
  the tool execution layer handle validation.
- What happens when the provider's streaming format changes (API
  version update)? The adapter MUST have provider-specific parsing
  isolated so changes affect only one provider's code.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The adapter MUST implement the agent loop's provider
  interface, delivering streaming responses as a normalized event
  stream.
- **FR-002**: The adapter MUST support at least 4 provider families:
  Anthropic, OpenAI, Google Gemini, and Ollama (local).
- **FR-003**: The adapter MUST auto-detect the provider from the
  model identifier string (e.g., "claude-*" routes to Anthropic,
  "gpt-*" routes to OpenAI).
- **FR-004**: The adapter MUST normalize tool definitions into each
  provider's expected format when sending requests.
- **FR-005**: The adapter MUST normalize tool-call responses from
  each provider into a standard format with: unique call ID, tool
  name, and parsed input arguments as a structured object.
- **FR-006**: The adapter MUST normalize tool results into each
  provider's expected format when sending follow-up requests (e.g.,
  `role: "tool"` for OpenAI, content block for Anthropic).
- **FR-007**: The adapter MUST stream text responses token by token,
  not buffer the entire response before delivery.
- **FR-008**: The adapter MUST deliver fully assembled tool-call
  objects (not partial chunks) to the agent loop.
- **FR-009**: The adapter MUST resolve API credentials from
  provider-specific environment variables.
- **FR-010**: The adapter MUST return a clear, actionable error when
  the required API key environment variable is not set.
- **FR-011**: The adapter MUST normalize provider-specific errors
  into standard error categories: connection failure, rate limit
  (with retry duration), token limit exceeded, and general provider
  error.
- **FR-012**: The adapter MUST support changing the model identifier
  at runtime without restart.
- **FR-013**: The adapter MUST include the system prompt and tool
  definitions in every request, formatted per provider expectations.

### Key Entities

- **Provider Adapter**: A provider-specific component that translates
  between the normalized interface and a specific LLM provider's API
  format. One adapter per provider family.
- **Model Route**: The mapping from a model identifier string to a
  specific provider adapter. Determined by prefix matching on the
  model name.
- **Normalized Stream Event**: A standard event type (stream start,
  text delta, tool call complete, stream end) that all provider
  adapters produce regardless of the underlying provider's format.
- **Tool Definition**: A description of a tool (name, description,
  input schema) that must be translated into each provider's specific
  format when included in a request.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: The agent can hold a complete text conversation using
  at least 4 different providers without any code changes between
  provider switches.
- **SC-002**: Tool calling works identically across all supported
  providers — a tool-use request from any provider produces the same
  normalized output structure.
- **SC-003**: Switching providers during a session takes effect on
  the very next request (zero-delay provider switching).
- **SC-004**: Missing API key errors are reported within 1 second of
  the request attempt, before any network call is made.
- **SC-005**: Provider-specific errors (rate limit, connection, token
  limit) are correctly classified 100% of the time for all supported
  providers.
- **SC-006**: Streaming text arrives at the agent loop within 500ms
  of the first token leaving the provider's API (adapter overhead
  under 500ms).
- **SC-007**: Adding a new provider requires changes only to the
  adapter layer — no changes to the agent loop or any other component.

## Assumptions

- The agent loop's provider interface (port trait) is already defined
  and stable from feature 001-core-agent-loop. This adapter
  implements that interface.
- API keys are stored in environment variables, not in configuration
  files. The adapter reads them at request time, not at startup.
- The adapter does not manage conversation history — it receives a
  complete message list on each call and returns a stream. State
  management is the agent loop's responsibility.
- Local providers (Ollama) are assumed to be running on the default
  local endpoint. Custom endpoint configuration is supported but
  optional.
- Token counting is out of scope for this feature. The adapter
  reports token usage if the provider includes it in the response
  but does not perform independent counting.
- Extended thinking / reasoning tokens (e.g., DeepSeek, Anthropic)
  are out of scope for v1. The adapter MAY pass them through as text
  deltas but is not required to handle them specially.
