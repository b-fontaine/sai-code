# Designing a multi-provider Rust CLI agent with ratatui

**A complete Rust + ratatui CLI coding agent — comparable to Claude Code but with multi-provider LLM support — is entirely feasible today using a mature ecosystem of crates.** The recommended stack centers on `genai` v0.5 for unified LLM abstraction across 14+ providers, `ratatui` v0.30 with the Component trait pattern for the terminal UI, `rmcp` v1.3 for Model Context Protocol integration, and `tree-sitter` v0.26 for code analysis and bash safety validation. This report details every layer of the architecture, from crate selection to configuration patterns, based on what's actively maintained and battle-tested as of early 2026.

---

## The LLM provider layer: genai vs rig-core vs rolling your own

The Rust ecosystem now offers three credible multi-provider LLM abstraction crates, each with different design philosophies. **`genai` v0.5.x** (681 GitHub stars, by Jeremy Chone) provides the broadest native provider coverage — **14+ providers** including OpenAI, Anthropic, Gemini, xAI, Ollama, Groq, DeepSeek, and Cohere — with a clean normalized API. It auto-routes to providers based on model name prefixes (e.g., `"gpt-4o"` → OpenAI, `"claude-sonnet-4"` → Anthropic). It supports streaming via a unified `ChatStreamEvent`, tool/function calling through a `Tool` trait, and runs on tokio + reqwest 0.13.

**`rig-core`** (5,900 stars, by 0xPlaygrounds) is the most popular Rust LLM framework, now at **v0.28–0.33** with rapid monthly releases. Rig takes a higher-level approach: it provides `CompletionModel` and `EmbeddingModel` traits, agent abstractions with `#[derive(Tool)]` macros, RAG support with vector store integrations (MongoDB, Qdrant, LanceDB), and MCP integration. It ships with built-in OpenAI, Anthropic, and Cohere providers, and supports Gemini via `rig-gemini-grpc`. Production users include St. Jude, Dria, and VT Code. The trade-off is a steeper learning curve and pre-1.0 API instability.

A third option, **`llm` v1.3.7** (by graniet, 275 stars), bundles agents, chains, evaluation, and a REST API server into a single crate with 12+ provider support. It's ambitious but newer and less battle-tested.

For a CLI coding agent, the recommended hybrid approach uses **`genai` as the primary provider abstraction** (broadest coverage, simplest API) supplemented by **`async-openai`** for deep OpenAI features and **`ollama-rs` v0.3.4** for local model management. If you need RAG or vector search, swap in `rig-core` instead of `genai`.

| Crate | Version | Providers | Streaming | Tool calling | Stars |
|-------|---------|-----------|-----------|-------------|-------|
| `genai` | 0.5.x | **14+ native** | ✅ Unified SSE | ✅ `Tool` trait | 681 |
| `rig-core` | 0.28–0.33 | 3-5 built-in + adapters | ✅ Multi-turn | ✅ `#[derive(Tool)]` | **5,900** |
| `llm` (graniet) | 1.3.7 | 12+ | ✅ | ✅ Standardized | 275 |
| `async-openai` | Latest | OpenAI-only* | ✅ Full SSE | ✅ Parallel | ~1,200+ |
| `ollama-rs` | **0.3.4** | Ollama-only | ✅ tokio-stream | ✅ | ~1,000 |

---

## Ratatui v0.30 and the Component architecture for complex TUI apps

Ratatui v0.30.0 shipped in late December 2025 as the framework's biggest release ever. The codebase split into a **modular workspace**: `ratatui-core` v0.1.0 (traits/types for library authors), `ratatui-widgets` (built-in widgets), and separate backend crates for crossterm, termion, and termwiz. The main `ratatui` crate re-exports everything. New features include `no_std` support for embedded targets, a simplified `ratatui::run()` entry point, vertical alignment enums, and serde `Serialize`/`Deserialize` derives on layout types. MSRV is **Rust 1.88.0**.

The **Component trait pattern** is the recommended architecture for complex apps. It's not a library-level trait but an application-level pattern documented on ratatui.rs and codified in the official `cargo generate ratatui/templates component` template. Each component implements five methods:

