# Complete Repository Bootstrap for sai-code

**sai-code is ready to go from zero to full architecture on day one.** This report provides every file, configuration, and specification needed to bootstrap a Rust CLI coding agent with hexagonal architecture, multi-provider LLM support, and spec-driven development. The design draws on Claude Code's proven architecture patterns, the Rust crate ecosystem's best libraries, and GitHub SpecKit's structured specification workflow. Below is the complete blueprint — workspace layout, CI/CD pipelines, configuration files, CLAUDE.md, and eight SpecKit specification documents.

---

## Part 1: GitHub SpecKit — what it is and how to use it

GitHub SpecKit is an **open-source toolkit for Spec-Driven Development (SDD)** created by GitHub. Instead of prompting an AI agent and hoping for the best, you first create formal specification documents (constitution, spec, plan, tasks) then let the AI implement code that conforms to them. All specs are **plain Markdown** — no YAML, no custom format. SpecKit has **81.4K+ stars** and supports 11+ AI coding agents including Claude Code.

### Installation and initialization

SpecKit is a **Python package** installed via `uv` (Astral's package manager), not npm:

```bash
# Install persistently
uv tool install specify-cli --from git+https://github.com/github/spec-kit.git@v0.5.0

# Initialize in the sai-code repo with Claude Code support
cd sai-code
specify init --here --ai claude

# Verify tools are available
specify check
```

### Directory structure created

After `specify init --here --ai claude`, SpecKit generates:

```
sai-code/
├── .claude/
│   └── commands/
│       ├── plan.md
│       ├── specify.md
│       └── tasks.md
├── .specify/
│   ├── memory/
│   │   ├── constitution.md
│   │   └── constitution_update_checklist.md
│   ├── scripts/
│   │   └── bash/
│   │       ├── check-task-prerequisites.sh
│   │       ├── common.sh
│   │       ├── create-new-feature.sh
│   │       ├── get-feature-paths.sh
│   │       ├── setup-plan.sh
│   │       └── update-agent-context.sh
│   └── templates/
│       ├── agent-file-template.md
│       ├── plan-template.md
│       ├── spec-template.md
│       └── tasks-template.md
```

### The seven slash commands

| Command | Purpose | Phase |
|---|---|---|
| `/speckit.constitution` | Set non-negotiable project principles (tech stack, testing standards) | Foundation |
| `/speckit.specify` | Define **what** to build and **why** (requirements, user stories) | Foundation |
| `/speckit.clarify` | AI asks sequential questions to resolve ambiguities | Foundation |
| `/speckit.plan` | Define **how** — frameworks, architecture, dependencies | Implementation |
| `/speckit.tasks` | Break spec + plan into phased, actionable tasks | Implementation |
| `/speckit.analyze` | Cross-artifact consistency validation against constitution | Quality Gate |
| `/speckit.implement` | Execute all tasks to build the feature | Execution |

### Workflow for sai-code

After `specify init`, run `/speckit.constitution` in Claude Code with this input:

> "This project is sai-code, a Rust CLI coding agent. Tech stack: Rust 2021 edition, Cargo workspace, hexagonal architecture with ports-and-adapters. Core crates: genai for LLM, rmcp for MCP, ratatui for TUI, tree-sitter for code analysis, clap for CLI, tokio async runtime. All business logic lives in domain crate with zero infrastructure dependencies. Error handling: thiserror for domain errors, color-eyre for application. TDD with cargo-nextest. All public APIs must have doc comments."

Then for each feature (agent loop, TUI, tool system, etc.), run the specify → clarify → plan → tasks → implement cycle. Each feature produces a `specs/NNN-feature-name/` directory with `spec.md`, `plan.md`, `research.md`, `data-model.md`, `tasks.md`, and optionally `contracts/` and `quickstart.md`.

---

## Part 2: Complete Cargo workspace architecture

### Workspace layout

```
sai-code/
├── Cargo.toml                 # Workspace root
├── Cargo.lock                 # Tracked (binary project)
├── crates/
│   ├── sai-core/              # Domain: entities, port traits, agent loop logic
│   ├── sai-llm/               # Adapter: genai-based multi-provider LLM
│   ├── sai-mcp/               # Adapter: rmcp-based MCP client/server
│   ├── sai-tools/             # Adapter: built-in tool implementations
│   ├── sai-analysis/          # Adapter: tree-sitter code analysis
│   ├── sai-tui/               # Adapter: ratatui TUI interface
│   ├── sai-config/            # Infrastructure: TOML config loading
│   ├── sai-permissions/       # Infrastructure: permission enforcement
│   ├── sai-context/           # Infrastructure: context compression/memory
│   └── sai-cli/               # Application: binary entry point, DI wiring
├── specs/                     # SpecKit feature specifications
├── tests/                     # Integration/E2E tests
└── fixtures/                  # Test data, mock LLM responses
```

### Workspace root Cargo.toml

```toml
[workspace]
resolver = "2"
members = [
    "crates/sai-core",
    "crates/sai-llm",
    "crates/sai-mcp",
    "crates/sai-tools",
    "crates/sai-analysis",
    "crates/sai-tui",
    "crates/sai-config",
    "crates/sai-permissions",
    "crates/sai-context",
    "crates/sai-cli",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.80.0"
authors = ["Brice Fontaine"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/b-fontaine/sai-code"
description = "An AI-powered CLI coding agent"

[workspace.dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Error handling
thiserror = "2"
color-eyre = "0.6"

# Logging/tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# LLM abstraction
genai = "0.5"

# MCP protocol
rmcp = { version = "1.3", features = ["client", "transport-io"] }

# TUI
ratatui = { version = "0.30", features = ["all-widgets"] }
crossterm = "0.28"

# Code analysis
tree-sitter = "0.26"

# CLI
clap = { version = "4.6", features = ["derive", "env"] }

# Async traits
async-trait = "0.1"

# UUID for session IDs
uuid = { version = "1", features = ["v4", "serde"] }

# Filesystem
notify = "7"
walkdir = "2"
globset = "0.4"
ignore = "0.4"

# HTTP (for LLM health checks, etc.)
reqwest = { version = "0.12", features = ["json"] }

# Testing
mockall = "0.13"
tempfile = "3"
assert_cmd = "2"
predicates = "3"
tokio-test = "0.4"

# Internal crates
sai-core = { path = "crates/sai-core" }
sai-llm = { path = "crates/sai-llm" }
sai-mcp = { path = "crates/sai-mcp" }
sai-tools = { path = "crates/sai-tools" }
sai-analysis = { path = "crates/sai-analysis" }
sai-tui = { path = "crates/sai-tui" }
sai-config = { path = "crates/sai-config" }
sai-permissions = { path = "crates/sai-permissions" }
sai-context = { path = "crates/sai-context" }

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
must_use_candidate = "allow"
missing_errors_doc = "allow"
missing_panics_doc = "allow"
unwrap_used = "warn"
expect_used = "warn"

[workspace.lints.rust]
unsafe_code = "deny"
missing_docs = "warn"
```

### Crate responsibilities and dependency graph

```
sai-cli ──┬── sai-tui ──────── sai-core
           ├── sai-llm ─────── sai-core
           ├── sai-mcp ─────── sai-core
           ├── sai-tools ───── sai-core
           ├── sai-analysis ── sai-core
           ├── sai-config ──── sai-core
           ├── sai-permissions ── sai-core
           ├── sai-context ──── sai-core
           └── sai-core (direct for wiring)

Arrow direction: "depends on"
sai-core has ZERO infrastructure dependencies.
```

### sai-core — Domain layer (zero infrastructure deps)

```toml
# crates/sai-core/Cargo.toml
[package]
name = "sai-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Core domain: entities, port traits, agent loop"

[dependencies]
thiserror.workspace = true
serde.workspace = true
serde_json.workspace = true
async-trait.workspace = true
uuid.workspace = true
tokio = { workspace = true, features = ["sync"] }

[dev-dependencies]
mockall.workspace = true
tokio-test.workspace = true

[lints]
workspace = true
```

**Public API surface — port traits:**

```rust
// crates/sai-core/src/lib.rs
pub mod domain;    // Entities: Session, Message, ToolCall, ToolResult, ConversationTurn
pub mod ports;     // Trait definitions (the hexagonal "ports")
pub mod services;  // Use-case orchestration (AgentLoop, ContextManager)
pub mod error;     // Domain error types

// crates/sai-core/src/ports/llm.rs
#[async_trait]
pub trait LlmPort: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LlmError>;
    async fn chat_stream(&self, request: ChatRequest) -> Result<ChatStream, LlmError>;
    fn model_name(&self) -> &str;
    fn provider_name(&self) -> &str;
}

// crates/sai-core/src/ports/tool.rs
#[async_trait]
pub trait ToolPort: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> serde_json::Value;
    async fn execute(&self, input: serde_json::Value) -> Result<ToolOutput, ToolError>;
}

pub trait ToolRegistryPort: Send + Sync {
    fn register(&mut self, tool: Box<dyn ToolPort>);
    fn get(&self, name: &str) -> Option<&dyn ToolPort>;
    fn list(&self) -> Vec<&dyn ToolPort>;
}

// crates/sai-core/src/ports/mcp.rs
#[async_trait]
pub trait McpClientPort: Send + Sync {
    async fn connect(&mut self, config: McpServerConfig) -> Result<(), McpError>;
    async fn list_tools(&self) -> Result<Vec<McpToolInfo>, McpError>;
    async fn call_tool(&self, name: &str, args: serde_json::Value) -> Result<ToolOutput, McpError>;
    async fn disconnect(&mut self) -> Result<(), McpError>;
}

// crates/sai-core/src/ports/ui.rs
#[async_trait]
pub trait UiPort: Send + Sync {
    async fn render_message(&self, message: &Message) -> Result<(), UiError>;
    async fn render_tool_call(&self, call: &ToolCall) -> Result<(), UiError>;
    async fn get_user_input(&self) -> Result<String, UiError>;
    async fn request_permission(&self, request: &PermissionRequest) -> Result<bool, UiError>;
}

// crates/sai-core/src/ports/context.rs
#[async_trait]
pub trait ContextPort: Send + Sync {
    async fn compress(&self, messages: &[Message]) -> Result<Vec<Message>, ContextError>;
    fn token_count(&self, messages: &[Message]) -> usize;
    fn within_budget(&self, messages: &[Message], budget: usize) -> bool;
}

// crates/sai-core/src/ports/permissions.rs
#[async_trait]
pub trait PermissionPort: Send + Sync {
    async fn check(&self, request: &PermissionRequest) -> Result<PermissionDecision, PermissionError>;
    async fn grant_persistent(&mut self, rule: PermissionRule) -> Result<(), PermissionError>;
    async fn revoke(&mut self, rule_id: &str) -> Result<(), PermissionError>;
}

// crates/sai-core/src/ports/analysis.rs
#[async_trait]
pub trait CodeAnalysisPort: Send + Sync {
    async fn parse_file(&self, path: &Path, content: &str) -> Result<SyntaxInfo, AnalysisError>;
    async fn find_symbols(&self, path: &Path, content: &str) -> Result<Vec<Symbol>, AnalysisError>;
    async fn get_outline(&self, path: &Path, content: &str) -> Result<FileOutline, AnalysisError>;
}

// crates/sai-core/src/ports/config.rs
pub trait ConfigPort: Send + Sync {
    fn get_provider_config(&self, name: &str) -> Option<&ProviderConfig>;
    fn get_active_model(&self) -> &ModelConfig;
    fn get_all_providers(&self) -> &[ProviderConfig];
    fn get_mcp_servers(&self) -> &[McpServerConfig];
    fn get_permission_rules(&self) -> &[PermissionRule];
}
```

### sai-llm — LLM adapter

```toml
# crates/sai-llm/Cargo.toml
[package]
name = "sai-llm"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "LLM adapter: genai-based multi-provider support"

[dependencies]
sai-core.workspace = true
genai.workspace = true
tokio.workspace = true
async-trait.workspace = true
tracing.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true

[dev-dependencies]
mockall.workspace = true
tokio-test.workspace = true

[lints]
workspace = true
```

genai determines the provider by model name string. A single `genai::Client` handles all providers — switching providers at runtime means changing the model string:

```rust
// Provider is inferred from model name:
// "gpt-4o" → OpenAI, "claude-sonnet-4-20250514" → Anthropic
// "gemini-2.0-flash" → Gemini, "llama3:8b" → Ollama
// Namespaced: "ollama::mistral", "groq::llama-3.1-8b-instant"
```

### sai-mcp — MCP adapter

```toml
# crates/sai-mcp/Cargo.toml
[package]
name = "sai-mcp"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "MCP adapter: rmcp-based Model Context Protocol client"

[dependencies]
sai-core.workspace = true
rmcp = { workspace = true, features = ["client", "transport-io"] }
tokio.workspace = true
async-trait.workspace = true
tracing.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true

[lints]
workspace = true
```

rmcp supports stdio and SSE transports. The `#[tool]` macro auto-generates tool registration and JSON Schema. For sai-code as an MCP **client**, it connects to external MCP servers via `TokioChildProcess` (stdio) or SSE.

### sai-tools — Built-in tools

```toml
# crates/sai-tools/Cargo.toml
[package]
name = "sai-tools"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Built-in tools: file ops, shell, search, code editing"

[dependencies]
sai-core.workspace = true
tokio.workspace = true
async-trait.workspace = true
walkdir.workspace = true
globset.workspace = true
ignore.workspace = true
tracing.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true

[lints]
workspace = true
```

### sai-analysis — Code analysis adapter

```toml
# crates/sai-analysis/Cargo.toml
[package]
name = "sai-analysis"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Code analysis adapter: tree-sitter-based parsing"

[dependencies]
sai-core.workspace = true
tree-sitter.workspace = true
async-trait.workspace = true
thiserror.workspace = true

[dependencies.tree-sitter-rust]
version = "0.23"
optional = true

[dependencies.tree-sitter-javascript]
version = "0.23"
optional = true

[dependencies.tree-sitter-typescript]
version = "0.23"
optional = true

[dependencies.tree-sitter-python]
version = "0.23"
optional = true

[features]
default = ["lang-rust", "lang-javascript", "lang-typescript", "lang-python"]
lang-rust = ["dep:tree-sitter-rust"]
lang-javascript = ["dep:tree-sitter-javascript"]
lang-typescript = ["dep:tree-sitter-typescript"]
lang-python = ["dep:tree-sitter-python"]

[lints]
workspace = true
```

### sai-tui — TUI adapter

```toml
# crates/sai-tui/Cargo.toml
[package]
name = "sai-tui"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "TUI adapter: ratatui-based terminal interface"

[dependencies]
sai-core.workspace = true
ratatui.workspace = true
crossterm.workspace = true
tokio.workspace = true
async-trait.workspace = true
tracing.workspace = true
thiserror.workspace = true

[lints]
workspace = true
```

### sai-config — Configuration

```toml
# crates/sai-config/Cargo.toml
[package]
name = "sai-config"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Configuration loading from TOML files and environment"

[dependencies]
sai-core.workspace = true
serde.workspace = true
toml.workspace = true
thiserror.workspace = true
tracing.workspace = true

[dependencies.directories]
version = "5"

[lints]
workspace = true
```

### sai-permissions — Permission system

```toml
# crates/sai-permissions/Cargo.toml
[package]
name = "sai-permissions"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Permission enforcement: approval gates, persistent rules"

[dependencies]
sai-core.workspace = true
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
async-trait.workspace = true
thiserror.workspace = true
tracing.workspace = true
globset.workspace = true

[lints]
workspace = true
```

### sai-context — Context compression

```toml
# crates/sai-context/Cargo.toml
[package]
name = "sai-context"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Context window management: compression, token counting, memory"

[dependencies]
sai-core.workspace = true
tokio.workspace = true
async-trait.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tracing.workspace = true

[lints]
workspace = true
```

### sai-cli — Application entry point

```toml
# crates/sai-cli/Cargo.toml
[package]
name = "sai-cli"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "sai CLI binary: wires all adapters together"

[[bin]]
name = "sai"
path = "src/main.rs"

[dependencies]
sai-core.workspace = true
sai-llm.workspace = true
sai-mcp.workspace = true
sai-tools.workspace = true
sai-analysis.workspace = true
sai-tui.workspace = true
sai-config.workspace = true
sai-permissions.workspace = true
sai-context.workspace = true
clap.workspace = true
tokio.workspace = true
color-eyre.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
serde.workspace = true

[lints]
workspace = true
```

### TOML configuration format

The user configuration lives at `~/.config/sai/config.toml` (XDG) with project overrides at `.sai/config.toml`:

```toml
# ~/.config/sai/config.toml — Global sai-code configuration

# Active model selection
[model]
default = "claude-sonnet-4-20250514"    # Model string (genai resolves provider automatically)
fallback = "ollama::llama3:8b"          # Fallback if primary fails

# Provider-specific configuration
[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"       # Env var containing the key (never store keys directly)
max_tokens = 8192
temperature = 0.3

[providers.openai]
api_key_env = "OPENAI_API_KEY"
max_tokens = 4096
temperature = 0.2

[providers.gemini]
api_key_env = "GEMINI_API_KEY"
max_tokens = 8192

[providers.ollama]
base_url = "http://localhost:11434"     # Custom Ollama endpoint
# No API key needed for local Ollama

# Context management
[context]
max_tokens = 128000                     # Context window budget
compression_threshold = 0.8            # Compress when 80% full
preserve_recent = 10                   # Always keep last N turns uncompressed

# MCP server connections
[[mcp.servers]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/projects"]
transport = "stdio"

[[mcp.servers]]
name = "github"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
transport = "stdio"
env = { GITHUB_TOKEN = "${GITHUB_TOKEN}" }

# Permission rules
[permissions]
# Auto-approve read operations
auto_approve = ["file:read", "directory:list", "search:*"]
# Always require approval for these
require_approval = ["file:write", "file:delete", "shell:execute"]
# Never allow (even with approval)
deny = ["shell:rm -rf /", "file:write:~/.ssh/*"]

# TUI preferences
[tui]
theme = "dark"                         # "dark" | "light" | "auto"
show_token_count = true
show_cost_estimate = true
markdown_rendering = true
```

---

## Part 3: GitHub Actions CI/CD pipeline

### `.github/workflows/ci.yml`

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
  schedule:
    - cron: '00 04 * * MON'   # Weekly Monday 4am UTC

permissions:
  contents: read

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  check:
    name: Compile check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo check --workspace --all-targets

  fmt:
    name: Formatting
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all --check

  clippy:
    name: Clippy lints
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --workspace --all-targets -- -D warnings

  test:
    name: Tests (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@cargo-nextest
      - run: cargo nextest run --workspace
      - run: cargo test --workspace --doc

  docs:
    name: Documentation
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo doc --no-deps --document-private-items --workspace
        env:
          RUSTDOCFLAGS: -D warnings

  deny-licenses:
    name: License & ban check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2
        with:
          command: check bans licenses sources

  deny-advisories:
    name: Security advisories
    runs-on: ubuntu-latest
    continue-on-error: true    # Don't block CI on new advisories
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2
        with:
          command: check advisories
```

### `.github/workflows/release.yml`

```yaml
name: Release

on:
  push:
    tags:
      - 'v[0-9]+.*'

permissions:
  contents: write

jobs:
  create-release:
    name: Create GitHub release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: taiki-e/create-gh-release-action@v1
        with:
          changelog: CHANGELOG.md
          token: ${{ secrets.GITHUB_TOKEN }}

  build-and-upload:
    name: Build ${{ matrix.target }}
    needs: create-release
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-pc-windows-msvc
            os: windows-latest
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: taiki-e/upload-rust-binary-action@v1
        with:
          bin: sai
          target: ${{ matrix.target }}
          token: ${{ secrets.GITHUB_TOKEN }}
          tar: unix
          zip: windows
          checksum: sha256
```

### `.github/dependabot.yml`

```yaml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
      day: "monday"
    groups:
      rust-dependencies:
        patterns: ["*"]
        update-types: ["minor", "patch"]
    commit-message:
      prefix: "deps"
    open-pull-requests-limit: 10
    labels: ["dependencies", "rust"]

  - package-ecosystem: "github-actions"
    directory: "/"
    schedule:
      interval: "weekly"
    commit-message:
      prefix: "ci"
    labels: ["dependencies", "ci"]
```

### Branch protection recommendations

Configure these on `main`:
- Require pull request reviews (1 reviewer minimum)
- Require status checks to pass: `check`, `fmt`, `clippy`, `test` (all three OS), `docs`, `deny-licenses`
- Require branches to be up to date before merging
- Require signed commits (optional but recommended)
- Do not allow force pushes
- Do not allow deletions

---

## Part 4: CLAUDE.md

```markdown
# sai-code

AI-powered CLI coding agent built in Rust with hexagonal architecture.

## Architecture

Cargo workspace with ports-and-adapters pattern. Domain logic has zero infrastructure deps.

- `crates/sai-core` — Domain entities, port traits, agent loop. THE source of truth.
- `crates/sai-llm` — LLM adapter (genai). Implements `LlmPort`.
- `crates/sai-mcp` — MCP adapter (rmcp). Implements `McpClientPort`.
- `crates/sai-tools` — Built-in tools (file, shell, search). Implements `ToolPort`.
- `crates/sai-analysis` — Code analysis (tree-sitter). Implements `CodeAnalysisPort`.
- `crates/sai-tui` — TUI (ratatui + crossterm). Implements `UiPort`.
- `crates/sai-config` — TOML config loading. Implements `ConfigPort`.
- `crates/sai-permissions` — Permission enforcement. Implements `PermissionPort`.
- `crates/sai-context` — Context compression. Implements `ContextPort`.
- `crates/sai-cli` — Binary entry point. Wires adapters to ports via DI.

## Build & test

```
cargo check --workspace                  # Fast compile check
cargo build --workspace                  # Debug build
cargo nextest run --workspace            # Run all tests (preferred)
cargo test --workspace --doc             # Doc tests
cargo clippy --workspace --all-targets -- -D warnings  # Lint
cargo fmt --all --check                  # Format check
cargo deny check                         # Supply chain security
just ci                                  # Run full CI locally
```

## Code conventions

- **Error handling:** `thiserror` in all crates for typed errors. `color-eyre` only in `sai-cli`.
  Every domain operation gets its own error enum. Include `Unknown(#[from] Box<dyn std::error::Error + Send + Sync>)` variant for unexpected errors.
- **Async:** All port traits use `#[async_trait]`. Runtime is tokio.
- **Dependencies flow inward:** Adapters depend on `sai-core`, never the reverse. `sai-core` has zero infra deps.
- **Traits as ports:** Every external boundary is a trait in `sai-core/src/ports/`.
- **No unwrap/expect in production code.** Use `?` operator. Clippy enforces this.
- **All public items get doc comments.** `#[deny(missing_docs)]` on library crates.
- **Tests go next to code** (`#[cfg(test)] mod tests`) for unit tests. Integration tests in `tests/`.

## Architecture rules

1. Never add infrastructure dependencies (genai, rmcp, ratatui, etc.) to sai-core.
2. New external integrations = new port trait in sai-core + new adapter crate.
3. sai-cli is the composition root — it constructs concrete adapters and passes them as trait objects.
4. Domain entities must self-validate on construction (use builder pattern or `new()` that returns Result).
5. Prefer `impl Trait` for single concrete type, `Box<dyn Trait>` for runtime polymorphism.

## Common pitfalls

- Don't leak adapter types into domain — always convert to domain types at adapter boundary.
- genai model strings encode the provider (e.g., "claude-sonnet-4-20250514" → Anthropic). Don't hardcode providers separately.
- rmcp 1.x has breaking changes from 0.x — use the `#[tool]` macro API, not the old manual registration.
- ratatui 0.30 requires crossterm 0.28 — version mismatch causes compile errors.
- Run `cargo deny check` before adding new dependencies to catch license issues early.
```

---

## Part 5: Repository initialization files

### README.md

```markdown
# sai-code

**sai** is an AI-powered CLI coding agent that helps you write, understand, and refactor code directly from your terminal.

> ⚠️ **Early development** — not yet ready for production use.

## Features

- 🤖 **Multi-provider LLM support** — Claude, GPT, Gemini, Ollama (local), and more via genai
- 🔧 **Built-in tools** — File editing, shell commands, code search, and syntax analysis
- 🔌 **MCP integration** — Connect to any Model Context Protocol server for extensibility
- 🖥️ **Rich TUI** — Interactive terminal interface with streaming responses
- 🔒 **Permission system** — Granular approval gates for destructive operations
- 📦 **Context management** — Automatic compression to maximize effective context window

## Installation

### From source

```bash
cargo install --git https://github.com/b-fontaine/sai-code sai-cli
```

### From release binaries

Download the latest binary for your platform from [Releases](https://github.com/b-fontaine/sai-code/releases).

## Quick start

```bash
# Configure your LLM provider
export ANTHROPIC_API_KEY="sk-ant-..."

# Start sai in the current directory
sai

# Or use a specific model
sai --model gpt-4o

# Use local Ollama
sai --model ollama::llama3:8b
```

## Configuration

Create `~/.config/sai/config.toml`:

```toml
[model]
default = "claude-sonnet-4-20250514"

[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"

[providers.ollama]
base_url = "http://localhost:11434"
```

See [docs/configuration.md](docs/configuration.md) for full reference.

## Architecture

sai-code uses hexagonal architecture (ports and adapters) with a Cargo workspace:

| Crate | Purpose |
|---|---|
| `sai-core` | Domain entities and port traits (zero infra deps) |
| `sai-llm` | Multi-provider LLM adapter via genai |
| `sai-mcp` | MCP client via rmcp |
| `sai-tools` | Built-in tool implementations |
| `sai-tui` | Terminal UI via ratatui |

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
```

### CONTRIBUTING.md

```markdown
# Contributing to sai-code

Thank you for your interest in contributing! This document covers the development setup and guidelines.

## Development setup

1. Install Rust via [rustup](https://rustup.rs/) (MSRV: 1.80.0)
2. Install [just](https://github.com/casey/just) for task running
3. Install [cargo-nextest](https://nexte.st/) for testing
4. Install [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) for supply chain checks

```bash
cargo install just cargo-nextest cargo-deny
```

## Workflow

1. Fork the repo and create a feature branch from `main`
2. Write tests first (TDD) — all new functionality needs tests
3. Implement the feature
4. Run the full CI suite locally: `just ci`
5. Open a pull request

## Code style

- Run `cargo fmt --all` before committing
- All clippy warnings must be resolved: `cargo clippy --workspace --all-targets -- -D warnings`
- Follow the hexagonal architecture — see CLAUDE.md for rules
- Use `thiserror` for error types, never `anyhow` in library crates
- All public APIs need doc comments

## Testing

- Unit tests: next to the code in `#[cfg(test)] mod tests`
- Integration tests: in `tests/` directory
- Use `mockall` for mocking port traits
- Run with: `cargo nextest run --workspace`

## Architecture decision records

Major architecture decisions should be discussed in a GitHub issue before implementation. Reference the SpecKit spec documents in `specs/` for feature requirements.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/):
- `feat:` new feature
- `fix:` bug fix
- `refactor:` code restructuring
- `docs:` documentation only
- `test:` adding/updating tests
- `ci:` CI/CD changes
- `deps:` dependency updates
```

### .gitignore

```gitignore
# Rust build artifacts
debug/
target/

# Backup files from rustfmt
**/*.rs.bk

# MSVC debugging
*.pdb

# Mutation testing
**/mutants.out*/

# Editor/IDE
.vscode/
!.vscode/settings.json
!.vscode/extensions.json
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db

# Environment
.env
.env.local

# Coverage
lcov.info
tarpaulin-report.html
coverage/

# SpecKit local state (keep templates, ignore generated runtime state)
# .specify/scripts/ is committed; local tool state is not
```

### rustfmt.toml

```toml
edition = "2021"
max_width = 100
hard_tabs = false
tab_spaces = 4
reorder_imports = true
reorder_modules = true
merge_derives = true
use_small_heuristics = "Default"
```

### clippy.toml

```toml
cognitive-complexity-threshold = 30
too-many-arguments-threshold = 10
```

### deny.toml

```toml
[graph]
targets = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
]
all-features = true

[advisories]
vulnerability = "deny"
unmaintained = "warn"
unsound = "warn"
yanked = "warn"
notice = "warn"

[licenses]
unlicensed = "deny"
copyleft = "deny"
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    "Zlib",
    "BSL-1.0",
    "0BSD",
    "CC0-1.0",
    "OpenSSL",
]
confidence-threshold = 0.8

[bans]
multiple-versions = "warn"
wildcards = "deny"
highlight = "simplest-path"

[sources]
unknown-registry = "warn"
unknown-git = "warn"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

### justfile

```just
set dotenv-load

# Show available commands
default:
    @just --list

# ─── Development ───

# Fast compile check
check:
    cargo check --workspace --all-targets

# Debug build
build:
    cargo build --workspace

# Release build
build-release:
    cargo build --workspace --release

# Run sai with arguments
run *args='':
    cargo run -p sai-cli -- {{args}}

# ─── Quality ───

# Run all lints
lint: fmt-check clippy

# Check formatting
fmt-check:
    cargo fmt --all --check

# Apply formatting
fmt:
    cargo fmt --all

# Run clippy
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# ─── Testing ───

# Run all tests with nextest
test:
    cargo nextest run --workspace

# Run doc tests (nextest doesn't support these)
test-doc:
    cargo test --workspace --doc

# Run tests for a specific crate
test-crate crate:
    cargo nextest run -p {{crate}}

# ─── Security ───

# Run cargo-deny checks
deny:
    cargo deny check

# Run security audit
audit:
    cargo audit

# ─── Documentation ───

# Build and open docs
docs:
    cargo doc --workspace --no-deps --open

# ─── CI ───

# Run full CI pipeline locally
ci: check lint test test-doc docs deny

# ─── Maintenance ───

# Clean build artifacts
clean:
    cargo clean

# Update dependencies
update:
    cargo update

# Show dependency tree
deps:
    cargo tree --workspace
```

### License

Use **MIT OR Apache-2.0** dual license (Rust community standard). Create two files:

- `LICENSE-MIT` — Standard MIT license text with "Copyright (c) 2025 Brice Fontaine"
- `LICENSE-APACHE` — Standard Apache 2.0 license text

In every `Cargo.toml`: `license = "MIT OR Apache-2.0"`

---

## Part 6: SpecKit specification documents

These specifications should live in the `specs/` directory after running `/speckit.specify` for each feature. Below is the content for each spec.md following SpecKit's Markdown template format.

### `specs/001-agent-loop/spec.md`

```markdown
# Spec: Core Agent Loop

## Overview
The core agent loop is the central orchestrator of sai-code. It implements an observe-think-act-verify cycle that continuously processes user input, reasons about actions, executes tools, and verifies results.

## User stories
- As a developer, I want to type a natural language request and have sai take autonomous action to fulfill it.
- As a developer, I want sai to stop and ask for permission before performing destructive operations.
- As a developer, I want to see streaming output as the LLM reasons about my request.
- As a developer, I want the agent to self-correct when a tool execution fails.

## Functional requirements
1. The agent loop receives user input and constructs a prompt with system context, conversation history, and available tools.
2. The loop sends the prompt to the configured LLM via `LlmPort` and streams the response.
3. When the LLM emits a tool call, the loop dispatches it via `ToolRegistryPort` after checking `PermissionPort`.
4. Tool results are appended to conversation history and fed back to the LLM for the next reasoning step.
5. The loop continues until the LLM produces a final text response with no tool calls.
6. If context approaches the budget limit, `ContextPort::compress` is invoked automatically.
7. Errors from tool execution are formatted and fed back to the LLM so it can self-correct.
8. The loop supports cancellation via Ctrl+C, cleanly aborting in-progress operations.

## Non-functional requirements
- Latency: first token streamed to TUI within 500ms of LLM response start.
- The agent loop runs on a single tokio task with channels for UI communication.
- All state transitions must be logged via tracing at DEBUG level.
- The loop must be testable with mock LLM and tool implementations.

## Domain entities
- `Session`: Contains conversation history, active model, tool registry, session metadata.
- `ConversationTurn`: A (user_message, assistant_response, tool_calls[]) triple.
- `AgentState`: Enum — `WaitingForInput`, `Reasoning`, `ExecutingTool`, `AwaitingPermission`, `Compressing`, `Complete`.
```

### `specs/002-llm-abstraction/spec.md`

```markdown
# Spec: Multi-Provider LLM Abstraction

## Overview
sai-code must support multiple LLM providers (Anthropic Claude, OpenAI GPT, Google Gemini, local Ollama) through a single abstraction. The genai crate provides the adapter implementation, while sai-core defines the port trait.

## User stories
- As a developer, I want to switch between LLM providers by changing a single config value or CLI flag.
- As a developer, I want to use local Ollama models for offline/private coding.
- As a developer, I want streaming responses from all providers.
- As a developer, I want to configure API keys via environment variables, not config files.

## Functional requirements
1. `LlmPort` trait supports both streaming (`chat_stream`) and non-streaming (`chat`) completions.
2. Provider selection is determined by the model name string (genai convention): "claude-sonnet-4-20250514" → Anthropic, "gpt-4o" → OpenAI, "gemini-2.0-flash" → Gemini, "ollama::llama3" → Ollama.
3. The adapter reads API keys from environment variables specified in config (`api_key_env` field).
4. Chat options (temperature, max_tokens) are configurable per-provider in config.toml.
5. Tool/function calling schemas are passed to the LLM via `ChatRequest` tools field.
6. The adapter converts genai's streaming events into domain `ChatStream` events.
7. A fallback model can be configured for automatic failover.
8. Health check method to verify provider connectivity before starting a session.

## Non-functional requirements
- Provider switching must not require application restart.
- API key errors must produce clear, actionable error messages.
- Token usage must be tracked per-request for cost estimation in the TUI.
```

### `specs/003-tool-system/spec.md`

```markdown
# Spec: Tool System

## Overview
The tool system provides sai with the ability to interact with the filesystem, execute commands, search code, and perform structured edits. Tools implement the `ToolPort` trait and are registered in a `ToolRegistry`.

## User stories
- As a developer, I want sai to read and write files in my project.
- As a developer, I want sai to run shell commands and see their output.
- As a developer, I want sai to search my codebase for patterns and symbols.
- As a developer, I want to extend sai with custom tools via MCP servers.

## Functional requirements

### Built-in tools
1. **file_read** — Read file contents, with optional line range.
2. **file_write** — Write full file contents (create or overwrite).
3. **file_edit** — Apply targeted edits using search-and-replace blocks (not full file rewrites).
4. **directory_list** — List directory contents with optional glob filtering.
5. **shell_execute** — Run a shell command with timeout, capture stdout/stderr/exit code.
6. **code_search** — Regex search across project files (respects .gitignore).
7. **symbol_search** — Find function/type definitions using tree-sitter.

### Tool registry
8. All tools self-describe via `name()`, `description()`, and `input_schema()` (JSON Schema).
9. Tools are registered at startup. MCP tools are lazily discovered on first connection.
10. The registry merges built-in tools with MCP-provided tools, with built-in tools taking priority on name conflicts.

### Execution
11. Every tool execution goes through the permission system before running.
12. Tool execution has a configurable timeout (default: 30s for shell, no limit for file ops).
13. Tool output is truncated to a configurable maximum to avoid context explosion.

## Non-functional requirements
- File operations must respect `.gitignore` patterns.
- Shell execution must be sandboxed to the project directory.
- Tool schemas must be valid JSON Schema for LLM function calling compatibility.
```

### `specs/004-permission-system/spec.md`

```markdown
# Spec: Permission System

## Overview
The permission system implements defense-in-depth safety for tool execution. It prevents the agent from performing destructive operations without explicit user consent, while allowing safe read operations to proceed automatically.

## User stories
- As a developer, I want read operations (file read, search) to execute without interrupting my flow.
- As a developer, I want to approve or deny file writes and shell commands individually.
- As a developer, I want to "always allow" specific patterns (e.g., writing to test files).
- As a developer, I want to see exactly what the agent is about to do before approving.

## Functional requirements
1. Three permission tiers: **auto-approve**, **require-approval**, **deny**.
2. Permission rules use glob patterns for matching (e.g., `file:write:tests/**` → auto-approve).
3. Persistent rules stored in `.sai/permissions.json` per project and `~/.config/sai/permissions.json` globally.
4. When approval is required, the TUI displays the tool name, arguments, and a human-readable description of what will happen.
5. User can respond: Allow (once), Allow Always (creates persistent rule), Deny (once), Deny Always.
6. A "turbo mode" flag disables all approval prompts (for trusted environments / CI).
7. Hardcoded deny list for dangerous patterns (e.g., `rm -rf /`, writing to `.ssh/`) that cannot be overridden.

## Non-functional requirements
- Permission check latency: <1ms for cached decisions.
- Permission rules are loaded once at startup and cached in memory.
- Rules file format must be human-readable and hand-editable.
```

### `specs/005-context-compression/spec.md`

```markdown
# Spec: Context Compression

## Overview
Context compression manages the LLM's finite context window. When conversation history approaches the token budget, earlier turns are summarized while preserving key facts. This lets long sessions continue without losing critical context.

## User stories
- As a developer, I want long coding sessions to work without hitting context limits.
- As a developer, I want the agent to remember what files were modified even after compression.
- As a developer, I want to manually trigger compression with specific preservation instructions.

## Functional requirements
1. Token counting: estimate token count for messages using a simple heuristic (chars / 4) or provider-specific tokenizer.
2. Automatic compression triggers when conversation fills `compression_threshold` (default 80%) of `max_tokens`.
3. Compression strategy: summarize old turns while preserving the most recent N turns verbatim.
4. The summary must retain: list of modified files, current task status, key decisions made, error patterns encountered.
5. Manual compression via `/compact` command, with optional focus instructions.
6. Session memory: cross-session persistence of key facts in a `.sai/memory.md` file.
7. The compression request itself goes through the LLM (asking it to summarize the conversation).

## Non-functional requirements
- Compression should reduce token count by at least 60% while preserving actionable context.
- The token counter must handle multi-byte characters correctly.
- Memory file must be human-readable Markdown.
```

### `specs/006-tui/spec.md`

```markdown
# Spec: Terminal User Interface

## Overview
The TUI provides an interactive terminal experience using ratatui and crossterm. It renders streaming LLM responses, tool execution status, and permission prompts in a rich terminal layout.

## User stories
- As a developer, I want to see LLM responses stream in real-time with markdown formatting.
- As a developer, I want to see which tool is executing and its progress.
- As a developer, I want an input area where I can type multi-line messages.
- As a developer, I want to scroll back through conversation history.

## Functional requirements
1. Layout: three regions — conversation history (scrollable), status bar, input area.
2. Streaming display: tokens appear as they arrive from the LLM, with markdown rendering.
3. Tool call display: show tool name and arguments in a distinct style, followed by result.
4. Permission prompt: modal overlay displaying the action details with Accept/Deny buttons.
5. Input: multi-line text input with Ctrl+Enter to submit, Up arrow for history.
6. Status bar: shows active model, token count, estimated cost, and agent state.
7. Keyboard shortcuts: Ctrl+C to cancel current operation, Ctrl+D to quit, `/` prefix for commands.
8. Slash commands: `/compact`, `/model <name>`, `/clear`, `/help`, `/quit`.
9. Alternative: non-TUI streaming mode for piped/non-interactive usage (plain text output).

## Non-functional requirements
- Must render at 60fps with no flicker during streaming.
- Works in terminals with minimum 80x24 dimensions.
- Supports both dark and light terminal backgrounds.
- Graceful degradation: if terminal doesn't support features, fall back to simpler rendering.
```

### `specs/007-configuration/spec.md`

```markdown
# Spec: Configuration System

## Overview
The configuration system loads settings from TOML files with a layered precedence: CLI flags > environment variables > project config (.sai/config.toml) > user config (~/.config/sai/config.toml) > built-in defaults.

## User stories
- As a developer, I want project-specific model and tool configuration.
- As a developer, I want global defaults that apply to all my projects.
- As a developer, I want to override any config value via CLI flags or env vars.

## Functional requirements
1. Config resolution order (highest to lowest priority): CLI args → env vars → `.sai/config.toml` → `~/.config/sai/config.toml` → built-in defaults.
2. Config sections: `[model]`, `[providers.*]`, `[context]`, `[mcp.servers]`, `[permissions]`, `[tui]`.
3. Provider configs include: `api_key_env`, `base_url`, `max_tokens`, `temperature`.
4. MCP server configs include: `name`, `command`, `args`, `transport`, `env`.
5. Config validation at load time with actionable error messages.
6. `sai config show` subcommand to display resolved configuration.
7. `sai config init` subcommand to create a template config file interactively.
8. API keys are never stored in config files — only the env var name is stored.

## Non-functional requirements
- Config loading must complete in <50ms.
- Config file errors must point to the exact line and field that is invalid.
- The config format must be forward-compatible (unknown fields are ignored with a warning).
```

### `specs/008-mcp-integration/spec.md`

```markdown
# Spec: MCP Integration

## Overview
MCP (Model Context Protocol) integration allows sai-code to connect to external MCP servers, discovering and using their tools alongside built-in tools. This makes sai extensible without code changes.

## User stories
- As a developer, I want to connect sai to MCP servers for GitHub, databases, or custom APIs.
- As a developer, I want MCP tools to appear seamlessly alongside built-in tools.
- As a developer, I want to configure MCP servers in my project config.

## Functional requirements
1. MCP client connects to servers via **stdio transport** (spawning a child process) using rmcp's `TokioChildProcess`.
2. On connection, the client calls `list_tools()` to discover available tools.
3. Discovered MCP tools are wrapped in `ToolPort` adapters and registered in the tool registry.
4. Tool execution: the agent loop calls the MCP tool via `call_tool()`, which sends JSON-RPC to the server.
5. Multiple MCP servers can be configured and connected simultaneously.
6. Lazy connection: MCP servers connect on first use, not at startup.
7. Reconnection: if an MCP server crashes, attempt reconnection on the next tool call.
8. Server lifecycle management: sai starts and stops MCP server processes.

## Non-functional requirements
- MCP server startup must complete within 10 seconds or timeout.
- Failed MCP connections must not block the agent — other tools remain available.
- MCP tool schemas must be converted to match the same JSON Schema format as built-in tools.
- Support for SSE transport as a future extension (not required for v0.1).
```

---

## How to bootstrap the repo from scratch

Execute these steps in order to go from empty directory to fully configured project:

```bash
# 1. Create repo and initial structure
mkdir sai-code && cd sai-code
git init

# 2. Create all directories
mkdir -p crates/{sai-core,sai-llm,sai-mcp,sai-tools,sai-analysis,sai-tui,sai-config,sai-permissions,sai-context,sai-cli}/src
mkdir -p .github/workflows tests fixtures docs specs

# 3. Place all config files from this report:
#    Cargo.toml (workspace root), each crate's Cargo.toml,
#    .gitignore, rustfmt.toml, clippy.toml, deny.toml, justfile,
#    CLAUDE.md, README.md, CONTRIBUTING.md, LICENSE-MIT, LICENSE-APACHE,
#    .github/workflows/ci.yml, .github/workflows/release.yml,
#    .github/dependabot.yml

# 4. Create minimal lib.rs stubs in each crate
for crate in sai-core sai-llm sai-mcp sai-tools sai-analysis sai-tui sai-config sai-permissions sai-context; do
    echo '//! `'"$crate"'` crate.' > "crates/$crate/src/lib.rs"
done
echo 'fn main() { println!("sai v0.1.0"); }' > crates/sai-cli/src/main.rs

# 5. Verify it compiles
cargo check --workspace

# 6. Initialize SpecKit
uv tool install specify-cli --from git+https://github.com/github/spec-kit.git@v0.5.0
specify init --here --ai claude

# 7. Run the constitution command in Claude Code
# /speckit.constitution → paste the constitution text from Part 1

# 8. Place spec documents into specs/ directory

# 9. First commit
git add -A
git commit -m "feat: initial repository bootstrap with full architecture"
```

**This blueprint gives sai-code a production-grade foundation from day one** — hexagonal architecture that enforces clean dependency boundaries, a CI pipeline that catches issues before merge, supply chain security via cargo-deny, and SpecKit-driven specs that serve as living documentation for every major feature. The ten-crate workspace mirrors the architectural boundaries of proven coding agents while keeping each piece independently testable and replaceable.