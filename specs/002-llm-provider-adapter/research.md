# Research: Multi-Provider LLM Adapter

**Feature**: 002-llm-provider-adapter
**Date**: 2026-04-05
**Status**: Complete

## R1: genai Crate as Provider Abstraction

**Decision**: Use `genai` v0.5.x as the sole provider abstraction
inside `sai-llm`. The crate is a private dependency — no genai types
appear in the public API.

**Rationale**: genai covers 14+ providers natively with a single
`Client`. It auto-routes by model name prefix, handles SSE streaming,
and normalizes tool calling. Writing custom HTTP clients per provider
would cost months; genai delivers this out of the box.

**Key API mapping** (genai → sai-core):

| genai type | sai-core type | Notes |
|---|---|---|
| `genai::Client` | (internal) | Single client for all providers |
| `genai::chat::ChatRequest` | `sai_core::ports::llm::ChatRequest` | Convert messages + tools + system prompt |
| `genai::chat::ChatMessage` | `sai_core::domain::message::Message` | Role mapping: User/Assistant/Tool |
| `genai::chat::ChatStreamEvent::Chunk` | `ChatStreamEvent::TextDelta` | Text delta forwarding |
| `genai::chat::ChatStreamEvent::End` | `ChatStreamEvent::StreamEnd` | Includes assembled tool calls |
| `genai::chat::ToolCall` | `sai_core::domain::tool_call::ToolCall` | ID + name + args |
| `genai::chat::Tool` | JSON schema from `ToolPort` | genai Tool is data, not a trait |

**Alternatives considered**:

- `rig-core`: Higher-level agent abstractions we don't need; would
  pull in vector search and RAG dependencies.
- Custom `reqwest` clients: Maximum control but 4x maintenance burden
  for SSE parsing, tool format normalization per provider.
- `async-openai` + `anthropic-sdk`: Two crates instead of one,
  missing Gemini/Ollama.

## R2: Message Format Conversion

**Decision**: Convert `sai_core::Message` variants to
`genai::ChatMessage` before each request, and convert genai stream
events back to `sai_core::ChatStreamEvent` during streaming.

**Key conversion details**:

- `Message::User { content }` → `ChatMessage::user(content)`
- `Message::Assistant { content, .. }` → `ChatMessage::assistant(text)`
  (tool-use blocks become part of the assistant message content)
- `Message::ToolResult { call_id, content, .. }` →
  `ChatMessage::tool(vec![ToolCallResponse { call_id, content }])`
- System prompt: passed via `ChatRequest::with_system(prompt)`, not
  as a message in the history

**Provider-specific normalization** (handled by genai internally):

| Aspect | OpenAI | Anthropic | Gemini | Ollama |
|---|---|---|---|---|
| Tool schema key | `function.parameters` | `input_schema` | `parameters` | `function.parameters` |
| Args format | JSON string → parse | Parsed object | Parsed object | Parsed object |
| Result role | `role: "tool"` | `role: "user"` + block | `functionResponse` | `role: "tool"` |
| ID matching | `tool_call_id` | `tool_use_id` | `id` | `tool_name` (no ID) |

genai handles most of this internally. The `sai-llm` adapter only
needs to normalize the final output and handle edge cases genai
doesn't cover (e.g., synthetic IDs for Ollama).

## R3: Streaming Pipeline

**Decision**: Wrap genai's `ChatStream` in a tokio `mpsc` channel
that emits `sai_core::ChatStreamEvent` items. The conversion happens
inline as each genai event arrives.

**Pipeline**:

```
genai stream → convert each event → yield sai ChatStreamEvent
```

- `genai::StreamStart` → `ChatStreamEvent::StreamStart`
- `genai::Chunk(text)` → `ChatStreamEvent::TextDelta(text)`
- `genai::End { tool_calls, .. }` → emit one
  `ChatStreamEvent::ToolCallComplete` per tool call, then
  `ChatStreamEvent::StreamEnd { stop_reason }`

**Alternatives considered**:

- Direct `Stream` wrapping with `async_stream!` macro: Simpler but
  harder to test (no intermediate channel to inspect).
- Buffering entire response: Violates FR-007 (must stream).

## R4: Model Routing and API Key Resolution

**Decision**: Delegate model routing entirely to genai's built-in
prefix matching. API key resolution uses `std::env::var()` with
provider-specific variable names.

**Model prefix → Provider** (genai built-in):

| Prefix | Provider | Env var |
|---|---|---|
| `claude-*` | Anthropic | `ANTHROPIC_API_KEY` |
| `gpt-*`, `o1-*`, `o3-*` | OpenAI | `OPENAI_API_KEY` |
| `gemini-*` | Google Gemini | `GEMINI_API_KEY` |
| `ollama::*`, `llama*` | Ollama (local) | (none required) |
| `groq::*` | Groq | `GROQ_API_KEY` |
| `deepseek-*` | DeepSeek | `DEEPSEEK_API_KEY` |

**Pre-flight key check**: Before calling genai, the adapter checks
whether the required env var is set and returns `LlmError::Connection`
with a message like "ANTHROPIC_API_KEY not set" if missing. This
avoids waiting for a network timeout.

## R5: Error Mapping

**Decision**: Map genai error types to `sai_core::error::LlmError`
variants.

| genai error | sai LlmError | Retryable? |
|---|---|---|
| Network/connection failure | `LlmError::Connection(msg)` | Yes |
| HTTP 429 (rate limit) | `LlmError::RateLimited { retry_after_secs }` | Yes |
| HTTP 400 (bad request) | `LlmError::Provider(msg)` | No |
| HTTP 401/403 (auth) | `LlmError::Connection("auth failed")` | No |
| Context length exceeded | `LlmError::TokenLimitExceeded` | No |
| Unknown/other | `LlmError::Provider(msg)` | No |

The `retry_after_secs` is extracted from the `Retry-After` header
if present, defaulting to 5 seconds otherwise.

## R6: Runtime Provider Switching

**Decision**: The adapter holds the model name as mutable state.
`chat_stream()` reads the current model name on each call and passes
it to `genai::Client`. Since genai resolves the provider from the
model string, switching is automatic.

No special lifecycle management is needed — genai's `Client` is
stateless with respect to providers. A single `Client` instance
handles all providers.