```rust
pub trait Component {
    fn init(&mut self, area: Rect) -> Result<()>;
    fn handle_events(&mut self, event: Option<Event>) -> Result<Option<Action>>;
    fn update(&mut self, action: Action) -> Result<Option<Action>>;
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()>;
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()>;
}
```

The async integration pattern uses **two channel systems**: a `Tui` struct that reads crossterm's `EventStream` (via the `event-stream` feature) inside a `tokio::select!` loop alongside tick and render intervals, emitting `Event`s; and an `mpsc::unbounded_channel<Action>` for dispatching domain actions between components. Background work like LLM streaming spawns via `tokio::spawn` with a cloned `action_tx` sender, pushing results like `Action::StreamChunk(text)` back into the main loop. The render cycle fires only on `Action::Render` at a controlled frame rate (typically 30 FPS), decoupling UI updates from event processing.

Notable large ratatui apps to study include **Yazi** (async file manager contributing to ratatui-core), **gitui** (complex multi-panel Git UI), and **OpenCrabs** (self-hosted AI agent with full async architecture). For LLM-specific TUI apps, **tenere** and **Oatmeal** demonstrate chat interfaces with streaming token display.

---

## MCP support with rmcp v1.3 is production-ready

The **`rmcp` crate v1.3.0** is the official Rust SDK for Model Context Protocol, maintained under the `modelcontextprotocol` GitHub organization with **3,200 stars** and **6.6 million downloads**. It supports all three transport modes: **stdio** (`transport-io` feature), **SSE** (`transport-sse`, legacy), and the new **Streamable HTTP** (`transport-streamable-http-server`, MCP spec 2025-03-26+) for axum-based servers. Both client and server modes are fully implemented.

Tool registration uses an ergonomic macro system via `rmcp-macros`:

```rust
#[tool(tool_box)]
impl Calculator {
    #[tool(description = "Calculate the sum of two numbers")]
    async fn sum(&self, #[tool(aggr)] req: SumRequest) -> String {
        (req.a + req.b).to_string()
    }
}
```

The SDK covers resources, prompts, sampling (server-initiated LLM calls via client), roots, logging, and completions. It supports protocol versions from 2024-11-05 through 2025-11-25. For a CLI agent, the client mode is most relevant — connecting to existing MCP servers (filesystem, database, GitHub, etc.) to expand the agent's tool capabilities. The pattern is straightforward: `().serve(TokioChildProcess::new(command)).await?` creates a client session, and `client.list_all_tools().await?` discovers available tools.

Alternative MCP crates exist — `rust-mcp-sdk` v0.8.0, `mcpr`, and `prism-mcp-rs` — but `rmcp` is the clear standard, used by Goose and Swiftide.

---

## Lessons from Goose, Swiftide, and yoagent on agent architecture

Three Rust agent projects provide the most valuable architectural lessons. **Goose by Block** (35,000+ stars) is the largest Rust coding agent, structured as a Cargo workspace: `goose` (core logic), `goose-cli`, `goose-server` (backend binary `goosed`), and `goose-mcp` (MCP server implementations for shell, file operations). It uses a `Provider` trait for LLM integration and MCP extensions for tool registration.

**Swiftide** (by bosun-ai, `swiftide-agents` v0.32.1) provides the most mature Rust agent framework with proper lifecycle hooks. Its `Agent::builder()` pattern supports `.on_stream()` for real-time token display, `.before_completion()` hooks, `.tools()` registration, and configurable iteration limits. Swiftide integrates directly with `rmcp` for MCP tool discovery and supports OpenAI, Anthropic, Gemini, Ollama, and AWS Bedrock.

**yoagent** offers the cleanest minimal implementation of the core agent loop — the pattern every coding agent shares:

1. Build message history with system prompt + user input
2. Call LLM with tools defined → receive streamed response
3. If response contains tool calls → execute tools (parallel by default), append results to messages, loop back to step 2
4. If response is text-only → return final answer

