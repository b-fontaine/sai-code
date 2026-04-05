# Research: Core Agent Loop

**Feature**: 001-core-agent-loop
**Date**: 2026-04-05
**Status**: Complete

## R1: genai Crate API for Agent Loop Integration

**Decision**: Use `genai` v0.5.x as the LLM abstraction behind the
`LlmPort` trait in `sai-llm`. The agent loop in `sai-core` never
touches genai directly.

**Rationale**: genai provides the broadest native provider coverage
(14+ providers) with a clean normalized API. It auto-routes to
providers based on model name prefixes (`"gpt-4o"` → OpenAI,
`"claude-sonnet-4"` → Anthropic). A single `genai::Client` handles
all providers — switching at runtime means changing the model string.

**Key API surface** (consumed only by `sai-llm` adapter):

- `Client::new()` → single client for all providers
- `client.exec_chat(model_name, chat_req, options)` → `ChatResponse`
- `client.exec_chat_stream(model_name, chat_req, options)` → `ChatStreamResponse`
  with `ChatStreamEvent` variants: `StreamStart`, `StreamChunk(text)`,
  `ToolCallChunk`, `StreamEnd`
- `ChatRequest` built via `ChatRequest::new(messages)` with
  `.with_system(text)`, `.with_tools(vec![tool_def])`, `.append_message(msg)`
- `ChatMessage` has roles: `System`, `User`, `Assistant`, `Tool`
- Tool definitions use the `Tool` trait with `name()`,
  `description()`, `input_schema() -> Value`, `call(args) -> Value`
- Tool results sent back as `ChatMessage::Tool { tool_call_id, content }`

**Alternatives considered**:

- `rig-core` v0.28-0.33: Higher-level agent abstractions, but
  steeper learning curve and pre-1.0 API instability. Better suited
  if RAG/vector search were needed.
- `async-openai`: OpenAI-only — too narrow for multi-provider.
- Custom `reqwest` client: Maximum control but high maintenance
  burden for normalizing 4+ provider wire formats.

## R2: Async Agent Loop Pattern

**Decision**: Use a `while` loop with `tokio::select!` for
concurrent event handling, and `tokio::sync::mpsc` channels for
streaming events from the loop to the UI layer.

**Rationale**: This is the universal pattern used by production Rust
agents (Goose, yoagent, Swiftide). The loop is simple, testable,
and works naturally with Rust's ownership model.

**Core pattern**:

```
loop {
    let response = llm_port.chat_stream(request).await?;
    let (tool_calls, text) = collect_stream(response, event_tx).await?;

    if tool_calls.is_empty() {
        // End of turn — text-only response
        break Ok(text);
    }

    // Execute tools, append results, continue loop
    let results = tool_executor.execute(tool_calls).await?;
    request.append_tool_results(results);
    iteration_count += 1;

    if iteration_count >= max_iterations {
        break Err(AgentError::IterationLimitExceeded);
    }
}
```

**Event delivery**: The agent loop sends `AgentEvent` variants
through an `mpsc::Sender<AgentEvent>` to the UI layer:
- `StreamStart`
- `TextDelta(String)`
- `ToolCallStart { name, call_id }`
- `ToolCallComplete { call_id, result }`
- `TurnComplete`
- `Error(AgentError)`

This pattern matches yoagent's `AgentEvent` stream delivered through
`tokio::sync::mpsc::Receiver<AgentEvent>`.

**Alternatives considered**:

- Async generators (`Stream` trait): More ergonomic for consumers but
  harder to test and doesn't compose well with the mutable loop state.
- `tokio::select!` with multiple branches: Overkill for the agent
  loop itself (which is sequential); better suited for the TUI event
  loop that listens to both user input and agent events.

## R3: Parallel Tool Execution

**Decision**: Use `tokio::task::JoinSet` for parallel tool execution
with concurrency-safety partitioning.

**Rationale**: JoinSet provides structured concurrency — all spawned
tasks are tracked and awaited, with clean cancellation semantics.
This matches the pattern used by Claude Code's
`StreamingToolExecutor` which partitions tool calls into
concurrent-safe (parallel, capped at 10) and non-safe (sequential).

**Partitioning logic**:

