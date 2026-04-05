# Port Trait Contracts: Core Agent Loop

**Feature**: 001-core-agent-loop
**Date**: 2026-04-05

These contracts define the port traits that the agent loop depends on.
Adapter crates MUST implement these traits. The agent loop MUST NOT
depend on any other external interface.

## LlmPort

The agent loop's sole interface to language models.

**Contract**:
- `chat_stream(request) -> ChatStream`: Send conversation history to
  the model and receive a streaming response. The stream yields
  `ChatStreamEvent` variants until completion.
- `model_name() -> &str`: Return the identifier of the active model.
- `provider_name() -> &str`: Return the name of the active provider.

**Behavioral expectations**:
- Streaming MUST begin yielding events within a reasonable time after
  the request is sent (provider-dependent).
- The stream MUST eventually terminate with a `StreamEnd` event or
  an error.
- Tool-use blocks MUST be yielded as complete `ToolCallComplete`
  events (not partial chunks) so the agent loop can process them
  without reassembly.
- The `ChatRequest` MUST accept a system prompt, a list of messages,
  and an optional list of tool definitions.

## ToolPort

Interface for a single tool's execution.

**Contract**:
- `name() -> &str`: Unique tool identifier.
- `description() -> &str`: Human-readable description (sent to model).
- `input_schema() -> JsonValue`: JSON Schema for the tool's input.
- `execute(input) -> ToolOutput`: Run the tool with given arguments.
- `is_concurrency_safe() -> bool`: Whether this tool can run in
  parallel with other tools (default: false).
- `is_read_only() -> bool`: Whether this tool only reads state
  (default: false).

**Behavioral expectations**:
- `execute()` MUST NOT panic. All errors MUST be returned as
  `ToolOutput::Error`.
- `input_schema()` MUST return a valid JSON Schema object.

## ToolRegistryPort

Interface for looking up and listing registered tools.

**Contract**:
- `get(name) -> Option<&dyn ToolPort>`: Look up a tool by name.
- `list() -> Vec<&dyn ToolPort>`: Return all registered tools.
- `tool_definitions() -> Vec<JsonValue>`: Return tool definitions
  in the format expected by the LLM (name, description, schema).

**Behavioral expectations**:
- `get()` MUST return `None` for unregistered tools (not panic).
- `list()` MUST return a consistent snapshot (no partial updates
  during iteration).

## UiPort

Interface for delivering events to the user.

**Contract**:
- `emit_event(event: AgentEvent)`: Send an event to the UI layer.

**Behavioral expectations**:
- MUST NOT block the agent loop. If the UI can't keep up, events
  MAY be buffered or dropped (but dropping MUST be logged).
- The UI layer is responsible for rendering; the agent loop only
  emits events.

## PermissionPort

Interface for checking whether a tool may be executed.

**Contract**:
- `check(request: PermissionRequest) -> PermissionDecision`: Evaluate
  whether the given tool call is permitted.
  - `PermissionDecision::Allow`: Proceed with execution.
  - `PermissionDecision::Deny(reason)`: Do not execute; return
    reason as tool error result to the model.
  - `PermissionDecision::Ask`: Prompt the user for approval (routed
    through UiPort).

**Behavioral expectations**:
- MUST be called before every tool execution. The agent loop MUST NOT
  execute a tool without a permission check.
- A `Deny` result MUST NOT cause the agent loop to crash; instead,
  the denial reason is sent to the model as a tool error result.
