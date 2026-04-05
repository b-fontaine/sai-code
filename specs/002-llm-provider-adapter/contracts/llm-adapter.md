# Contract: LlmPort Implementation

**Feature**: 002-llm-provider-adapter
**Date**: 2026-04-05

## Interface

`GenaiLlmAdapter` implements `sai_core::ports::llm::LlmPort`.

### `chat_stream(request: ChatRequest) -> Result<ChatStream, LlmError>`

**Pre-conditions**:
- `request.messages` MUST contain at least one `Message::User`.
- API key environment variable for the active provider MUST be set
  (except for local providers like Ollama).

**Post-conditions**:
- Returns a `ChatStream` that yields `ChatStreamEvent` items.
- The stream starts with `StreamStart`.
- Text content arrives as one or more `TextDelta(String)` events.
- Tool calls arrive as `ToolCallComplete(ToolCall)` events with:
  - A non-empty `id` (synthetic if provider doesn't supply one).
  - A non-empty `name`.
  - `input` as a parsed `serde_json::Value` (never a raw JSON string).
- The stream ends with `StreamEnd { stop_reason }`.
- If the provider errors, the stream yields an `Err(LlmError)`.

**Error conditions**:
- Missing API key → `LlmError::Connection("PROVIDER_API_KEY not set")`
- Network failure → `LlmError::Connection(details)`
- Rate limited → `LlmError::RateLimited { retry_after_secs }`
- Token limit → `LlmError::TokenLimitExceeded`
- Other → `LlmError::Provider(details)`

### `model_name() -> &str`

Returns the current model identifier string (e.g., `"claude-sonnet-4"`).

### `provider_name() -> &str`

Returns the derived provider name (e.g., `"anthropic"`, `"openai"`,
`"gemini"`, `"ollama"`).

## Normalization Guarantees

These invariants MUST hold for all supported providers:

1. **Tool call IDs are always present**: If the provider doesn't
   supply call IDs (e.g., Ollama), the adapter generates UUID-based
   synthetic IDs.
2. **Tool call arguments are always parsed**: If the provider returns
   arguments as a JSON string (e.g., OpenAI), the adapter parses them
   into a `serde_json::Value` object.
3. **Stop reasons are always normalized**: Provider-specific stop
   signals are mapped to `StopReason::EndTurn`, `StopReason::ToolUse`,
   or `StopReason::MaxTokens`.
4. **Messages are provider-agnostic on input**: The adapter accepts
   `sai_core::Message` variants and handles all provider-specific
   formatting internally.
5. **Errors are always classified**: No raw HTTP errors or
   provider-specific error types reach the agent loop.