yoagent exposes this as an event stream (`AgentStart → TurnStart → MessageUpdate → ToolExecution → TurnEnd → AgentEnd`) delivered through a `tokio::sync::mpsc::Receiver<AgentEvent>` — the exact pattern needed for feeding streaming output into a ratatui UI.

For **permission systems**, the universal pattern is a `confirm_fn: Option<Box<dyn Fn(&str) -> bool>>` callback on dangerous tools, combined with deny-pattern lists and path restrictions. Goose adds MCP tool annotations for security metadata, while AutoAgents implements guardrail pipelines (Block, Sanitize, Audit policies).

---

## SSE streaming and the reqwest-eventsource ecosystem

LLM APIs universally use Server-Sent Events for token streaming, and **`reqwest-eventsource` v0.6.0** is the de facto standard Rust crate for consuming them — used by `async-openai`, `genai`, `langchain-rust`, and `aichat`. It wraps `eventsource-stream` (the parser layer) with automatic reconnection on top of reqwest. The critical pattern for LLM APIs is POST-based SSE, since all providers require a POST with JSON body:

```rust
let builder = client.post("https://api.anthropic.com/v1/messages")
    .header("x-api-key", api_key)
    .json(&request_body);
let mut es = EventSource::new(builder)?;
while let Some(event) = es.next().await {
    match event {
        Ok(Event::Message(msg)) => { /* parse provider-specific chunk */ }
        Err(e) => { es.close(); break; }
    }
}
```

For lower-level control, **`eventsource-stream` v0.2.3** provides just the parser as a `Stream` extension trait on any byte stream. **`eventsource-client` v0.17** by LaunchDarkly is the most actively maintained option with production-grade reconnection and exponential backoff, though it's hyper-native rather than reqwest-native. A newer crate, **`sseer`**, offers a `JsonStream` feature that directly deserializes SSE data fields into typed structs — potentially valuable for eliminating boilerplate when parsing `ChatCompletionChunk` objects.

The stream processing pipeline typically chains `futures::StreamExt` combinators (`.map()`, `.filter()`, `.try_next()`) and feeds parsed chunks through a `tokio::mpsc` channel into the ratatui action loop.

---

## Tool calling differs significantly across providers

The four major providers implement tool/function calling with substantive differences in wire format. **OpenAI** wraps tools in `{"type": "function", "function": {..., "parameters": ...}}` and returns arguments as a **JSON string** requiring parsing. **Anthropic** uses `input_schema` instead of `parameters`, returns arguments as a **parsed object**, and embeds tool calls as `content` blocks with `type: "tool_use"` — interleaved with text blocks. Tool results go in `role: "user"` messages (not `role: "tool"`). **Gemini** nests definitions in `functionDeclarations` arrays and returns calls as `functionCall` parts with parsed args. **Ollama** follows OpenAI's format closely but returns parsed objects and uses `tool_name` instead of `tool_call_id` for matching results.

The key abstraction requires a **`ProviderAdapter` trait** that handles four responsibilities: serializing tool definitions into provider-specific JSON, serializing message history (especially tool results which differ dramatically), parsing SSE stream events into normalized `StreamEvent` enums, and detecting tool-call stop conditions (`finish_reason: "tool_calls"` for OpenAI vs `stop_reason: "tool_use"` for Anthropic). A unified `ToolCall` struct should always store arguments as a parsed `serde_json::Value`, normalizing OpenAI's string format on deserialization. For Ollama, which lacks call IDs, generate synthetic UUIDs.

| Aspect | OpenAI | Anthropic | Gemini | Ollama |
|--------|--------|-----------|--------|--------|
| Schema key | `function.parameters` | `input_schema` | `parameters` | `function.parameters` |
| Args format | **JSON string** | Parsed object | Parsed object | Parsed object |
| Result role | `role: "tool"` | `role: "user"` + `tool_result` block | `functionResponse` part | `role: "tool"` |
| ID matching | `tool_call_id` | `tool_use_id` | `id` | `tool_name` (no ID) |
| Stop signal | `finish_reason: "tool_calls"` | `stop_reason: "tool_use"` | `functionCall` present | `tool_calls` present |