```
fn partition(calls: Vec<ToolCall>) -> (Vec<ToolCall>, Vec<ToolCall>) {
    // Check ToolPort::is_concurrency_safe() for each call
    calls.into_iter().partition(|c| registry.is_concurrency_safe(c.name))
}
```

Parallel batch runs first via JoinSet (capped at configurable
concurrency limit, default 10). Sequential batch runs after, one
at a time. All results are collected in original request order.

**Alternatives considered**:

- `FuturesUnordered`: Lower overhead but lacks structured
  cancellation and is harder to limit concurrency.
- Sequential-only: Simpler but unacceptably slow for common
  multi-file operations.
- `tokio::spawn` with manual tracking: JoinSet does this better
  with built-in cancellation on drop.

## R4: Streaming Response Collection

**Decision**: Collect streaming tokens into both the UI event
channel and an accumulator buffer simultaneously. Tool-use blocks
are detected as they complete within the stream.

**Rationale**: Claude Code's `StreamingToolExecutor` demonstrates
that tool execution can start before the full response finishes
streaming. However, for v1, we simplify: collect the full response
stream first, then execute tools. Early execution is a future
optimization.

**Stream collection**:

```
async fn collect_stream(
    stream: ChatStream,
    event_tx: &mpsc::Sender<AgentEvent>,
) -> Result<(Vec<ToolCall>, Option<String>), AgentError> {
    let mut text = String::new();
    let mut tool_calls = Vec::new();

    while let Some(event) = stream.next().await {
        match event? {
            StreamChunk(delta) => {
                text.push_str(&delta);
                event_tx.send(AgentEvent::TextDelta(delta)).await?;
            }
            ToolCallComplete(call) => {
                tool_calls.push(call);
                event_tx.send(AgentEvent::ToolCallStart {
                    name: call.name.clone(),
                    call_id: call.id.clone(),
                }).await?;
            }
            StreamEnd => break,
        }
    }

    Ok((tool_calls, if text.is_empty() { None } else { Some(text) }))
}
```

## R5: Error Handling Strategy

**Decision**: Three error categories with distinct handling:

1. **Tool errors** (unknown tool, execution failure): Converted to
   `ToolResult::Error` and sent back to the model. The loop continues.
2. **Model errors** (connection failure, rate limit, token limit):
   Surfaced to the user via `AgentEvent::Error`. Retryable errors
   (rate limit, transient connection) trigger automatic retry with
   exponential backoff (max 3 retries). Non-retryable errors break
   the loop.
3. **Iteration limit**: Breaks the loop and informs the user.

**Rationale**: Feeding tool errors back to the model is the universal
pattern (used by Claude Code, Goose, yoagent) — it gives the model
a chance to recover. Model errors require user awareness since they
indicate infrastructure issues.

**Error types** (in `sai-core/src/error.rs`):

```
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Tool '{name}' not found in registry")]
    ToolNotFound { name: String },
    #[error("Tool '{name}' execution failed: {reason}")]
    ToolExecutionFailed { name: String, reason: String },
    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),
    #[error("Iteration limit exceeded ({limit} iterations)")]
    IterationLimitExceeded { limit: usize },
    #[error("Event channel closed")]
    ChannelClosed,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("Connection failed: {0}")]
    Connection(String),
    #[error("Rate limited, retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },
    #[error("Token limit exceeded")]
    TokenLimitExceeded,
    #[error("Provider error: {0}")]
    Provider(String),
}
```

## R6: Message History Representation

**Decision**: Use a `Vec<Message>` in the `AgentSession` struct,
with `Message` as a Rust enum with typed variants for each role.

**Rationale**: An enum with distinct variants for User, Assistant,
and ToolResult messages provides compile-time guarantees that
invalid message sequences are caught. This matches Claude Code's
approach where `messages[]` is the central state of the conversation.

**Key design**:

- Messages are append-only within a turn (no mutation of existing
  messages).
- The system prompt is stored separately in `AgentConfig` and
  prepended on each LLM call, not stored in the history.
- Tool calls are embedded within `AssistantMessage` content
  (matching the Anthropic API format where tool_use blocks are
  content blocks alongside text).
- Tool results are separate `Message::ToolResult` entries that
  reference the call ID.
- Token counting is deferred to the `ContextPort` (out of scope
  for this feature) but `Message` carries an optional
  `cached_token_count` field for future use.
