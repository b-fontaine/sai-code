# Data Model: Multi-Provider LLM Adapter

**Feature**: 002-llm-provider-adapter
**Date**: 2026-04-05

## Entities

This feature introduces no new domain entities. It implements
the existing port trait (`LlmPort`) and converts between sai-core
domain types and genai internal types.

### Conversion Types (internal to sai-llm)

These types exist only inside the adapter crate to facilitate
conversion. They are NOT part of the public API.

#### RequestConverter

Converts sai-core types to genai request types.

```
RequestConverter
├── convert_messages(Vec<Message>) → Vec<genai::ChatMessage>
├── convert_system_prompt(Option<String>) → genai system param
├── convert_tools(Vec<JsonValue>) → Vec<genai::Tool>
└── convert_tool_result(ToolResult) → genai::ToolCallResponse
```

#### StreamConverter

Converts genai stream events to sai-core stream events.

```
StreamConverter
├── convert_event(genai::ChatStreamEvent) → ChatStreamEvent
├── extract_tool_calls(genai::End) → Vec<ToolCall>
├── map_stop_reason(genai stop) → StopReason
└── generate_synthetic_id() → String  (for Ollama)
```

#### ErrorConverter

Maps genai errors to sai-core LlmError variants.

```
ErrorConverter
├── from_genai_error(genai::Error) → LlmError
├── classify_http_status(u16) → LlmError
└── extract_retry_after(headers) → Option<u64>
```

### GenaiLlmAdapter (public)

The concrete implementation of `LlmPort`.

```
GenaiLlmAdapter
├── client: genai::Client          # Single client for all providers
├── model_name: String             # Current model identifier
└── provider_name: String          # Derived from model name
```

**Methods**:
- `new(model_name) → Self`: Create adapter, validate model prefix
- `chat_stream(request) → ChatStream`: Convert, call genai, stream back
- `model_name() → &str`: Return current model
- `provider_name() → &str`: Return derived provider
- `set_model(name)`: Runtime provider switching

## Conversion Flow

```
Agent Loop                    sai-llm Adapter                    genai / Provider
    │                              │                                  │
    │  ChatRequest                 │                                  │
    │  (sai-core types)            │                                  │
    ├─────────────────────────────►│                                  │
    │                              │  RequestConverter                │
    │                              │  ├ messages → ChatMessage[]      │
    │                              │  ├ tools → Tool[]                │
    │                              │  └ system → system param         │
    │                              │                                  │
    │                              │  genai::ChatRequest              │
    │                              ├─────────────────────────────────►│
    │                              │                                  │
    │                              │  genai::ChatStreamEvent          │
    │                              │◄─────────────────────────────────┤
    │                              │                                  │
    │                              │  StreamConverter                 │
    │                              │  ├ Chunk → TextDelta             │
    │                              │  ├ End → ToolCallComplete[]      │
    │                              │  └ End → StreamEnd               │
    │                              │                                  │
    │  ChatStreamEvent             │                                  │
    │  (sai-core types)            │                                  │
    │◄─────────────────────────────┤                                  │
```