---

## Tree-sitter v0.26 for code analysis and command safety

The `tree-sitter` Rust bindings at **v0.26.6** (February 2026) are actively maintained and provide two critical capabilities for a CLI agent. First, **bash command safety validation**: parsing user-requested shell commands with `tree-sitter-bash` v0.25.1 and walking the AST to detect dangerous patterns — `command` nodes containing `rm`, `dd`, `mkfs`; `pipeline` nodes piping to `bash` or `eval`; `redirected_statement` nodes targeting sensitive paths. This structural analysis is far more reliable than regex matching, correctly handling quoted strings, variable expansion, and complex command chains.

Second, **code context extraction**: using query patterns like `(function_item name: (identifier) @name parameters: (parameters) @params)` to extract function signatures, struct definitions, and import statements from source files. This produces condensed context for LLM prompts instead of sending entire files, dramatically improving token efficiency. Language grammars are available for Rust, Python, JavaScript/TypeScript, Go, and dozens more, all following the same `Parser::set_language()` → `parse()` → `Query` → `QueryCursor` pattern.

---

## TOML configuration with layered overrides

**TOML is the unambiguous choice** for Rust application configuration. The `serde_yaml` crate was deprecated by dtolnay in March 2024 with no official successor, while the `toml` crate remains actively maintained. The `config` crate v0.15.18 enables layered configuration merging (defaults → environment-specific → local overrides → environment variables), which is the pattern used by production tools like `aichat` v0.30 (29,000+ stars).

The recommended configuration structure separates providers, models, routing, and security into distinct TOML tables. API keys should **never** appear in config files — reference environment variable names instead (`api_key_env = "ANTHROPIC_API_KEY"`). Model routing maps task types to named model configurations with fallback chains. The `provider:model-name` format pioneered by aichat (e.g., `"anthropic:claude-sonnet-4"`) is an intuitive convention for multi-provider model references.

```toml
[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"
base_url = "https://api.anthropic.com"

[models.claude-sonnet]
provider = "anthropic"
model_id = "claude-sonnet-4-20250514"
temperature = 0.3
max_tokens = 8192

[routing]
code_generation = "claude-sonnet"
fallbacks = { claude-sonnet = ["gpt4o", "local-codellama"] }
```

The corresponding Rust structs use `serde::Deserialize` with `HashMap<String, ProviderConfig>` for dynamic provider registration, making it trivial to add new providers without code changes.

---

## Recommended complete dependency stack

The full `Cargo.toml` for this architecture brings together all the researched crates into a coherent stack:

```toml
[dependencies]
# TUI
ratatui = "0.30"
crossterm = { version = "0.29", features = ["event-stream"] }

# Async runtime
tokio = { version = "1", features = ["full"] }
tokio-util = "0.7"
tokio-stream = "0.1"
futures = "0.3"

# LLM providers
genai = "0.5"                    # Multi-provider abstraction (14+ providers)
async-openai = { version = "*", optional = true }  # Deep OpenAI support
ollama-rs = { version = "0.3", features = ["stream"] }  # Local model management

# MCP
rmcp = { version = "1.3", features = ["client", "transport-io", "transport-sse"] }

# SSE streaming
reqwest-eventsource = "0.6"
eventsource-stream = "0.2"

# Code analysis
tree-sitter = "0.26"
tree-sitter-bash = "0.25"
tree-sitter-rust = "0.23"

# Configuration
toml = "0.8"
config = "0.15"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling & logging
color-eyre = "0.6"
tracing = "0.1"
tracing-subscriber = "0.3"

# CLI
clap = { version = "4", features = ["derive"] }
```

This stack is entirely async/tokio-compatible, with every crate actively maintained as of early 2026. The architecture follows the proven patterns from Goose, Swiftide, and yoagent for the agent loop; the ratatui Component pattern for UI; genai for provider abstraction; rmcp for MCP tool integration; and tree-sitter for code intelligence — yielding a modular, extensible foundation for a production-grade multi-provider CLI coding agent.