# Quickstart: Multi-Provider LLM Adapter

**Feature**: 002-llm-provider-adapter
**Date**: 2026-04-05

## Prerequisites

- Rust toolchain (1.80.0+)
- cargo-nextest installed
- At least one LLM provider API key:
  - `ANTHROPIC_API_KEY` for Claude models
  - `OPENAI_API_KEY` for GPT models
  - `GEMINI_API_KEY` for Gemini models
  - Ollama running locally (no key needed)

## Build

```bash
# Build the adapter crate
cargo build -p sai-llm

# Build with all workspace crates
cargo build
```

## Run Tests

```bash
# Unit tests (no API key needed — uses mock responses)
cargo nextest run -p sai-llm

# Integration tests with Anthropic (requires API key)
ANTHROPIC_API_KEY=sk-ant-... cargo nextest run -p sai-llm -- integration

# Integration tests with OpenAI
OPENAI_API_KEY=sk-... cargo nextest run -p sai-llm -- integration

# Integration tests with Ollama (must be running locally)
cargo nextest run -p sai-llm -- integration_ollama
```

## Verify Provider Support

### 1. Anthropic (Claude)

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
# The adapter routes "claude-*" models to Anthropic
```

### 2. OpenAI (GPT)

```bash
export OPENAI_API_KEY="sk-..."
# The adapter routes "gpt-*" and "o1-*" models to OpenAI
```

### 3. Google Gemini

```bash
export GEMINI_API_KEY="..."
# The adapter routes "gemini-*" models to Gemini
```

### 4. Ollama (Local)

```bash
# Start Ollama with a model
ollama pull llama3.2
ollama serve
# The adapter routes "ollama::*" models to localhost:11434
```

## Verify Streaming

The adapter MUST stream tokens incrementally. A correct implementation
shows text appearing progressively, not all at once after a delay.

## Verify Tool Calling

Send a request with tool definitions. The adapter MUST:
1. Include tool definitions in the provider's expected format
2. Return tool calls with normalized fields (id, name, input)
3. Accept tool results and format them per provider expectations

## Configuration

| Setting | Source | Default |
|---------|--------|---------|
| Model name | `AgentConfig.model_name` | `"claude-sonnet-4"` |
| API keys | Environment variables | (none) |
| Ollama endpoint | `OLLAMA_HOST` env var | `http://localhost:11434` |

## Architecture

```
AgentLoop (sai-core)
    │
    │  calls LlmPort::chat_stream()
    │
    ▼
GenaiLlmAdapter (sai-llm)
    │
    │  converts to genai types
    │  calls genai::Client
    │
    ▼
genai Client (internal)
    │
    │  auto-routes by model prefix
    │
    ├──► Anthropic API
    ├──► OpenAI API
    ├──► Gemini API
    └──► Ollama (local)
```
